use std::collections::BTreeMap;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_repository::AmendmentRepository;
use crate::application::ports::amendment_source::{AmendmentBatch, AmendmentSource, ArchiveScan};
use crate::domain::amendment::GroupResolutionReport;
use crate::domain::legislature::CURRENT_LEGISLATURE;

/// Nom de la source dans `source_archives`.
pub const ARCHIVE_LABEL: &str = "amendements";

/// Amendements par lot remonte de la source. Assez large pour amortir le
/// chargement du referentiel, assez petit pour borner la memoire tenue.
const BATCH_SIZE: usize = 1000;

#[derive(Debug, thiserror::Error)]
pub enum RefreshAmendmentsError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::RepositoryError),
}

/// Ce qu'une passe a produit. Les lacunes y figurent explicitement: un
/// signataire sans groupe date, un sort hors referentiel ou un reliquat non
/// ingere doivent se voir (README.md §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AmendmentsSummary {
    /// Vrai quand la source sert l'archive deja ingeree a la passe precedente:
    /// rien n'est reparse, rien n'est reecrit.
    pub skipped_unchanged: bool,
    pub written: usize,
    /// Amendements laisses pour la passe suivante parce que le plafond de passe
    /// est atteint. Non nul = chargement en cours, a afficher comme tel.
    pub pending: usize,
    pub json_entries: usize,
    pub parsed: usize,
    pub undecodable: usize,
    pub malformed: usize,
    pub refused: usize,
    pub unreadable: usize,
    pub failures: BTreeMap<String, usize>,
    pub top_level: BTreeMap<String, usize>,
    pub without_text_ref: usize,
    pub other_legislature: usize,
    /// Sorts publies hors referentiel, avec leur nombre (RM-04).
    pub unknown_fates: BTreeMap<String, usize>,
    pub groups: GroupResolutionReport,
    /// Renseigne quand le referentiel des acteurs ne permet pas de dater les
    /// groupes. Les amendements sont ecrits quand meme, sans groupe.
    pub registry_anomaly: Option<String>,
}

/// CU-01 — Ingerer les amendements.
pub struct RefreshAmendments<'a> {
    source: &'a dyn AmendmentSource,
    repository: &'a dyn AmendmentRepository,
    actor_repository: &'a dyn ActorRepository,
    /// Amendements ecrits par passe. Plafond, pas objectif: le reliquat est
    /// repris a la passe suivante. `0` leve la borne, pour un premier
    /// chargement lance a la main hors cadence.
    ceiling: usize,
}

impl<'a> RefreshAmendments<'a> {
    pub fn new(
        source: &'a dyn AmendmentSource,
        repository: &'a dyn AmendmentRepository,
        actor_repository: &'a dyn ActorRepository,
        ceiling: usize,
    ) -> Self {
        Self {
            source,
            repository,
            actor_repository,
            ceiling,
        }
    }

    pub async fn execute(&self) -> Result<AmendmentsSummary, RefreshAmendmentsError> {
        let mut summary = AmendmentsSummary::default();

        let feed = self
            .source
            .fetch_amendments(CURRENT_LEGISLATURE, BATCH_SIZE)
            .await?;

        // Une archive republiee a l'identique ne vaut pas d'etre reparcourue.
        // ArchiveFetcher evite deja le retelechargement; ce test-ci evite le
        // reparsing, qui est le vrai cout sur ce jeu de donnees.
        if let Some(archive_id) = &feed.archive_id {
            let known = self.repository.last_archive_id(ARCHIVE_LABEL).await?;
            if known.as_deref() == Some(archive_id.as_str()) {
                tracing::info!("Amendments archive unchanged ({archive_id}), skipping the walk");
                summary.skipped_unchanged = true;
                return Ok(summary);
            }
        }

        let mut batches = feed.batches;
        let mut scan: Option<ArchiveScan> = None;
        let mut stopped_at_ceiling = false;

        while let Some(message) = batches.next().await {
            match message? {
                AmendmentBatch::Items(mut items) => {
                    if stopped_at_ceiling {
                        continue;
                    }
                    if self.ceiling > 0 && summary.written >= self.ceiling {
                        stopped_at_ceiling = true;
                        continue;
                    }

                    self.resolve_groups(&mut items, &mut summary).await?;
                    summary.written += self.repository.save_amendments(&items).await?;
                }
                AmendmentBatch::Done(done) => scan = Some(done),
            }
        }

        if let Some(scan) = scan {
            summary.json_entries = scan.json_entries;
            summary.parsed = scan.parsed;
            summary.undecodable = scan.undecodable;
            summary.malformed = scan.malformed;
            summary.refused = scan.refused;
            summary.unreadable = scan.unreadable();
            summary.failures = scan.failures;
            summary.top_level = scan.top_level;
            summary.without_text_ref = scan.without_text_ref;
            summary.other_legislature = scan.other_legislature;
            summary.unknown_fates = scan.unknown_fates;
            // Le bilan compte tout ce que le parcours a lu; l'ecart avec ce que
            // la passe a ecrit est le reliquat.
            summary.pending = scan.parsed.saturating_sub(summary.written);
        }

        // L'identite n'est retenue qu'apres une passe entiere: une passe tronquee
        // par le plafond ne doit pas faire sauter la suivante.
        let walked_to_nothing = summary.json_entries > 0 && summary.parsed == 0;
        if walked_to_nothing {
            tracing::error!(
                "Amendments archive exposed {} JSON entries but parsed none; archive identity is not retained",
                summary.json_entries
            );
        }

        if summary.pending == 0 && !walked_to_nothing {
            if let Some(archive_id) = feed.archive_id {
                self.repository
                    .remember_archive(ARCHIVE_LABEL, &archive_id)
                    .await?;
            }
        }

        tracing::info!(
            "Amendments refreshed: {} written, {} pending, {} unreadable, {} without a text ref",
            summary.written,
            summary.pending,
            summary.unreadable,
            summary.without_text_ref
        );
        if summary.groups.unresolved > 0 || summary.groups.ambiguous > 0 {
            tracing::warn!(
                "{} signatories carry no dated group, {} are ambiguous at their deposit date",
                summary.groups.unresolved,
                summary.groups.ambiguous
            );
        }
        if summary.groups.undated > 0 {
            tracing::warn!(
                "{} signatories belong to amendments with no deposit date: no dated group is computable",
                summary.groups.undated
            );
        }

        Ok(summary)
    }

    /// RM-02: le groupe d'un signataire est celui de la date de depot.
    ///
    /// Le referentiel n'est charge que pour les acteurs cites par le lot, comme
    /// le fait deja `RefreshScrutins` pour les positions nominales.
    async fn resolve_groups(
        &self,
        items: &mut [crate::domain::amendment::Amendment],
        summary: &mut AmendmentsSummary,
    ) -> Result<(), RefreshAmendmentsError> {
        let mut uids: Vec<_> = items.iter().flat_map(|a| a.signatory_uids()).collect();
        uids.sort();
        uids.dedup();
        if uids.is_empty() {
            return Ok(());
        }

        let directory = self.actor_repository.load_directory_for(&uids).await?;
        if directory.is_empty() {
            // Le referentiel est vide ou en retard. Les amendements entrent quand
            // meme, sans groupe: rien n'est devine, rien n'est perdu.
            summary.registry_anomaly.get_or_insert_with(|| {
                "actor registry empty: signatories stored without a dated group".to_string()
            });
            return Ok(());
        }

        for amendment in items.iter_mut() {
            summary
                .groups
                .merge(amendment.resolve_signatory_groups(&directory));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::NaiveDate;

    use super::*;
    use crate::application::ports::actor_repository::RegistrySummary;
    use crate::application::ports::amendment_repository::{
        AmendmentGroupOption, AmendmentPage, AmendmentPageRequest, DossierAmendmentCoverage,
        SignatoryRow,
    };
    use crate::application::ports::amendment_source::{AmendmentBatches, AmendmentFeed};
    use crate::application::ports::{RepositoryError, SourceError};
    use crate::domain::actor::{
        Actor, ActorDirectory, ActorRegistry, ActorRole, ActorUid, GroupMembership, GroupUid,
        MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };
    use crate::domain::amendment::{
        Amendment, AmendmentFate, AmendmentNumber, AmendmentTarget, AmendmentUid, Author,
        GroupOrigin, NewAmendment, Signatory, SignatoryRole,
    };

    fn amendment(uid: &str, actor: &str, deposited: Option<NaiveDate>) -> Amendment {
        Amendment::new(NewAmendment {
            uid: AmendmentUid::new(uid.to_string()).unwrap(),
            legislature: 17,
            number: AmendmentNumber::new("78".to_string()).unwrap(),
            text_ref: None,
            examination_ref: None,
            target: AmendmentTarget::new("ARTICLE 3".to_string(), None).unwrap(),
            author: Some(Author::Deputy(Signatory::new(
                ActorUid::new(actor.to_string()).unwrap(),
                SignatoryRole::Author,
                0,
                None,
            ))),
            cosignatories: Vec::new(),
            summary: Some("Exposé".to_string()),
            fate: AmendmentFate::from_source(Some("Adopté")),
            state_label: None,
            deposited_on: deposited,
            parent_uid: None,
        })
        .unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct FakeSource {
        feed: Mutex<Option<AmendmentFeed>>,
    }

    impl FakeSource {
        fn with(
            archive_id: Option<&str>,
            batches: Vec<Result<AmendmentBatch, SourceError>>,
        ) -> Self {
            Self {
                feed: Mutex::new(Some(AmendmentFeed {
                    archive_id: archive_id.map(str::to_string),
                    batches: AmendmentBatches::from_batches(batches),
                })),
            }
        }
    }

    #[async_trait]
    impl AmendmentSource for FakeSource {
        async fn fetch_amendments(
            &self,
            _legislature: u16,
            _batch_size: usize,
        ) -> Result<AmendmentFeed, SourceError> {
            self.feed
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| SourceError::Download("source unavailable".into()))
        }
    }

    #[derive(Default)]
    struct FakeRepository {
        saved: Mutex<Vec<Amendment>>,
        known_archive: Mutex<Option<String>>,
        remembered: Mutex<Option<String>>,
    }

    #[async_trait]
    impl AmendmentRepository for FakeRepository {
        async fn save_amendments(
            &self,
            amendments: &[Amendment],
        ) -> Result<usize, RepositoryError> {
            self.saved.lock().unwrap().extend_from_slice(amendments);
            Ok(amendments.len())
        }

        async fn by_dossier(
            &self,
            _dossier_uid: &str,
            _page: &AmendmentPageRequest,
        ) -> Result<AmendmentPage, RepositoryError> {
            unreachable!()
        }

        async fn groups_by_dossier(
            &self,
            _dossier_uid: &str,
        ) -> Result<Vec<AmendmentGroupOption>, RepositoryError> {
            unreachable!()
        }

        async fn dossier_coverage(
            &self,
            _dossier_uid: &str,
        ) -> Result<DossierAmendmentCoverage, RepositoryError> {
            unreachable!()
        }

        async fn signatories_of(
            &self,
            _amendment_uid: &str,
        ) -> Result<Vec<SignatoryRow>, RepositoryError> {
            unreachable!()
        }

        async fn last_archive_id(&self, _label: &str) -> Result<Option<String>, RepositoryError> {
            Ok(self.known_archive.lock().unwrap().clone())
        }

        async fn remember_archive(&self, _label: &str, id: &str) -> Result<(), RepositoryError> {
            *self.remembered.lock().unwrap() = Some(id.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeActorRepository {
        requested: Mutex<Vec<ActorUid>>,
        directory: Mutex<Option<ActorDirectory>>,
    }

    #[async_trait]
    impl ActorRepository for FakeActorRepository {
        async fn save_registry(
            &self,
            _registry: &ActorRegistry,
        ) -> Result<RegistrySummary, RepositoryError> {
            unreachable!()
        }

        async fn load_directory_for(
            &self,
            actor_uids: &[ActorUid],
        ) -> Result<ActorDirectory, RepositoryError> {
            self.requested.lock().unwrap().extend_from_slice(actor_uids);
            Ok(self
                .directory
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| ActorDirectory::new(Vec::new(), Vec::new(), Vec::new())))
        }
    }

    fn directory_with(actor: &str, group: &str, from: NaiveDate) -> ActorDirectory {
        let actor_uid = ActorUid::new(actor.to_string()).unwrap();
        let group_uid = GroupUid::new(group.to_string()).unwrap();
        ActorDirectory::new(
            vec![Actor::new(
                actor_uid.clone(),
                None,
                "Jean".to_string(),
                "Dupont".to_string(),
                ActorRole::Deputy,
            )
            .unwrap()],
            vec![ParliamentaryGroup::new(
                group_uid.clone(),
                17,
                "Groupe".to_string(),
                "G".to_string(),
                None,
                None,
                None,
            )
            .unwrap()],
            vec![GroupMembership::new(
                "PM1".to_string(),
                actor_uid,
                group_uid,
                17,
                MembershipPeriod::new(from, None).unwrap(),
                MembershipQuality::new("Membre".to_string()).unwrap(),
            )],
        )
    }

    #[tokio::test]
    async fn every_amendment_is_written_and_the_scan_is_reported() {
        let source = FakeSource::with(
            Some("etag-1"),
            vec![
                Ok(AmendmentBatch::Items(vec![
                    amendment("AM1", "PA1", Some(date(2025, 3, 12))),
                    amendment("AM2", "PA1", Some(date(2025, 3, 13))),
                ])),
                Ok(AmendmentBatch::Done(ArchiveScan {
                    parsed: 2,
                    undecodable: 1,
                    malformed: 1,
                    refused: 1,
                    without_text_ref: 2,
                    other_legislature: 1,
                    ..Default::default()
                })),
            ],
        );
        let repository = FakeRepository::default();
        let actors = FakeActorRepository {
            directory: Mutex::new(Some(directory_with("PA1", "PO100", date(2024, 7, 18)))),
            ..Default::default()
        };

        let summary = RefreshAmendments::new(&source, &repository, &actors, 0)
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 2);
        assert_eq!(summary.pending, 0);
        assert_eq!(summary.unreadable, 3);
        assert_eq!(summary.undecodable, 1);
        assert_eq!(summary.malformed, 1);
        assert_eq!(summary.refused, 1);
        assert_eq!(summary.without_text_ref, 2);
        assert_eq!(summary.other_legislature, 1);
        assert_eq!(repository.saved.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn the_dated_group_is_posted_on_the_signatories() {
        let source = FakeSource::with(
            None,
            vec![
                Ok(AmendmentBatch::Items(vec![amendment(
                    "AM1",
                    "PA1",
                    Some(date(2025, 3, 12)),
                )])),
                Ok(AmendmentBatch::Done(ArchiveScan {
                    parsed: 1,
                    ..Default::default()
                })),
            ],
        );
        let repository = FakeRepository::default();
        let actors = FakeActorRepository {
            directory: Mutex::new(Some(directory_with("PA1", "PO100", date(2024, 7, 18)))),
            ..Default::default()
        };

        let summary = RefreshAmendments::new(&source, &repository, &actors, 0)
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.groups.resolved, 1);
        let saved = repository.saved.lock().unwrap();
        let Author::Deputy(author) = saved[0].author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_origin, GroupOrigin::ResolvedAtDeposit);
        // Le referentiel n'est interroge que pour les acteurs cites.
        assert_eq!(actors.requested.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_empty_registry_does_not_cost_the_amendments() {
        let source = FakeSource::with(
            None,
            vec![
                Ok(AmendmentBatch::Items(vec![amendment(
                    "AM1",
                    "PA1",
                    Some(date(2025, 3, 12)),
                )])),
                Ok(AmendmentBatch::Done(ArchiveScan {
                    parsed: 1,
                    ..Default::default()
                })),
            ],
        );
        let repository = FakeRepository::default();
        let actors = FakeActorRepository::default();

        let summary = RefreshAmendments::new(&source, &repository, &actors, 0)
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert!(summary.registry_anomaly.is_some());
        let saved = repository.saved.lock().unwrap();
        let Author::Deputy(author) = saved[0].author() else {
            panic!("expected a deputy author");
        };
        assert_eq!(author.group_origin, GroupOrigin::Unknown);
    }

    #[tokio::test]
    async fn an_unchanged_archive_is_not_walked_again() {
        let source = FakeSource::with(
            Some("etag-1"),
            vec![Ok(AmendmentBatch::Items(vec![amendment(
                "AM1",
                "PA1",
                Some(date(2025, 3, 12)),
            )]))],
        );
        let repository = FakeRepository {
            known_archive: Mutex::new(Some("etag-1".to_string())),
            ..Default::default()
        };
        let actors = FakeActorRepository::default();

        let summary = RefreshAmendments::new(&source, &repository, &actors, 0)
            .execute()
            .await
            .unwrap();

        assert!(summary.skipped_unchanged);
        assert_eq!(summary.written, 0);
        assert!(repository.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_complete_pass_remembers_the_archive_a_truncated_one_does_not() {
        let batches = || {
            vec![
                Ok(AmendmentBatch::Items(vec![amendment(
                    "AM1",
                    "PA1",
                    Some(date(2025, 3, 12)),
                )])),
                Ok(AmendmentBatch::Items(vec![amendment(
                    "AM2",
                    "PA1",
                    Some(date(2025, 3, 13)),
                )])),
                Ok(AmendmentBatch::Done(ArchiveScan {
                    parsed: 2,
                    ..Default::default()
                })),
            ]
        };

        let complete = FakeRepository::default();
        let summary = RefreshAmendments::new(
            &FakeSource::with(Some("etag-1"), batches()),
            &complete,
            &FakeActorRepository::default(),
            0,
        )
        .execute()
        .await
        .unwrap();
        assert_eq!(summary.pending, 0);
        assert_eq!(
            complete.remembered.lock().unwrap().as_deref(),
            Some("etag-1")
        );

        let truncated = FakeRepository::default();
        let summary = RefreshAmendments::new(
            &FakeSource::with(Some("etag-1"), batches()),
            &truncated,
            &FakeActorRepository::default(),
            1,
        )
        .execute()
        .await
        .unwrap();
        assert_eq!(summary.written, 1);
        assert_eq!(summary.pending, 1);
        assert!(truncated.remembered.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn an_archive_that_parses_nothing_is_not_remembered() {
        let source = FakeSource::with(
            Some("etag-1"),
            vec![Ok(AmendmentBatch::Done(ArchiveScan {
                json_entries: 2,
                malformed: 2,
                ..Default::default()
            }))],
        );
        let repository = FakeRepository::default();

        let summary = RefreshAmendments::new(
            &source,
            &repository,
            &FakeActorRepository::default(),
            0,
        )
        .execute()
        .await
        .unwrap();

        assert_eq!(summary.parsed, 0);
        assert!(repository.remembered.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn an_unavailable_source_leaves_the_stored_amendments_alone() {
        let source = FakeSource::with(None, Vec::new());
        // Le flux est consomme une premiere fois, la seconde echoue.
        let repository = FakeRepository::default();
        let actors = FakeActorRepository::default();
        let use_case = RefreshAmendments::new(&source, &repository, &actors, 0);

        assert!(use_case.execute().await.is_ok());
        assert!(matches!(
            use_case.execute().await,
            Err(RefreshAmendmentsError::Source(_))
        ));
        assert!(repository.saved.lock().unwrap().is_empty());
    }
}
