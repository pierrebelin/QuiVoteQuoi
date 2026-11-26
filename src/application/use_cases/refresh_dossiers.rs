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
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDate, TimeZone, Utc};

    use crate::application::use_cases::fakes::actor_repository::InMemoryActorRepository;
    use crate::application::use_cases::fakes::assembly_source::FakeAssemblySource;
    use crate::application::use_cases::fakes::dossier_repository::InMemoryDossierRepository;
    use crate::application::use_cases::fakes::fixtures::dossier;
    use crate::application::use_cases::journal_fakes::InMemoryIngestionJournal;
    use crate::domain::ingestion::{CheckOutcome, IngestionSource};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[tokio::test]
    async fn attaches_each_initiator_to_the_group_held_on_its_own_deposit_date() {
        let source = FakeAssemblySource::with_refs(vec![
            (
                dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec!["PA111111".into()],
            ),
            (
                dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                vec!["PA111111".into()],
            ),
        ]);

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let count = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(count.written, 2);
        assert_eq!(repo.count(), 2);

        let d1 = repo.get("D1").unwrap();
        let d1 = &d1.initiators()[0];
        assert_eq!(d1.full_name(), "Jean Dupont");
        assert_eq!(d1.group().unwrap().abbrev, "A");
        assert_eq!(d1.reference_date(), Some(date(2025, 5, 13)));

        let d2 = repo.get("D2").unwrap();
        let d2 = &d2.initiators()[0];
        assert_eq!(d2.group().unwrap().abbrev, "B");
        assert_eq!(d2.reference_date(), Some(date(2025, 9, 1)));
    }

    #[tokio::test]
    async fn asks_the_registry_for_each_referenced_actor_only_once() {
        let source = FakeAssemblySource::with_refs(vec![
            (
                dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec!["PA111111".into()],
            ),
            (
                dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                vec!["PA111111".into()],
            ),
        ]);

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
        let source = FakeAssemblySource::with_refs(vec![(
            dossier("D3", Some(date(2025, 5, 13)), date(2025, 5, 13)),
            vec![],
        )]);

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert!(repo.get("D3").unwrap().initiators().is_empty());
    }

    #[tokio::test]
    async fn unknown_actor_is_kept_without_a_group() {
        let source = FakeAssemblySource::with_refs(vec![(
            dossier("D4", Some(date(2025, 5, 13)), date(2025, 5, 13)),
            vec!["PA_UNKNOWN".into()],
        )]);

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        let dossier = repo.get("D4").unwrap();
        let initiator = &dossier.initiators()[0];
        assert_eq!(initiator.full_name(), "PA_UNKNOWN");
        assert!(initiator.group().is_none());
    }

    #[tokio::test]
    async fn dossier_without_deposit_date_gets_a_name_but_no_group() {
        let source = FakeAssemblySource::with_refs(vec![(
            dossier("D5", None, date(2025, 5, 13)),
            vec!["PA111111".into()],
        )]);

        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        let dossier = repo.get("D5").unwrap();
        let initiator = &dossier.initiators()[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert!(initiator.group().is_none());
        assert!(initiator.reference_date().is_none());
    }

    fn attempted_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 8, 30, 0).unwrap()
    }

    #[tokio::test]
    async fn should_record_succeeded_check_with_dossiers_seen_when_dossier_refresh_succeeds() {
        let source = FakeAssemblySource::with_refs(vec![
            (
                dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13)),
                vec![],
            ),
            (
                dossier("D2", Some(date(2025, 9, 1)), date(2025, 9, 1)),
                vec![],
            ),
        ]);
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
            &FakeAssemblySource::down("source unavailable"),
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
    use super::*;
    use crate::application::use_cases::fakes::actor_repository::InMemoryActorRepository;
    use crate::application::use_cases::fakes::assembly_source::FakeAssemblySource;
    use crate::application::use_cases::fakes::dossier_repository::InMemoryDossierRepository;
    use crate::application::use_cases::fakes::fixtures::{dossier, dossier_parts};
    use crate::domain::dossier::{
        DossierOutcome, LegislativeAct, LegislativeDossier, NewLegislativeDossier,
    };
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
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
            conclusion_label: None,
            conclusion_code: None,
        });
        dossier.last_activity_date = act_date;
        dossier.last_activity_label = "R\u{00e9}union de commission".into();
        dossier
    }

    #[tokio::test]
    async fn rewrites_only_the_dossiers_whose_source_moved() {
        let unchanged = dossier("D1", Some(date(2025, 5, 13)), date(2025, 5, 13));
        let moved = dossier_parts("D2", Some(date(2025, 5, 13)), date(2025, 5, 13));
        let repo = InMemoryDossierRepository::with_dossiers(vec![
            unchanged.clone(),
            LegislativeDossier::new(moved.clone()),
        ]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeAssemblySource::without_refs(vec![
            unchanged,
            LegislativeDossier::new(with_extra_act(moved, date(2026, 1, 20))),
        ]);

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
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(
            stored.clone(),
        )]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let mut backdated = stored;
        backdated.acts.push(LegislativeAct {
            date: date(2025, 9, 3),
            label: "Nomination de rapporteur".into(),
            code: Some("AN1-COM-FOND-NOMIN".into()),
            decision_vote: None,
            conclusion_label: None,
            conclusion_code: None,
        });

        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(backdated)]);

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
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(
            promulgated.clone(),
        )]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(with_extra_act(
            promulgated,
            date(2026, 6, 1),
        ))]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 0);
        assert_eq!(summary.skipped_final, 1);
        assert!(repo.written.lock().unwrap().is_empty());
    }

    /// Rattrapage SPEC-navette lot 1 : une loi promulguee stockee avant le
    /// calcul de l'etat est reecrite une fois, puis sautee comme avant.
    #[tokio::test]
    async fn a_final_outcome_stored_before_the_law_progress_is_rewritten_once() {
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
        let repo = InMemoryDossierRepository::with_dossiers(vec![promulgated.clone()]);
        repo.without_law_progress
            .lock()
            .unwrap()
            .insert("D1".to_string());
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let source = FakeAssemblySource::without_refs(vec![promulgated]);
        let refresh = RefreshDossiers::new(&source, &repo, &actors, Journaling::off());

        let first = refresh.execute().await.unwrap();
        let second = refresh.execute().await.unwrap();

        assert_eq!(first.written, 1);
        assert_eq!(second.written, 0);
        assert_eq!(second.skipped_final, 1);
        assert_eq!(*repo.written.lock().unwrap(), vec!["D1".to_string()]);
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
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(
            rejected.clone(),
        )]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(with_extra_act(
            rejected,
            date(2026, 3, 17),
        ))]);

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
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(
            dormant.clone(),
        )]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let still_asleep =
            FakeAssemblySource::without_refs(vec![LegislativeDossier::new(dormant.clone())]);
        let summary = RefreshDossiers::new(&still_asleep, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();
        assert_eq!(summary.written, 0);
        assert_eq!(summary.skipped_unchanged, 1);

        let scheduled = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(
            with_extra_act(dormant, date(2026, 8, 5)),
        )]);
        let summary = RefreshDossiers::new(&scheduled, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();
        assert_eq!(summary.written, 1);
    }

    #[tokio::test]
    async fn a_dossier_absent_from_the_database_is_always_written() {
        let repo = InMemoryDossierRepository::with_dossiers(vec![]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeAssemblySource::without_refs(vec![dossier(
            "D1",
            Some(date(2025, 5, 13)),
            date(2025, 5, 13),
        )]);

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
        let repo =
            InMemoryDossierRepository::with_dossiers(vec![promulgated.clone(), unchanged.clone()]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let source = FakeAssemblySource::without_refs(vec![promulgated, unchanged]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute_with(RefreshScope::Full)
            .await
            .unwrap();

        assert_eq!(summary.written, 2);
        assert_eq!(summary.skipped_final, 0);
        assert_eq!(summary.skipped_unchanged, 0);
    }

    /// Decision de seance du jour de la derniere activite: la completer ne
    /// bouge ni la derniere activite ni le nombre d'actes.
    fn with_decision_act(
        mut dossier: NewLegislativeDossier,
        decision_vote: Option<&str>,
        conclusion_label: Option<&str>,
    ) -> NewLegislativeDossier {
        dossier.acts.push(LegislativeAct {
            date: dossier.last_activity_date,
            label: "D\u{00e9}cision".into(),
            code: Some("AN1-DEBATS-DEC".into()),
            decision_vote: decision_vote
                .map(|uid| crate::domain::scrutin::ScrutinUid::new(uid.into()).unwrap()),
            conclusion_label: conclusion_label.map(Into::into),
            conclusion_code: conclusion_label.map(|_| "TSORTF05".into()),
        });
        dossier
    }

    fn promulgated_parts(uid: &str) -> NewLegislativeDossier {
        let mut promulgated = dossier_parts(uid, Some(date(2025, 5, 13)), date(2026, 4, 21));
        promulgated.outcome = DossierOutcome::Promulgated {
            date: date(2026, 4, 21),
            publication: crate::domain::dossier::LawPublication {
                law_code: Some("2026-300".into()),
                jo_date: None,
                legifrance_url: None,
            },
        };
        promulgated
    }

    /// Constat du 22/11/2026 (DLR5L17N51095): l'AN ajoute `voteRefs` a une
    /// decision deja publiee. Sans reecriture, `decision_scrutin_uid` reste
    /// vide et le scrutin reste rattache au mauvais dossier (README §2).
    #[tokio::test]
    async fn a_decision_vote_added_to_an_existing_act_triggers_a_write() {
        let base = dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 11, 19));
        let stored = with_decision_act(base.clone(), None, Some("adopt\u{00e9}e"));
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(stored)]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(
            with_decision_act(base, Some("VTANR5L17V10080"), Some("adopt\u{00e9}e")),
        )]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_unchanged, 0);
    }

    /// Le sens d'une decision (« ou en est la loi ») se lit dans le
    /// `statutConclusion`: il peut arriver apres l'acte.
    #[tokio::test]
    async fn a_conclusion_added_to_an_existing_act_triggers_a_write() {
        let base = dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 11, 19));
        let stored = with_decision_act(base.clone(), None, None);
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(stored)]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(
            with_decision_act(base, None, Some("modifi\u{00e9}e")),
        )]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_unchanged, 0);
    }

    /// Le sort ne peut plus changer, mais la source peut encore completer une
    /// decision qui l'a precede: le scrutin qui a tranche doit etre recueilli.
    #[tokio::test]
    async fn a_decision_vote_added_after_promulgation_is_still_written() {
        let base = promulgated_parts("D1");
        let stored = with_decision_act(base.clone(), None, Some("adopt\u{00e9}e"));
        let repo = InMemoryDossierRepository::with_dossiers(vec![LegislativeDossier::new(stored)]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let source = FakeAssemblySource::without_refs(vec![LegislativeDossier::new(
            with_decision_act(base, Some("VTANR5L17V9097"), Some("adopt\u{00e9}e")),
        )]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 1);
        assert_eq!(summary.skipped_final, 0);
    }

    /// Une decision deja complete ne justifie aucune reecriture.
    #[tokio::test]
    async fn an_already_recorded_decision_vote_keeps_the_dossier_skipped() {
        let decided = LegislativeDossier::new(with_decision_act(
            dossier_parts("D1", Some(date(2025, 5, 13)), date(2026, 11, 19)),
            Some("VTANR5L17V10080"),
            Some("adopt\u{00e9}e"),
        ));
        let repo = InMemoryDossierRepository::with_dossiers(vec![decided.clone()]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let source = FakeAssemblySource::without_refs(vec![decided]);

        let summary = RefreshDossiers::new(&source, &repo, &actors, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(summary.written, 0);
        assert_eq!(summary.skipped_unchanged, 1);
    }
}
