use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::dossier::{
    CurationStatus, DossierUid, Initiative, LegislativeDossier, StoredDossierState,
};

pub use super::RepositoryError;

/// Ce à quoi le visiteur restreint la liste des dossiers.
///
/// Les critères restreignent l'affichage demandé, ils ne retirent rien de la
/// base (README.md §2). Aucun critère renseigné : la liste reste entière.
#[derive(Debug, Clone, Default)]
pub struct DossierCriteria {
    /// Fragment cherché dans le titre du dossier.
    pub search: Option<String>,
    /// Sort du dossier, tel que `DossierOutcome::kind` l'écrit.
    pub outcome_kind: Option<String>,
    pub initiative: Option<Initiative>,
}

impl DossierCriteria {
    pub fn is_empty(&self) -> bool {
        self.search.is_none() && self.outcome_kind.is_none() && self.initiative.is_none()
    }
}

/// Tranche de la liste des dossiers.
///
/// `total` est le nombre de dossiers correspondant aux critères, pagination
/// exclue : le visiteur doit voir combien de dossiers il regarde, pas seulement
/// ceux de sa page (README.md §2).
#[derive(Debug, Clone)]
pub struct DossierPage {
    pub items: Vec<LegislativeDossier>,
    pub total: i64,
}

#[async_trait]
pub trait DossierRepository: Send + Sync {
    async fn save_all(&self, dossiers: &[LegislativeDossier]) -> Result<usize, RepositoryError>;
    /// Etat stocke de tous les dossiers, indexe par uid. Une seule requete.
    async fn load_states(&self) -> Result<HashMap<String, StoredDossierState>, RepositoryError>;
    async fn find_recent(
        &self,
        since: NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError>;
    /// Les dossiers répondant aux critères, du plus récent au plus ancien.
    async fn find_page(
        &self,
        criteria: &DossierCriteria,
        limit: i64,
        offset: i64,
    ) -> Result<DossierPage, RepositoryError>;
    async fn find_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, RepositoryError>;
    async fn find_suggestions(
        &self,
        count: usize,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError>;
    async fn update_curation_status(
        &self,
        uid: &DossierUid,
        status: CurationStatus,
    ) -> Result<bool, RepositoryError>;
}
