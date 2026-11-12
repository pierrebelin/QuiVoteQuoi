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
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::ports::assembly_source::SourceError;
    use crate::application::ports::dossier_repository::RepositoryError;
    use crate::application::use_cases::refresh_dossiers::tests::InMemoryActorRepository;
    use crate::domain::dossier::{DossierOutcome, LegislativeAct, NewLegislativeDossier, Score};

    struct InMemoryDossierRepository {
        dossiers: Mutex<HashMap<String, LegislativeDossier>>,
    }

    #[async_trait]
    impl DossierRepository for InMemoryDossierRepository {
        async fn load_states(
            &self,
        ) -> Result<
            std::collections::HashMap<String, crate::domain::dossier::StoredDossierState>,
            RepositoryError,
        > {
            unreachable!()
        }
        async fn save_all(
            &self,
            _dossiers: &[LegislativeDossier],
        ) -> Result<usize, RepositoryError> {
            unreachable!()
        }

        async fn find_recent(
            &self,
            _since: NaiveDate,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
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
            uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, RepositoryError> {
            let store = self.dossiers.lock().unwrap();
            Ok(store.get(uid.as_str()).cloned())
        }

        async fn find_by_uids(
            &self,
            _uids: &[crate::domain::dossier::DossierUid],
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }
    }

    struct FakeSource {
        dossiers: Mutex<HashMap<String, (LegislativeDossier, Vec<String>)>>,
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
            unreachable!()
        }

        async fn fetch_dossier_by_uid(
            &self,
            uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            let store = self.dossiers.lock().unwrap();
            Ok(store.get(uid.as_str()).map(|(d, _)| d.clone()))
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            let store = self.dossiers.lock().unwrap();
            Ok(store.get(uid.as_str()).cloned())
        }
    }

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
                },
                LegislativeAct {
                    date: NaiveDate::from_ymd_opt(2026, 6, 25).unwrap(),
                    label: "Vote solennel".into(),
                    code: None,
                    decision_vote: None,
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
        let mut map = HashMap::new();
        map.insert("DLR5L17N12345".into(), sample_dossier());

        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(map),
        };
        let source = FakeSource {
            dossiers: Mutex::new(HashMap::new()),
        };
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
        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(HashMap::new()),
        };
        let mut source_map = HashMap::new();
        source_map.insert(
            "DLR5L17N12345".into(),
            (sample_dossier(), vec!["PA111111".into()]),
        );
        let source = FakeSource {
            dossiers: Mutex::new(source_map),
        };
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
        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(HashMap::new()),
        };
        let source = FakeSource {
            dossiers: Mutex::new(HashMap::new()),
        };
        let actors = InMemoryActorRepository::with_deputy_changing_group();
        let uc = GetDossierDetail::new(&repo, &source, &actors);
        let uid = DossierUid::new("UNKNOWN".into()).unwrap();
        let result = uc.execute(&uid).await.unwrap();
        assert!(result.is_none());
    }
}
