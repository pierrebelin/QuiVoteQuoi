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

    use crate::application::use_cases::fakes::dossier_repository::InMemoryDossierRepository;
    use crate::domain::dossier::{
        DossierOutcome, DossierUid, LegislativeDossier, NewLegislativeDossier,
    };
    use crate::domain::scoring::compute_score;

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
            outcome: DossierOutcome::NoRecordedConclusion,
        })
    }

    #[tokio::test]
    async fn returns_dossiers_sorted_by_date_desc() {
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

        let repo = InMemoryDossierRepository::with_dossiers(vec![d1, d2]);
        let uc = FetchRecentDossiers::new(&repo);
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let result = uc.execute(365, today).await.unwrap();

        assert_eq!(result[0].uid().as_str(), "D2");
        assert_eq!(result[1].uid().as_str(), "D1");
    }
}
