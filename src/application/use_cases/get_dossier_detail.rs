use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::assembly_source::AssemblySource;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::domain::actor::ActorUid;
use crate::domain::dossier::{DossierUid, LegislativeDossier};

#[derive(Debug, thiserror::Error)]
pub enum GetDossierError {
    #[error("{0}")]
    Repository(#[from] crate::application::ports::dossier_repository::RepositoryError),
    #[error("{0}")]
    Source(#[from] crate::application::ports::assembly_source::SourceError),
}

pub struct DossierDetailResult {
    pub dossier: LegislativeDossier,
    pub persisted: bool,
}

pub struct GetDossierDetail<'a> {
    repository: &'a dyn DossierRepository,
    source: &'a dyn AssemblySource,
    actor_repository: &'a dyn ActorRepository,
}

impl<'a> GetDossierDetail<'a> {
    pub fn new(
        repository: &'a dyn DossierRepository,
        source: &'a dyn AssemblySource,
        actor_repository: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            repository,
            source,
            actor_repository,
        }
    }

    pub async fn execute(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<DossierDetailResult>, GetDossierError> {
        if let Some(dossier) = self.repository.find_by_uid(uid).await? {
            return Ok(Some(DossierDetailResult {
                dossier,
                persisted: true,
            }));
        }

        if let Some((mut dossier, refs)) = self.source.fetch_dossier_by_uid_with_refs(uid).await? {
            if !refs.is_empty() && dossier.initiators().is_empty() {
                let uids: Vec<ActorUid> = refs
                    .into_iter()
                    .filter_map(|r| ActorUid::new(r).ok())
                    .collect();
                let directory = self.actor_repository.load_directory_for(&uids).await?;
                dossier.attach_initiators(&uids, &directory);
            }
            return Ok(Some(DossierDetailResult {
                dossier,
                persisted: false,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    use crate::application::use_cases::fakes::actor_repository::InMemoryActorRepository;
    use crate::application::use_cases::fakes::assembly_source::FakeAssemblySource;
    use crate::application::use_cases::fakes::dossier_repository::InMemoryDossierRepository;
    use crate::domain::dossier::{DossierOutcome, LegislativeAct, NewLegislativeDossier, Score};

    fn sample_dossier() -> LegislativeDossier {
        LegislativeDossier::new(NewLegislativeDossier {
            uid: DossierUid::new("DLR5L17N12345".into()).unwrap(),
            title: "Projet de loi de finances".into(),
            procedure: "PL".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date: NaiveDate::from_ymd_opt(2025, 5, 13),
            last_activity_date: NaiveDate::from_ymd_opt(2026, 6, 25).unwrap(),
            last_activity_label: "Vote solennel".into(),
            acts: vec![
                LegislativeAct {
                    date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
                    label: "Dépôt".into(),
                    code: Some("AN1-DEPOT".into()),
                    decision_vote: None,
                    conclusion_label: None,
                    conclusion_code: None,
                },
                LegislativeAct {
                    date: NaiveDate::from_ymd_opt(2026, 6, 25).unwrap(),
                    label: "Vote solennel".into(),
                    code: None,
                    decision_vote: None,
                    conclusion_label: None,
                    conclusion_code: None,
                },
            ],
            documents: vec![],
            score: Score::new(9, 10, 4, 85).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    #[tokio::test]
    async fn returns_persisted_dossier_from_repository() {
        let repo = InMemoryDossierRepository::with_dossiers(vec![sample_dossier()]);
        let source = FakeAssemblySource::default();
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let uc = GetDossierDetail::new(&repo, &source, &actors);
        let uid = DossierUid::new("DLR5L17N12345".into()).unwrap();
        let result = uc.execute(&uid).await.unwrap().unwrap();

        assert!(result.persisted);
        assert_eq!(result.dossier.uid().as_str(), "DLR5L17N12345");
        assert_eq!(result.dossier.acts().len(), 2);
    }

    #[tokio::test]
    async fn falls_back_to_source_and_dates_initiators_on_the_deposit_date() {
        let repo = InMemoryDossierRepository::new();
        let source = FakeAssemblySource::with_refs(vec![(
            sample_dossier(),
            vec!["PA111111".into()],
        )]);
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let uc = GetDossierDetail::new(&repo, &source, &actors);
        let uid = DossierUid::new("DLR5L17N12345".into()).unwrap();
        let result = uc.execute(&uid).await.unwrap().unwrap();

        assert!(!result.persisted);
        assert_eq!(result.dossier.initiators().len(), 1);
        let initiator = &result.dossier.initiators()[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert_eq!(initiator.group().unwrap().abbrev, "A");
        assert_eq!(
            initiator.reference_date(),
            NaiveDate::from_ymd_opt(2025, 5, 13)
        );
    }

    #[tokio::test]
    async fn returns_none_when_not_found_anywhere() {
        let repo = InMemoryDossierRepository::new();
        let source = FakeAssemblySource::default();
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let uc = GetDossierDetail::new(&repo, &source, &actors);
        let uid = DossierUid::new("UNKNOWN".into()).unwrap();
        let result = uc.execute(&uid).await.unwrap();
        assert!(result.is_none());
    }
}
