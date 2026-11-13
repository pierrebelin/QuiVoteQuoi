use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::theme_repository::AssignedFamily;
use crate::domain::final_vote::OfficialTextVersion;
use crate::domain::scrutin::VoteTally;
use crate::domain::theme::FamilyCode;

pub use super::RepositoryError;

/// Criteres de la liste des votes sur l'ensemble (CU-07).
///
/// Le filtre restreint l'affichage demande, jamais le contenu de la base
/// (RM-01). Le total renvoye est celui du filtre.
#[derive(Debug, Clone)]
pub struct FinalVoteFilter {
    pub family: Option<FamilyCode>,
    /// Groupes dont la position est demandee, dans l'ordre d'affichage.
    pub group_uids: Vec<String>,
    /// Bornes incluses de la periode; `None` = sans borne.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for FinalVoteFilter {
    fn default() -> Self {
        Self {
            family: None,
            group_uids: Vec::new(),
            from: None,
            to: None,
            limit: 20,
            offset: 0,
        }
    }
}

/// Ligne brute d'un vote sur l'ensemble, telle que la base la porte.
#[derive(Debug, Clone)]
pub struct FinalVoteRecord {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub subject: String,
    pub ballot_type_label: String,
    pub outcome_code: String,
    pub outcome_label: String,
    pub text_key: String,
    pub text_label: String,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    /// Présent seulement quand une source officielle établit explicitement le
    /// lien entre ce scrutin et cette version de document.
    pub official_text: Option<OfficialTextVersion>,
    pub synthesis: VoteTally,
    pub families: Vec<AssignedFamily>,
    pub tallies: Vec<GroupTallyRecord>,
}

/// Ventilation publiee d'un groupe sur un scrutin, avec l'identite du groupe.
#[derive(Debug, Clone)]
pub struct GroupTallyRecord {
    pub group_uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub member_count: Option<u16>,
    /// Position majoritaire publiee par la source (RM-02).
    pub majority_position: Option<String>,
    pub tally: VoteTally,
}

#[derive(Debug, Clone)]
pub struct FinalVotePage {
    pub items: Vec<FinalVoteRecord>,
    /// Nombre total de votes correspondant au filtre, pagination exclue.
    pub total: i64,
}

/// Groupe proposable au filtre, avec sa couverture.
///
/// `final_vote_count` rend visible qu'un groupe cree en cours de legislature
/// n'a pas de position sur les votes anterieurs: la lacune est affichee plutot
/// que comblee par un zero (README.md §2).
#[derive(Debug, Clone)]
pub struct GroupOption {
    pub uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub final_vote_count: i64,
}

/// Volumes de reference de la page, filtre exclu.
///
/// `with_family` rend visible l'avancement de la thematisation: un filtre par
/// theme ne peut pas trouver ce qui n'est pas encore rattache, et le taire
/// laisserait croire a une absence de vote (README.md §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalVoteTotals {
    pub total: i64,
    pub with_family: i64,
}

#[async_trait]
pub trait FinalVoteRepository: Send + Sync {
    async fn list_final_votes(
        &self,
        filter: &FinalVoteFilter,
    ) -> Result<FinalVotePage, RepositoryError>;

    /// Groupes ayant au moins une ventilation sur un vote sur l'ensemble.
    async fn groups(&self) -> Result<Vec<GroupOption>, RepositoryError>;

    /// Volumes de reference, filtre exclu.
    async fn totals(&self) -> Result<FinalVoteTotals, RepositoryError>;
}
