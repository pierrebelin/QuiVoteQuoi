use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::dataset::DatasetShape;
use crate::domain::scrutin::{Scrutin, ScrutinUid, VoteTally};

pub use super::RepositoryError;

/// Criteres de la liste des scrutins (CU-02).
///
/// Les filtres restreignent l'affichage a la demande du visiteur; ils ne
/// retirent rien de la base (RM-01).
#[derive(Debug, Clone)]
pub struct ScrutinFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub outcome_code: Option<String>,
    pub ballot_type_code: Option<String>,
    /// `Some(true)` : seulement les scrutins rattaches a un dossier.
    /// `Some(false)` : seulement ceux qui n'en ont pas.
    pub with_dossier: Option<bool>,
    pub dossier_uid: Option<String>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for ScrutinFilter {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            outcome_code: None,
            ballot_type_code: None,
            with_dossier: None,
            dossier_uid: None,
            search: None,
            limit: 50,
            offset: 0,
        }
    }
}

/// Ligne de liste: ce qui tient sans deplier le scrutin.
#[derive(Debug, Clone)]
pub struct ScrutinSummary {
    pub uid: String,
    pub number: String,
    pub legislature: u16,
    pub date: NaiveDate,
    pub subject: String,
    pub ballot_type_label: String,
    pub outcome_code: String,
    pub outcome_label: String,
    pub tally: VoteTally,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    /// Declenche la mention de methode (RM-03).
    pub has_reconstructed_tallies: bool,
}

#[derive(Debug, Clone)]
pub struct ScrutinPage {
    pub items: Vec<ScrutinSummary>,
    /// Nombre total de scrutins correspondant au filtre, pagination exclue.
    pub total: i64,
}

#[async_trait]
pub trait ScrutinRepository: Send + Sync {
    /// Ecrit les scrutins et tout ce qu'ils portent. Reecrit integralement les
    /// scrutins fournis: une mise au point ajoutee apres coup doit apparaitre.
    async fn save_scrutins(&self, scrutins: &[Scrutin]) -> Result<usize, RepositoryError>;

    async fn list(&self, filter: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError>;

    async fn by_uid(&self, uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError>;

    /// Scrutins que la source rattache a ce dossier (CU-04).
    async fn by_dossier(&self, dossier_uid: &str) -> Result<Vec<ScrutinSummary>, RepositoryError>;

    /// Read model de la page « Comprendre » : volumétrie et répartitions.
    async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError>;
}
