use crate::application::ports::dossier_repository::{
    DossierCriteria, DossierPage, DossierRepository, RepositoryError,
};

pub const DEFAULT_PER_PAGE: i64 = 20;
pub const MAX_PER_PAGE: i64 = 100;

/// Ce que le visiteur demande à voir : une page, éventuellement restreinte.
///
/// Les bornes hors limites sont ramenées plutôt que refusées : la page 9999
/// renvoie une tranche vide avec le total réel, pas une erreur. Le visiteur
/// doit voir combien de dossiers existent.
#[derive(Debug, Clone)]
pub struct DossierQuery {
    page: i64,
    per_page: i64,
    criteria: DossierCriteria,
}

impl DossierQuery {
    pub fn new(page: i64, per_page: i64, criteria: DossierCriteria) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, MAX_PER_PAGE),
            criteria,
        }
    }

    pub fn page(&self) -> i64 {
        self.page
    }

    pub fn per_page(&self) -> i64 {
        self.per_page
    }

    pub fn criteria(&self) -> &DossierCriteria {
        &self.criteria
    }

    fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }
}

impl Default for DossierQuery {
    fn default() -> Self {
        Self::new(1, DEFAULT_PER_PAGE, DossierCriteria::default())
    }
}

/// Parcourir les dossiers, du plus récent au plus ancien.
///
/// Aucun seuil de score, aucune sélection éditoriale : sans critère la liste
/// est entière (README.md §2), et les critères du visiteur ne restreignent que
/// son affichage. Le tri porte sur la dernière activité — c'est la date qui dit
/// où en est un texte.
pub struct BrowseDossiers<'a> {
    repository: &'a dyn DossierRepository,
}

impl<'a> BrowseDossiers<'a> {
    pub fn new(repository: &'a dyn DossierRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, query: DossierQuery) -> Result<DossierPage, RepositoryError> {
        self.repository
            .find_page(query.criteria(), query.per_page(), query.offset())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::sync::Mutex;

    use crate::domain::dossier::{
        CurationStatus, DossierOutcome, DossierUid, Initiative, LegislativeDossier,
        NewLegislativeDossier,
    };
    use crate::domain::scoring::compute_score;

    struct InMemoryDossierRepository {
        dossiers: Mutex<Vec<LegislativeDossier>>,
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
            criteria: &DossierCriteria,
            limit: i64,
            offset: i64,
        ) -> Result<DossierPage, RepositoryError> {
            let store = self.dossiers.lock().unwrap();
            let mut kept: Vec<_> = store
                .iter()
                .filter(|d| matches(d, criteria))
                .cloned()
                .collect();
            kept.sort_by(|a, b| b.last_activity_date().cmp(&a.last_activity_date()));
            Ok(DossierPage {
                total: kept.len() as i64,
                items: kept
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect(),
            })
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

    /// Le fake reproduit les critères que le dépôt applique en SQL : sans cela
    /// les tests de filtre ne vérifieraient que l'appel, pas son effet.
    fn matches(dossier: &LegislativeDossier, criteria: &DossierCriteria) -> bool {
        let title_matches = criteria.search.as_ref().is_none_or(|needle| {
            dossier
                .title()
                .to_lowercase()
                .contains(&needle.to_lowercase())
        });
        let outcome_matches = criteria
            .outcome_kind
            .as_ref()
            .is_none_or(|kind| dossier.outcome().kind() == kind);
        let initiative_matches = criteria.initiative.is_none_or(|initiative| {
            dossier
                .procedure()
                .starts_with(initiative.procedure_prefix())
        });

        title_matches && outcome_matches && initiative_matches
    }

    fn dossier_parts(uid: &str, day: u32) -> NewLegislativeDossier {
        NewLegislativeDossier {
            uid: DossierUid::new(uid.into()).unwrap(),
            title: uid.into(),
            procedure: "Projet de loi".into(),
            legislature: 17,
            url: None,
            summary: None,
            deposit_date: None,
            last_activity_date: NaiveDate::from_ymd_opt(2026, 6, day).unwrap(),
            last_activity_label: "Dépôt".into(),
            acts: vec![],
            documents: vec![],
            score: compute_score(uid, "Dépôt", 0),
            current_stage: None,
            initiators: vec![],
            committee: None,
            curation_status: CurationStatus::New,
            outcome: DossierOutcome::NoRecordedConclusion,
        }
    }

    fn dossier(uid: &str, day: u32) -> LegislativeDossier {
        LegislativeDossier::new(dossier_parts(uid, day))
    }

    fn repo(count: u32) -> InMemoryDossierRepository {
        InMemoryDossierRepository {
            dossiers: Mutex::new((1..=count).map(|d| dossier(&format!("D{d}"), d)).collect()),
        }
    }

    fn query(page: i64, per_page: i64) -> DossierQuery {
        DossierQuery::new(page, per_page, DossierCriteria::default())
    }

    #[tokio::test]
    async fn first_page_holds_the_most_recent_dossiers() {
        let repo = repo(5);
        let page = BrowseDossiers::new(&repo)
            .execute(query(1, 2))
            .await
            .unwrap();

        assert_eq!(page.total, 5);
        let uids: Vec<_> = page.items.iter().map(|d| d.uid().as_str()).collect();
        assert_eq!(uids, vec!["D5", "D4"]);
    }

    #[tokio::test]
    async fn following_page_continues_without_repeating() {
        let repo = repo(5);
        let page = BrowseDossiers::new(&repo)
            .execute(query(2, 2))
            .await
            .unwrap();

        let uids: Vec<_> = page.items.iter().map(|d| d.uid().as_str()).collect();
        assert_eq!(uids, vec!["D3", "D2"]);
    }

    /// Une page au-delà du dernier dossier reste une réponse valide.
    #[tokio::test]
    async fn page_beyond_the_end_is_empty_and_still_reports_the_total() {
        let repo = repo(5);
        let page = BrowseDossiers::new(&repo)
            .execute(query(99, 20))
            .await
            .unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.total, 5);
    }

    #[test]
    fn out_of_range_parameters_are_brought_back_within_bounds() {
        assert_eq!(query(0, 20).page(), 1);
        assert_eq!(query(-3, 20).page(), 1);
        assert_eq!(query(1, 0).per_page(), 1);
        assert_eq!(query(1, 10_000).per_page(), MAX_PER_PAGE);
    }

    fn mixed_repo() -> InMemoryDossierRepository {
        let mut promulgated = dossier_parts("Loi de finances", 1);
        promulgated.outcome = DossierOutcome::Promulgated {
            date: NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            publication: crate::domain::dossier::LawPublication {
                law_code: None,
                jo_date: None,
                legifrance_url: None,
            },
        };

        let mut proposition = dossier_parts("R\u{00e9}forme du logement", 2);
        proposition.procedure = "Proposition de loi ordinaire".into();

        InMemoryDossierRepository {
            dossiers: Mutex::new(vec![
                LegislativeDossier::new(promulgated),
                LegislativeDossier::new(proposition),
                dossier("Loi agricole", 3),
            ]),
        }
    }

    #[tokio::test]
    async fn search_keeps_only_the_matching_titles() {
        let page = BrowseDossiers::new(&mixed_repo())
            .execute(DossierQuery::new(
                1,
                20,
                DossierCriteria {
                    search: Some("loi".into()),
                    ..Default::default()
                },
            ))
            .await
            .unwrap();

        let titles: Vec<_> = page.items.iter().map(|d| d.title()).collect();
        assert_eq!(titles, vec!["Loi agricole", "Loi de finances"]);
    }

    #[tokio::test]
    async fn outcome_criterion_keeps_only_that_outcome() {
        let page = BrowseDossiers::new(&mixed_repo())
            .execute(DossierQuery::new(
                1,
                20,
                DossierCriteria {
                    outcome_kind: Some("promulgated".into()),
                    ..Default::default()
                },
            ))
            .await
            .unwrap();

        let titles: Vec<_> = page.items.iter().map(|d| d.title()).collect();
        assert_eq!(titles, vec!["Loi de finances"]);
    }

    #[tokio::test]
    async fn initiative_criterion_separates_projets_from_propositions() {
        let page = BrowseDossiers::new(&mixed_repo())
            .execute(DossierQuery::new(
                1,
                20,
                DossierCriteria {
                    initiative: Some(Initiative::Parliamentary),
                    ..Default::default()
                },
            ))
            .await
            .unwrap();

        let titles: Vec<_> = page.items.iter().map(|d| d.title()).collect();
        assert_eq!(titles, vec!["R\u{00e9}forme du logement"]);
    }

    /// Le total suit les critères : le visiteur voit combien de dossiers il
    /// regarde, pas combien la base en contient (README.md §2).
    #[tokio::test]
    async fn total_counts_the_filtered_dossiers_not_the_whole_base() {
        let page = BrowseDossiers::new(&mixed_repo())
            .execute(DossierQuery::new(
                1,
                1,
                DossierCriteria {
                    search: Some("loi".into()),
                    ..Default::default()
                },
            ))
            .await
            .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.total, 2);
    }

    #[tokio::test]
    async fn criteria_combine_and_can_leave_nothing() {
        let page = BrowseDossiers::new(&mixed_repo())
            .execute(DossierQuery::new(
                1,
                20,
                DossierCriteria {
                    search: Some("loi".into()),
                    outcome_kind: Some("promulgated".into()),
                    initiative: Some(Initiative::Parliamentary),
                },
            ))
            .await
            .unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }
}
