//! Port d'etat des amendements.
//!
//! Les read models vivent ici et non dans le domaine, comme pour les scrutins:
//! ce sont des formes de lecture, pas des regles.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::amendment::Amendment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentGroupOption {
    pub uid: String,
    pub label: String,
    pub abbrev: String,
}

pub use super::RepositoryError;

/// Bornage mecanique de l'affichage.
///
/// Ce n'est pas un filtre editorial: le total est toujours rendu, la borne est
/// annoncee, et rien n'est retire de la base (README.md §2, RM-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentPageRequest {
    pub limit: i64,
    pub offset: i64,
    pub search: Option<String>,
    pub fate_code: Option<String>,
    pub author_group_uid: Option<String>,
    pub number: Option<String>,
}

/// Borne dure. Une requete qui demande davantage est ramenee ici plutot que
/// refusee: le lecteur n'a pas a connaitre nos limites pour lire la page.
pub const MAX_PAGE_SIZE: i64 = 200;
pub const DEFAULT_PAGE_SIZE: i64 = 50;

impl AmendmentPageRequest {
    pub fn new(limit: Option<i64>, offset: Option<i64>) -> Self {
        Self {
            limit: limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE),
            offset: offset.unwrap_or(0).max(0),
            search: None,
            fate_code: None,
            author_group_uid: None,
            number: None,
        }
    }
}

impl Default for AmendmentPageRequest {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// Un signataire tel qu'il s'affiche: nomme, avec le groupe qu'il avait au
/// depot, et ce qui manque quand il manque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatoryRow {
    pub actor_uid: String,
    pub role: String,
    pub rank: i16,
    pub group_uid: Option<String>,
    pub group_origin: String,
    pub group_ambiguous: bool,
}

/// Ligne de liste. Porte l'expose sommaire entier: RM-03 interdit d'en servir
/// un extrait choisi, et le replier est une affaire d'affichage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentSummary {
    pub uid: String,
    pub number: String,
    pub target_title: String,
    pub target_kind: Option<String>,
    pub author_kind: String,
    pub author_actor_uid: Option<String>,
    pub author_label: Option<String>,
    pub author_group_uid: Option<String>,
    pub author_group_origin: String,
    pub author_group_ambiguous: bool,
    pub fate_code: String,
    pub fate_label: String,
    pub state_label: Option<String>,
    pub deposited_on: Option<NaiveDate>,
    pub summary: Option<String>,
    pub cosignatory_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentPage {
    pub items: Vec<AmendmentSummary>,
    pub total: i64,
}

/// Ce que le site n'a pas, chiffre. Une lacune tue est pire qu'une lacune
/// affichee (README.md §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DossierAmendmentCoverage {
    /// Amendements en base, tous dossiers confondus.
    ///
    /// Sans ce chiffre, une section vide est indechiffrable: elle peut vouloir
    /// dire « rien n'est encore ingere », « rien ne se rattache a ce dossier »
    /// ou « ce dossier n'a pas d'amendement ». Trois etats tres differents, et
    /// en presenter un pour les trois est exactement la lacune silencieuse que
    /// README.md §2 interdit.
    pub base_total: i64,
    pub total: i64,
    /// Amendements du dossier dont la source ne publie aucun expose sommaire.
    pub without_summary: i64,
    /// Amendements du dossier dont le sort publie sort du referentiel (RM-04).
    pub unknown_fates: i64,
}

#[async_trait]
pub trait AmendmentRepository: Send + Sync {
    /// Ecrit les amendements et leurs signataires. Rend le nombre ecrit.
    async fn save_amendments(&self, amendments: &[Amendment]) -> Result<usize, RepositoryError>;

    /// Amendements portant sur un texte du dossier, en ordre de depot.
    async fn by_dossier(
        &self,
        dossier_uid: &str,
        page: &AmendmentPageRequest,
    ) -> Result<AmendmentPage, RepositoryError>;

    async fn groups_by_dossier(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<AmendmentGroupOption>, RepositoryError>;

    async fn dossier_coverage(
        &self,
        dossier_uid: &str,
    ) -> Result<DossierAmendmentCoverage, RepositoryError>;

    async fn signatories_of(
        &self,
        amendment_uid: &str,
    ) -> Result<Vec<SignatoryRow>, RepositoryError>;

    /// Identite de la derniere archive entierement ingeree pour cette source.
    async fn last_archive_id(&self, label: &str) -> Result<Option<String>, RepositoryError>;

    /// Enregistre l'identite d'une archive entierement ingeree. N'est appelee
    /// qu'a l'issue d'une passe complete: une passe tronquee ne doit pas faire
    /// sauter la suivante.
    async fn remember_archive(&self, label: &str, id: &str) -> Result<(), RepositoryError>;
}
