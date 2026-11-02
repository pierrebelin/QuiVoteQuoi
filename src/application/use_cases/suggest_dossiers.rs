use crate::application::ports::dossier_repository::{DossierRepository, RepositoryError};
use crate::domain::dossier::LegislativeDossier;

pub struct SuggestDossiers<'a> {
    repository: &'a dyn DossierRepository,
}

impl<'a> SuggestDossiers<'a> {
    pub fn new(repository: &'a dyn DossierRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, count: usize) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        self.repository.find_suggestions(count).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::ports::dossier_repository::RepositoryError;
    use crate::domain::dossier::{
        CurationStatus, DossierOutcome, DossierUid, LegislativeDossier, NewLegislativeDossier,
        Score,
    };

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

        async fn find_suggestions(
            &self,
            count: usize,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            let store = self.dossiers.lock().unwrap();
            let mut new_dossiers: Vec<_> = store
                .values()
                .filter(|d| d.curation_status() == CurationStatus::New)
                .cloned()
                .collect();
            new_dossiers.sort_by(|a, b| b.score().total().cmp(&a.score().total()));
            new_dossiers.truncate(count);
            Ok(new_dossiers)
        }

        async fn update_curation_status(
            &self,
            _uid: &DossierUid,
            _status: CurationStatus,
        ) -> Result<bool, RepositoryError> {
            unreachable!()
        }
    }

    fn make_dossier(uid: &str, total: u8, status: CurationStatus) -> LegislativeDossier {
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
            score: Score::new(5, 5, 5, total).unwrap(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            curation_status: status,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    #[tokio::test]
    async fn returns_top_new_dossiers_by_score() {
        let mut map = HashMap::new();
        map.insert("D1".into(), make_dossier("D1", 80, CurationStatus::New));
        map.insert("D2".into(), make_dossier("D2", 90, CurationStatus::New));
        map.insert("D3".into(), make_dossier("D3", 70, CurationStatus::New));

        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(map),
        };
        let uc = SuggestDossiers::new(&repo);
        let result = uc.execute(2).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].uid().as_str(), "D2");
        assert_eq!(result[1].uid().as_str(), "D1");
    }

    #[tokio::test]
    async fn excludes_non_new_dossiers() {
        let mut map = HashMap::new();
        map.insert(
            "D1".into(),
            make_dossier("D1", 90, CurationStatus::Selected),
        );
        map.insert("D2".into(), make_dossier("D2", 80, CurationStatus::New));
        map.insert(
            "D3".into(),
            make_dossier("D3", 70, CurationStatus::Dismissed),
        );

        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(map),
        };
        let uc = SuggestDossiers::new(&repo);
        let result = uc.execute(3).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].uid().as_str(), "D2");
    }

    #[tokio::test]
    async fn returns_empty_when_no_new_dossiers() {
        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(HashMap::new()),
        };
        let uc = SuggestDossiers::new(&repo);
        let result = uc.execute(3).await.unwrap();
        assert!(result.is_empty());
    }
}
