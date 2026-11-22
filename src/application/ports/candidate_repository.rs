use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::candidate::CandidateId;
use crate::domain::theme::FamilyCode;

pub use super::RepositoryError;

#[derive(Debug, Clone)]
pub struct CandidateRecord {
    pub id: CandidateId,
    pub display_name: String,
    pub declared_on: NaiveDate,
    pub declaration_source_url: String,
    pub declaration_source_label: String,
    pub official_site_url: Option<String>,
    pub program_url: Option<String>,
    /// Au moins un extrait de programme enregistré, tous thèmes confondus :
    /// sans extrait, la candidature n'a rien à comparer.
    pub has_program_excerpts: bool,
    pub organizations: Vec<PoliticalOrganizationRecord>,
}

#[derive(Debug, Clone)]
pub struct PoliticalOrganizationRecord {
    pub label: String,
    pub official_url: Option<String>,
    pub source_url: String,
    pub source_label: String,
}

#[derive(Debug, Clone)]
pub struct CandidateProgramProposalRecord {
    pub candidate_id: CandidateId,
    pub family: FamilyCode,
    pub excerpt: String,
    pub source_url: String,
    pub source_label: String,
    pub source_published_on: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct CandidateParliamentaryGroupRecord {
    pub candidate_id: CandidateId,
    pub group_uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub linked_on: NaiveDate,
    pub source_url: String,
    pub source_label: String,
}

/// Source de programme dont le contenu a changé depuis sa première observation.
#[derive(Debug, Clone)]
pub struct ChangedSourceRecord {
    pub source_url: String,
    pub changed_at: DateTime<Utc>,
    /// Candidatures qui s'appuient sur cette source, pour orienter la revue.
    pub candidate_names: Vec<String>,
    /// Ce qui a changé, quand la lecture précédente avait gardé son texte.
    pub excerpt: Option<ChangeExcerpt>,
}

/// Fragments de texte ajoutés et retirés lors d'un changement de source.
/// Les listes sont tronquées ; les totaux comptent tous les fragments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeExcerpt {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub added_total: u32,
    pub removed_total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalChangeKind {
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct ProposalChange {
    pub candidate_id: CandidateId,
    pub candidate_name: String,
    pub family_code: FamilyCode,
    pub change: ProposalChangeKind,
    pub published_on: NaiveDate,
}

#[async_trait]
pub trait CandidateRepository: Send + Sync {
    /// Extraits ajoutés ou retirés par une publication entre `from` et `to` inclus.
    async fn proposal_changes_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ProposalChange>, RepositoryError>;

    /// Sources modifiées depuis `since`, à relire avant toute publication.
    async fn changed_sources(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ChangedSourceRecord>, RepositoryError>;

    /// Toutes les candidatures déjà établies par une déclaration primaire.
    async fn list_candidates(&self) -> Result<Vec<CandidateRecord>, RepositoryError>;

    /// Extraits déclarés, filtrés par thème seulement quand le lecteur le demande.
    async fn program_proposals(
        &self,
        candidate_ids: &[CandidateId],
        family: Option<FamilyCode>,
    ) -> Result<Vec<CandidateProgramProposalRecord>, RepositoryError>;

    /// Groupes associés par une source explicite, jamais par inférence de parti.
    async fn parliamentary_groups(
        &self,
        candidate_ids: &[CandidateId],
    ) -> Result<Vec<CandidateParliamentaryGroupRecord>, RepositoryError>;
}
