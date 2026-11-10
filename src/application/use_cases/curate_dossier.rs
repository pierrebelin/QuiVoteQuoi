use crate::application::ports::dossier_repository::{DossierRepository, RepositoryError};
use crate::domain::dossier::{CurationStatus, DossierUid};

#[derive(Debug, thiserror::Error)]
pub enum CurateError {
    #[error("{0}")]
    Repository(#[from] RepositoryError),
    #[error("dossier not found: {0}")]
    NotFound(String),
}

pub struct CurateDossier<'a> {
    repository: &'a dyn DossierRepository,
}

impl<'a> CurateDossier<'a> {
    pub fn new(repository: &'a dyn DossierRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        uid: &DossierUid,
        status: CurationStatus,
    ) -> Result<(), CurateError> {
        let updated = self.repository.update_curation_status(uid, status).await?;
        if !updated {
            return Err(CurateError::NotFound(uid.as_str().to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dossier::DossierOutcome;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::domain::dossier::{LegislativeDossier, NewLegislativeDossier};

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
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }

        async fn find_by_uids(
            &self,
            _uids: &[crate::domain::dossier::DossierUid],
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }

        async fn find_suggestions(
            &self,
            _count: usize,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }

        async fn update_curation_status(
            &self,
            uid: &DossierUid,
            status: CurationStatus,
        ) -> Result<bool, RepositoryError> {
            let mut store = self.dossiers.lock().unwrap();
            if let Some(d) = store.get_mut(uid.as_str()) {
                d.change_curation_status(status);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    fn make_dossier(uid: &str) -> LegislativeDossier {
        use crate::domain::dossier::Score;
        LegislativeDossier::new(NewLegislativeDossier {
            uid: DossierUid::new(uid.into()).unwrap(),
            title: format!("Dossier {uid}"),
            procedure: "PL".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date: None,
            last_activity_date: NaiveDate::from_ymd_opt(2026, 6, 25).unwrap(),
            last_activity_label: "Dépôt".into(),
            acts: vec![],
            documents: vec![],
            score: Score::new(5, 5, 5, 50).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            curation_status: CurationStatus::New,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    #[tokio::test]
    async fn updates_status_successfully() {
        let mut map = HashMap::new();
        map.insert("D1".into(), make_dossier("D1"));

        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(map),
        };
        let uc = CurateDossier::new(&repo);
        let uid = DossierUid::new("D1".into()).unwrap();
        uc.execute(&uid, CurationStatus::Selected).await.unwrap();

        let store = repo.dossiers.lock().unwrap();
        assert_eq!(store["D1"].curation_status(), CurationStatus::Selected);
    }

    #[tokio::test]
    async fn returns_not_found_for_unknown_uid() {
        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(HashMap::new()),
        };
        let uc = CurateDossier::new(&repo);
        let uid = DossierUid::new("UNKNOWN".into()).unwrap();
        let result = uc.execute(&uid, CurationStatus::Dismissed).await;

        assert!(matches!(result, Err(CurateError::NotFound(_))));
    }
}
