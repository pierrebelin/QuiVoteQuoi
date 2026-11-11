use std::collections::{BTreeSet, HashMap};

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::assembly_source::AssemblySource;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::application::use_cases::journaling::Journaling;
use crate::domain::actor::ActorUid;
use crate::domain::dossier::{InitiatorAttachment, WriteDecision};
use crate::domain::ingestion::IngestionSource;

/// Ce qu'un rafraichissement a ecrit, et ce qu'il a volontairement laisse de
/// cote. Les deux sont comptes: un dossier saute doit rester explicable
/// (README.md §2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DossiersSummary {
    /// Dossiers lus dans la source.
    pub seen: usize,
    /// Dossiers reellement reecrits en base.
    pub written: usize,
    /// Sort definitif deja stocke: plus rien ne peut changer.
    pub skipped_final: usize,
    /// Dernier acte et nombre d'actes inchanges depuis la derniere ingestion.
    pub skipped_unchanged: usize,
}

/// Portee du rafraichissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshScope {
    /// N'ecrit que les dossiers dont la source a bouge.
    Incremental,
    /// Reecrit tout. Necessaire apres un changement de regle de derivation
    /// (score, sort, rattachement): l'incremental ne verrait aucun changement
    /// cote source et laisserait la base sur l'ancienne regle.
    Full,
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::assembly_source::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::dossier_repository::RepositoryError),
}

pub struct RefreshDossiers<'a> {
    source: &'a dyn AssemblySource,
    repository: &'a dyn DossierRepository,
    actor_repository: &'a dyn ActorRepository,
    journaling: Journaling<'a>,
}

impl<'a> RefreshDossiers<'a> {
    pub fn new(
        source: &'a dyn AssemblySource,
        repository: &'a dyn DossierRepository,
        actor_repository: &'a dyn ActorRepository,
        journaling: Journaling<'a>,
    ) -> Self {
        Self {
            source,
            repository,
            actor_repository,
            journaling,
        }
    }

    pub async fn execute(&self) -> Result<DossiersSummary, RefreshError> {
        self.execute_with(RefreshScope::Incremental).await
    }

    pub async fn execute_with(&self, scope: RefreshScope) -> Result<DossiersSummary, RefreshError> {
        let result = self.refresh(scope).await;
        self.journaling
            .record(IngestionSource::Dossiers, &result, |summary| {
                summary.seen as u64
            })
            .await;
        result
    }

    async fn refresh(&self, scope: RefreshScope) -> Result<DossiersSummary, RefreshError> {
        let dossiers_with_refs = self.source.fetch_dossiers_since_with_refs(None).await?;

        let mut summary = DossiersSummary {
            seen: dossiers_with_refs.len(),
            ..Default::default()
        };

        let stored = match scope {
            RefreshScope::Incremental => self.repository.load_states().await?,
            RefreshScope::Full => HashMap::new(),
        };

        // Le tri precede le rattachement: un dossier qu'on n'ecrira pas n'a pas
        // besoin qu'on resolve ses initiateurs, et son absence allege d'autant
        // le referentiel a charger.
        let dossiers_with_refs: Vec<_> = dossiers_with_refs
            .into_iter()
            .filter(|(dossier, _)| {
                match dossier.write_decision(stored.get(dossier.uid().as_str())) {
                    WriteDecision::Write => true,
                    WriteDecision::SkipFinal => {
                        summary.skipped_final += 1;
                        false
                    }
                    WriteDecision::SkipUnchanged => {
                        summary.skipped_unchanged += 1;
                        false
                    }
                }
            })
            .collect();

        if dossiers_with_refs.is_empty() {
            tracing::info!(
                "Dossiers refresh: {} seen, nothing changed ({} final, {} unchanged)",
                summary.seen,
                summary.skipped_final,
                summary.skipped_unchanged
            );
            return Ok(summary);
        }

        let actor_uids: Vec<ActorUid> = dossiers_with_refs
            .iter()
            .flat_map(|(_, refs)| refs.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter_map(|r| ActorUid::new(r).ok())
            .collect();

        let directory = self
            .actor_repository
            .load_directory_for(&actor_uids)
            .await?;
        if directory.is_empty() && !actor_uids.is_empty() {
            tracing::warn!(
                "Actor registry is empty: {} initiator references cannot be resolved",
                actor_uids.len()
            );
        }

        let mut totals = InitiatorAttachment::default();
        let mut dossiers_without_deposit_date = 0;

        let dossiers: Vec<_> = dossiers_with_refs
            .into_iter()
            .map(|(mut dossier, refs)| {
                if !refs.is_empty() && dossier.initiators().is_empty() {
                    let uids: Vec<ActorUid> = refs
                        .into_iter()
                        .filter_map(|r| ActorUid::new(r).ok())
                        .collect();
                    let report = dossier.attach_initiators(&uids, &directory);

                    totals.resolved += report.resolved;
                    totals.unknown_actors += report.unknown_actors;
                    totals.without_group += report.without_group;
                    totals.ambiguous += report.ambiguous;
                    if report.missing_deposit_date {
                        dossiers_without_deposit_date += 1;
                    }
                }
                dossier
            })
            .collect();

        tracing::info!(
            "Initiators attached: {} resolved, {} unknown actors, {} without a group at the deposit date, {} ambiguous; {} dossiers have no deposit date",
            totals.resolved,
            totals.unknown_actors,
            totals.without_group,
            totals.ambiguous,
            dossiers_without_deposit_date
        );

        summary.written = self.repository.save_all(&dossiers).await?;

        tracing::info!(
            "Dossiers refresh: {} seen, {} written, {} skipped (final outcome), {} skipped (unchanged)",
            summary.seen,
            summary.written,
            summary.skipped_final,
            summary.skipped_unchanged
        );

        Ok(summary)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, NaiveDate, TimeZone, Utc};
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::use_cases::journal_fakes::InMemoryIngestionJournal;
    use crate::domain::ingestion::{CheckOutcome, IngestionSource};

    use crate::application::ports::actor_repository::{RegistrySummary, RepositoryError};
    use crate::application::ports::assembly_source::SourceError;
    use crate::domain::actor::{
        Actor, ActorDirectory, ActorRegistry, ActorRole, GroupMembership, GroupUid,
        MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };
    use crate::domain::dossier::{
        DossierOutcome, DossierUid, LegislativeAct, LegislativeDossier, NewLegislativeDossier,
        Score, StoredDossierState,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct FakeSource {
        dossiers_with_refs: Vec<(LegislativeDossier, Vec<String>)>,
    }

    #[async_trait]
    impl AssemblySource for FakeSource {
        async fn fetch_dossiers_since(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossiers_since_with_refs(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
            Ok(self.dossiers_with_refs.clone())
        }

        async fn fetch_dossier_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            unreachable!()
        }
    }

    /// Referentiel in-memory: un depute passe du groupe A au groupe B le 1er juillet 2025.
    pub(crate) struct InMemoryActorRepository {
        registry: Mutex<ActorRegistry>,
        pub(crate) requested_uids: Mutex<Vec<ActorUid>>,
    }

    impl InMemoryActorRepository {
        pub(crate) fn with_deputy_changing_group() -> Self {
            let actor = Actor::new(
                ActorUid::new("PA111111".into()).unwrap(),
                Some("M.".into()),
                "Jean".into(),
                "Dupont".into(),
                ActorRole::Deputy,
            )
            .unwrap();

            let group = |uid: &str, abbrev: &str, label: &str| {
                ParliamentaryGroup::new(
                    GroupUid::new(uid.into()).unwrap(),
                    17,
                    label.into(),
                    abbrev.into(),
                    None,
                    None,
                    None,
                )
                .unwrap()
            };

            let membership = |source: &str, group: &str, start, end| {
                GroupMembership::new(
                    source.into(),
                    ActorUid::new("PA111111".into()).unwrap(),
                    GroupUid::new(group.into()).unwrap(),
                    17,
                    MembershipPeriod::new(start, end).unwrap(),
                    MembershipQuality::new("Membre".into()).unwrap(),
                )
            };

            Self {
                registry: Mutex::new(ActorRegistry {
                    actors: vec![actor],
                    groups: vec![
                        group("PO_A", "A", "Groupe A"),
                        group("PO_B", "B", "Groupe B"),
                    ],
                    memberships: vec![
                        membership("PM1", "PO_A", date(2024, 7, 19), Some(date(2025, 6, 30))),
                        membership("PM2", "PO_B", date(2025, 7, 1), None),
                    ],
                }),
                requested_uids: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl ActorRepository for InMemoryActorRepository {
        async fn save_registry(
            &self,
            registry: &ActorRegistry,
        ) -> Result<RegistrySummary, RepositoryError> {
            let summary = RegistrySummary {
                actors: registry.actors.len(),
                groups: registry.groups.len(),
                memberships: registry.memberships.len(),
            };
            *self.registry.lock().unwrap() = registry.clone();
            Ok(summary)
        }

        async fn load_directory_for(
            &self,
            actor_uids: &[ActorUid],
        ) -> Result<ActorDirectory, RepositoryError> {
            self.requested_uids
                .lock()
                .unwrap()
                .extend_from_slice(actor_uids);

            let registry = self.registry.lock().unwrap();
            let actors: Vec<Actor> = registry
                .actors
                .iter()
                .filter(|a| actor_uids.contains(a.uid()))
                .cloned()
                .collect();
            let memberships: Vec<GroupMembership> = registry
                .memberships
                .iter()
                .filter(|m| actor_uids.contains(m.actor_uid()))
                .cloned()
                .collect();

            Ok(ActorDirectory::new(
                actors,
                registry.groups.clone(),
                memberships,
            ))
        }
    }

    pub(crate) struct InMemoryDossierRepository {
        pub(crate) dossiers: Mutex<HashMap<String, LegislativeDossier>>,
        /// Uid de chaque dossier reellement passe a `save_all`, dans l'ordre.
        /// C'est ce que le rafraichissement incremental doit reduire.
        pub(crate) written: Mutex<Vec<String>>,
    }

    impl InMemoryDossierRepository {
        pub(crate) fn new() -> Self {
            Self {
                dossiers: Mutex::new(HashMap::new()),
                written: Mutex::new(vec![]),
            }
        }

        fn count(&self) -> usize {
            self.dossiers.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl DossierRepository for InMemoryDossierRepository {
        async fn save_all(
            &self,
            dossiers: &[LegislativeDossier],
        ) -> Result<usize, crate::application::ports::dossier_repository::RepositoryError> {
            let mut store = self.dossiers.lock().unwrap();
            for d in dossiers {
                self.written
                    .lock()
                    .unwrap()
                    .push(d.uid().as_str().to_string());
                store.insert(d.uid().as_str().to_string(), d.clone());
            }
            Ok(dossiers.len())
        }

        async fn load_states(
            &self,
        ) -> Result<HashMap<String, StoredDossierState>, RepositoryError> {
            Ok(self
                .dossiers
                .lock()
                .unwrap()
                .iter()
                .map(|(uid, d)| {
                    (
                        uid.clone(),
                        StoredDossierState {
                            last_activity_date: d.last_activity_date(),
                            act_count: d.acts().len(),
                            outcome_is_final: d.outcome().is_final(),
                        },
                    )
                })
                .collect())
        }

        async fn find_recent(
            &self,
            _since: NaiveDate,
        ) -> Result<
            Vec<LegislativeDossier>,
            crate::application::ports::dossier_repository::RepositoryError,
        > {
            unreachable!()
        }

        async fn find_page(
            &self,
            _criteria: &crate::application::ports::dossier_repository::DossierCriteria,
            _order: crate::application::ports::dossier_repository::DossierOrder,
            _limit: i64,
            _offset: i64,
        ) -> Result<crate::application::ports::dossier_repository::DossierPage, RepositoryError>
        {
            unreachable!()
        }

        async fn find_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<
            Option<LegislativeDossier>,
            crate::application::ports::dossier_repository::RepositoryError,
        > {
            unreachable!()
        }

        async fn find_by_uids(
            &self,
            _uids: &[crate::domain::dossier::DossierUid],
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }
    }

    /// Elements d'un dossier de la source, avant construction: un test qui veut
    /// une variante (sort, acte supplementaire) les ajuste ici.
    pub(crate) fn dossier_parts(
        uid: &str,
        deposit_date: Option<NaiveDate>,
        last_activity: NaiveDate,
    ) -> NewLegislativeDossier {
        NewLegislativeDossier {
            uid: DossierUid::new(uid.into()).unwrap(),
            title: format!("Loi {uid}"),
            procedure: "Proposition de loi".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date,
            last_activity_date: last_activity,
            last_activity_label: "D\u{00e9}p\u{00f4}t".into(),
            acts: vec![LegislativeAct {
                date: last_activity,
                label: "D\u{00e9}p\u{00f4}t".into(),
                code: Some("AN1-DEPOT".into()),
                decision_vote: None,
            }],
            documents: vec![],
            score: Score::new(2, 4, 2, 23).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            outcome: DossierOutcome::NoRecordedConclusion,
        }
    }

    pub(crate) fn dossier(
        uid: &str,
        deposit_date: Option<NaiveDate>,
        last_activity: NaiveDate,
    ) -> LegislativeDossier {
        LegislativeDossier::new(dossier_parts(uid, deposit_date, last_activity))
    }

    #[tokio::test]
    async fn attaches_each_initiator_to_the_group_held_on_its_own_deposit_date() {
        let source = FakeSource {
            dossiers_with_refs: vec![
                (
                    dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                    vec!["PA111111".into()],
                ),
                (
                    dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                    vec!["PA111111".into()],
                ),
            ],
        };

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let count = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(count.written, 2);
        assert_eq!(repo.count(), 2);

        let store = repo.dossiers.lock().unwrap();
        let d1 = &store["D1"].initiators()[0];
        assert_eq!(d1.full_name(), "Jean Dupont");
        assert_eq!(d1.group().unwrap().abbrev, "A");
        assert_eq!(d1.reference_date(), Some(date(2025, 5, 13)));

        let d2 = &store["D2"].initiators()[0];
        assert_eq!(d2.group().unwrap().abbrev, "B");
        assert_eq!(d2.reference_date(), Some(date(2025, 9, 1)));
    }

    #[tokio::test]
    async fn asks_the_registry_for_each_referenced_actor_only_once() {
        let source = FakeSource {
            dossiers_with_refs: vec![
                (
                    dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                    vec!["PA111111".into()],
                ),
                (
                    dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                    vec!["PA111111".into()],
                ),
            ],
        };

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(actors.requested_uids.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn dossier_without_initiator_reference_keeps_an_empty_list() {
        let source = FakeSource {
            dossiers_with_refs: vec![(
                dossier("D3", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec![],
            )],
        };

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        let store = repo.dossiers.lock().unwrap();
        assert!(store["D3"].initiators().is_empty());
    }

    #[tokio::test]
    async fn unknown_actor_is_kept_without_a_group() {
        let source = FakeSource {
            dossiers_with_refs: vec![(
                dossier("D4", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec!["PA_UNKNOWN".into()],
            )],
        };

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        let store = repo.dossiers.lock().unwrap();
        let initiator = &store["D4"].initiators()[0];
        assert_eq!(initiator.full_name(), "PA_UNKNOWN");
        assert!(initiator.group().is_none());
    }

    #[tokio::test]
    async fn dossier_without_deposit_date_gets_a_name_but_no_group() {
        let source = FakeSource {
            dossiers_with_refs: vec![(
                dossier("D5", None, date(2025, 5, 13)),
                vec!["PA111111".into()],
            )],
        };

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        let store = repo.dossiers.lock().unwrap();
        let initiator = &store["D5"].initiators()[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert!(initiator.group().is_none());
        assert!(initiator.reference_date().is_none());
    }

    // Local: seul ce test a besoin d'une source de dossiers indisponible.
    struct DownSource;

    #[async_trait]
    impl AssemblySource for DownSource {
        async fn fetch_dossiers_since(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossiers_since_with_refs(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
            Err(SourceError::Download("source unavailable".into()))
        }

        async fn fetch_dossier_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            unreachable!()
        }
    }

    fn attempted_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 8, 30, 0).unwrap()
    }

    #[tokio::test]
    async fn should_record_succeeded_check_with_dossiers_seen_when_dossier_refresh_succeeds() {
        let source = FakeSource {
            dossiers_with_refs: vec![
                (
                    dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                    vec![],
                ),
                (
                    dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                    vec![],
                ),
            ],
        };
        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let journal = InMemoryIngestionJournal::default();

        RefreshDossiers::new(
            &source,
            &repo,
            &actors,
            Journaling::to(&journal, attempted_at()),
        )
        .execute()
        .await
        .unwrap();

        let recorded = journal.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].source(), IngestionSource::Dossiers);
        assert_eq!(recorded[0].attempted_at(), attempted_at());
        assert_eq!(
            recorded[0].outcome(),
            &CheckOutcome::Succeeded { items_read: 2 }
        );
    }

    #[tokio::test]
    async fn should_record_failed_check_when_assembly_source_fails() {
        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let journal = InMemoryIngestionJournal::default();

        let result = RefreshDossiers::new(
            &DownSource,
            &repo,
            &actors,
            Journaling::to(&journal, attempted_at()),
        )
        .execute()
        .await;

        assert!(matches!(result, Err(RefreshError::Source(_))));
        let recorded = journal.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].source(), IngestionSource::Dossiers);
        assert_eq!(recorded[0].attempted_at(), attempted_at());
        assert_eq!(
            recorded[0].outcome(),
            &CheckOutcome::Failed {
                reason: "download failed: source unavailable".to_string()
            }
        );
    }
}

#[cfg(test)]
mod incremental_tests {
    use super::tests::{
        dossier, dossier_parts, InMemoryActorRepository, InMemoryDossierRepository,
    };
    use super::*;
    use crate::application::ports::assembly_source::SourceError;
    use crate::domain::dossier::{
        DossierOutcome, DossierUid, LegislativeAct, LegislativeDossier, NewLegislativeDossier,
    };
    use async_trait::async_trait;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    struct FakeSource {
        dossiers: Vec<LegislativeDossier>,
    }

    #[async_trait]
    impl AssemblySource for FakeSource {
        async fn fetch_dossiers_since(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossiers_since_with_refs(
            &self,
            _since: Option<NaiveDate>,
        ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
            Ok(self.dossiers.iter().cloned().map(|d| (d, vec![])).collect())
        }

        async fn fetch_dossier_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            unreachable!()
        }
    }

    /// Pose l'etat initial de la base sans passer par le use case, puis remet
    /// le journal des ecritures a zero: seul ce que fera le rafraichissement
    /// suivant nous interesse.
    fn seeded_repository(stored: Vec<LegislativeDossier>) -> InMemoryDossierRepository {
        let repo = InMemoryDossierRepository::new();
        {
            let mut store = repo.dossiers.lock().unwrap();
            for d in stored {
                store.insert(d.uid().as_str().to_string(), d);
            }
        }
        repo.written.lock().unwrap().clear();
        repo
    }

    fn with_extra_act(
        mut dossier: NewLegislativeDossier,
        act_date: NaiveDate,
    ) -> NewLegislativeDossier {
        dossier.acts.push(LegislativeAct {
            date: act_date,
            label: "R\u{00e9}union de commission".into(),
            code: Some("AN1-COM-FOND-REUNION".into()),
            decision_vote: None,
        });
        dossier.last_activity_date = act_date;
        dossier.last_activity_label = "R\u{00e9}union de commission".into();
        dossier
    }

    #[tokio::test]
    async fn rewrites_only_the_dossiers_whose_source_moved() {
        let unchanged = dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13));
        let moved = dossier_parts("D2", Some(date(2025, 5, 13)), date(2025, 5, 13));
        let repo = seeded_repository(vec![
            unchanged.clone(),
            LegislativeDossier::new(moved.clone()),
        ]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeSource {
            dossiers: vec![
                unchanged,
                LegislativeDossier::new(with_extra_act(moved, date(2026, 1, 20))),
            ],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.seen, 2);
        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_unchanged, 1);
        assert_eq!(*repo.written.lock().unwrap(), vec!["D2".to_string()]);
    }

    /// Un acte insere a une date anterieure ne bouge pas la derniere activite:
    /// c'est le nombre d'actes qui doit le rattraper.
    #[tokio::test]
    async fn an_act_inserted_before_the_last_one_still_triggers_a_write() {
        let stored = dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 1, 20));
        let repo = seeded_repository(vec![LegislativeDossier::new(stored.clone())]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let mut backdated = stored;
        backdated.acts.push(LegislativeAct {
            date: date(2025, 9, 3),
            label: "Nomination de rapporteur".into(),
            code: Some("AN1-COM-FOND-NOMIN".into()),
            decision_vote: None,
        });

        let source = FakeSource {
            dossiers: vec![LegislativeDossier::new(backdated)],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_unchanged, 0);
    }

    /// Une loi promulguee ne peut plus bouger: meme si la source republie le
    /// dossier autrement, on ne le reecrit pas.
    #[tokio::test]
    async fn a_final_outcome_is_never_rewritten() {
        let mut promulgated = dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 4, 21));
        promulgated.outcome = DossierOutcome::Promulgated {
            date: date(2026, 4, 21),
            publication: crate::domain::dossier::LawPublication {
                law_code: Some("2026-300".into()),
                jo_date: None,
                legifrance_url: None,
            },
        };
        let repo = seeded_repository(vec![LegislativeDossier::new(promulgated.clone())]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeSource {
            dossiers: vec![LegislativeDossier::new(with_extra_act(
                promulgated,
                date(2026, 6, 1),
            ))],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 0);
        assert_eq!(summary.skipped_final, 1);
        assert!(repo.written.lock().unwrap().is_empty());
    }

    /// Un rejet n'est pas definitif: la navette peut reprendre, le dossier doit
    /// continuer a etre relu.
    #[tokio::test]
    async fn a_rejected_dossier_keeps_being_refreshed() {
        let mut rejected = dossier_parts("D1", Some(date(2025, 5, 13)), date(2025, 11, 4));
        rejected.outcome = DossierOutcome::Rejected {
            date: date(2025, 11, 4),
            label: "rejet\u{00e9}e".into(),
        };
        let repo = seeded_repository(vec![LegislativeDossier::new(rejected.clone())]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeSource {
            dossiers: vec![LegislativeDossier::new(with_extra_act(
                rejected,
                date(2026, 3, 17),
            ))],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_final, 0);
    }

    /// Une proposition dormante depuis deux ans est sautee tant qu'elle dort,
    /// et reecrite des qu'elle bouge: son age n'entre pas dans la decision.
    #[tokio::test]
    async fn a_long_dormant_dossier_is_written_again_as_soon_as_it_moves() {
        let dormant = dossier_parts("D1", Some(date(2024, 7, 20)), date(2024, 7, 20));
        let repo = seeded_repository(vec![LegislativeDossier::new(dormant.clone())]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let still_asleep = FakeSource {
            dossiers: vec![LegislativeDossier::new(dormant.clone())],
        };
        let summary = RefreshDossiers::new(&still_asleep, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();
        assert_eq!(summary.written, 0);
        assert_eq!(summary.skipped_unchanged, 1);

        let scheduled = FakeSource {
            dossiers: vec![LegislativeDossier::new(with_extra_act(
                dormant,
                date(2026, 8, 5),
            ))],
        };
        let summary = RefreshDossiers::new(&scheduled, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();
        assert_eq!(summary.written, 1);
    }

    #[tokio::test]
    async fn a_dossier_absent_from_the_database_is_always_written() {
        let repo = seeded_repository(vec![]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeSource {
            dossiers: vec![dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13))],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_unchanged, 0);
        assert_eq!(summary.skipped_final, 0);
    }

    /// Apres un changement de regle de derivation, la source n'a pas bouge mais
    /// la base est perimee: `Full` doit tout reecrire, sort definitif compris.
    #[tokio::test]
    async fn a_full_scope_rewrites_everything_including_final_outcomes() {
        let mut promulgated = dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 4, 21));
        promulgated.outcome = DossierOutcome::Promulgated {
            date: date(2026, 4, 21),
            publication: crate::domain::dossier::LawPublication {
                law_code: Some("2026-300".into()),
                jo_date: None,
                legifrance_url: None,
            },
        };
        let promulgated = LegislativeDossier::new(promulgated);
        let unchanged = dossier("D2", Some(date(2025, 5, 13)), date(2025, 5, 13));
        let repo = seeded_repository(vec![promulgated.clone(), unchanged.clone()]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeSource {
            dossiers: vec![promulgated, unchanged],
        };

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute_with(RefreshScope::Full)
            .await
            .unwrap();

        assert_eq!(summary.written, 2);
        assert_eq!(summary.skipped_final, 0);
        assert_eq!(summary.skipped_unchanged, 0);
    }
}
