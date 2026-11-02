use chrono::NaiveDate;

use crate::application::ports::dossier_repository::{DossierRepository, RepositoryError};
use crate::domain::dossier::LegislativeDossier;

pub struct FetchRecentDossiers<'a> {
    repository: &'a dyn DossierRepository,
}

impl<'a> FetchRecentDossiers<'a> {
    pub fn new(repository: &'a dyn DossierRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        days: u32,
        today: NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        let since = today - chrono::Duration::days(days as i64);
        self.repository.find_recent(since).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::ports::dossier_repository::RepositoryError;
    use crate::domain::dossier::{
        CurationStatus, DossierOutcome, DossierUid, LegislativeDossier, NewLegislativeDossier,
    };
    use crate::domain::scoring::compute_score;

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
            since: NaiveDate,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            let store = self.dossiers.lock().unwrap();
            let mut result: Vec<_> = store
                .values()
                .filter(|d| d.last_activity_date() >= since)
                .cloned()
                .collect();
            result.sort_by(|a, b| b.last_activity_date().cmp(&a.last_activity_date()));
            Ok(result)
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
            _count: usize,
        ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
            unreachable!()
        }

        async fn update_curation_status(
            &self,
            _uid: &DossierUid,
            _status: CurationStatus,
        ) -> Result<bool, RepositoryError> {
            unreachable!()
        }
    }

    fn make_dossier(
        uid: &str,
        title: &str,
        procedure: &str,
        date: NaiveDate,
        label: &str,
    ) -> LegislativeDossier {
        let score = compute_score(title, label, 0);
        LegislativeDossier::new(NewLegislativeDossier {
            uid: DossierUid::new(uid.into()).unwrap(),
            title: title.into(),
            procedure: procedure.into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date: None,
            last_activity_date: date,
            last_activity_label: label.into(),
            acts: vec![],
            documents: vec![],
            score,
            current_stage: None,
            initiators: vec![],
            committee: None,
            curation_status: CurationStatus::New,
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    #[tokio::test]
    async fn returns_dossiers_sorted_by_date_desc() {
        let mut map = HashMap::new();
        let d1 = make_dossier(
            "D1",
            "Ancien",
            "PL",
            NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            "Dépôt",
        );
        let d2 = make_dossier(
            "D2",
            "Récent",
            "PPL",
            NaiveDate::from_ymd_opt(2026, 6, 27).unwrap(),
            "Vote",
        );
        map.insert("D1".into(), d1);
        map.insert("D2".into(), d2);

        let repo = InMemoryDossierRepository {
            dossiers: Mutex::new(map),
        };
        let uc = FetchRecentDossiers::new(&repo);
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let result = uc.execute(365, today).await.unwrap();

        assert_eq!(result[0].uid().as_str(), "D2");
        assert_eq!(result[1].uid().as_str(), "D1");
    }
}
