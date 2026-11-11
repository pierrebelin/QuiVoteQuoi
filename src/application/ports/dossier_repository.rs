use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::theme_repository::AssignedFamily;
use crate::domain::dossier::{DossierUid, Initiative, LegislativeDossier, StoredDossierState};
use crate::domain::theme::FamilyCode;

pub use super::RepositoryError;

/// Ce à quoi le visiteur restreint la liste des dossiers.
///
/// Les critères restreignent l'affichage demandé, ils ne retirent rien de la
/// base (README.md §2). Aucun critère renseigné : la liste reste entière.
#[derive(Debug, Clone, Default)]
pub struct DossierCriteria {
    /// Fragment cherché dans le titre du dossier.
    pub search: Option<String>,
    /// Sorts retenus, tels que `DossierOutcome::kind` les écrit : le dossier
    /// doit porter l'un d'eux. Vide, aucun sort n'est écarté.
    pub outcome_kinds: Vec<String>,
    pub initiative: Option<Initiative>,
    /// Famille courante du dossier, héritée de ses textes ou propre (RM-06).
    pub family: Option<FamilyCode>,
    /// Vrai : seuls les dossiers auxquels la source rattache au moins un
    /// scrutin public restent.
    pub with_scrutins: bool,
}

impl DossierCriteria {
    pub fn is_empty(&self) -> bool {
        self.search.is_none()
            && self.outcome_kinds.is_empty()
            && self.initiative.is_none()
            && self.family.is_none()
            && !self.with_scrutins
    }
}

/// Ordre de la liste des dossiers. Le sort d'un dossier (`outcome`) est un
/// critère, pas un ordre.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DossierOrder {
    /// Dernière activité d'abord : la date qui dit où en est un texte.
    #[default]
    LastActivity,
    /// Dernier scrutin public d'abord ; un dossier sans scrutin vient après
    /// tous les autres.
    LastScrutin,
}

impl DossierOrder {
    /// Valeur du paramètre `sort` de l'API ; une valeur inconnue n'en est pas
    /// une.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "last_activity" => Some(Self::LastActivity),
            "last_scrutin" => Some(Self::LastScrutin),
            _ => None,
        }
    }
}

/// Décompte des scrutins publics d'un dossier, tels que la source les publie.
///
/// Adopté ou rejeté se lit dans le code de sort du scrutin (`Outcome`) : tout
/// scrutin non adopté est rejeté, il n'y a pas de troisième issue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DossierScrutinTally {
    pub count: i64,
    pub adopted: i64,
    pub rejected: i64,
    /// Date du dernier scrutin. `None` : aucun scrutin.
    pub last_date: Option<NaiveDate>,
}

/// Tranche de la liste des dossiers.
///
/// `total` est le nombre de dossiers correspondant aux critères, pagination
/// exclue : le visiteur doit voir combien de dossiers il regarde, pas seulement
/// ceux de sa page (README.md §2).
#[derive(Debug, Clone)]
pub struct DossierPage {
    pub items: Vec<DossierListing>,
    pub total: i64,
}

/// Ligne de la liste : le dossier avec ses initiateurs, mais sans actes ni
/// documents, et ce que la liste montre en plus de l'agrégat.
#[derive(Debug, Clone)]
pub struct DossierListing {
    pub dossier: LegislativeDossier,
    /// Familles courantes, héritées des textes que ses scrutins nomment, à
    /// défaut son propre rattachement (RM-06). Vide : aucune n'est ouverte.
    pub families: Vec<AssignedFamily>,
    /// Scrutins publics rattachés au dossier par la source.
    pub scrutins: DossierScrutinTally,
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
    /// Les dossiers répondant aux critères, dans l'ordre demandé ; à date
    /// égale, l'uid départage.
    async fn find_page(
        &self,
        criteria: &DossierCriteria,
        order: DossierOrder,
        limit: i64,
        offset: i64,
    ) -> Result<DossierPage, RepositoryError>;
    async fn find_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, RepositoryError>;
    /// Les dossiers nommés, sans actes, initiateurs ni documents : ce qu'il
    /// faut pour dire leur sort et leur étape. Une seule requête ; un uid
    /// inconnu est ignoré.
    async fn find_by_uids(
        &self,
        uids: &[DossierUid],
    ) -> Result<Vec<LegislativeDossier>, RepositoryError>;
}
