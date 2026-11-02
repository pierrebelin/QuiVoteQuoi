//! Port des citations d'amendement (rempli par un autre cycle du lot F2).

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::amendment_repository::AmendmentSummary;
use crate::application::ports::RepositoryError;
use crate::domain::amendment_citation::{
    CandidateAmendment, CitationCount, CitationOutcome, LinkMethod, UnlinkedCause,
};
use crate::domain::amendment::AmendmentUid;
use crate::domain::dossier::DossierUid;
use crate::domain::scrutin::{ScrutinUid, VoteTally};

/// Scrutin lu en base, avec le dossier publie que la source lui rattache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitingScrutin {
    pub uid: ScrutinUid,
    pub date: NaiveDate,
    pub subject: String,
    pub published_dossier: Option<DossierUid>,
}

/// Issue d'un numero cite, rattachee a son scrutin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCitation {
    pub scrutin: ScrutinUid,
    pub outcome: CitationOutcome,
}

/// Amendement cite, lu avec son dossier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedAmendmentRow {
    pub dossier_uid: DossierUid,
    pub summary: AmendmentSummary,
}

/// Numero cite par un scrutin et son issue stockee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationRow {
    pub scrutin: ScrutinUid,
    pub rank: u16,
    pub cited_number: Option<String>,
    pub amendment: Option<CitedAmendmentRow>,
    pub cause: Option<UnlinkedCause>,
    pub method: LinkMethod,
}

/// Scrutin ou un amendement a ete mis aux voix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentVote {
    pub amendment: AmendmentUid,
    pub scrutin_uid: ScrutinUid,
    pub scrutin_number: String,
    pub date: NaiveDate,
    pub tally: VoteTally,
}

#[async_trait]
pub trait AmendmentCitationRepository: Send + Sync {
    async fn citing_scrutins(&self) -> Result<Vec<CitingScrutin>, RepositoryError>;

    /// Liste vide: aucune lecture.
    async fn candidates_in(
        &self,
        dossiers: &[DossierUid],
    ) -> Result<Vec<CandidateAmendment>, RepositoryError>;

    /// Remplace toutes les citations en une transaction.
    async fn replace_citations(
        &self,
        citations: &[StoredCitation],
    ) -> Result<usize, RepositoryError>;

    /// Effectifs des citations stockees, groupes par cause et par methode.
    async fn citation_counts(&self) -> Result<Vec<CitationCount>, RepositoryError>;

    /// Citations stockees des scrutins demandes.
    async fn citations_of(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<CitationRow>, RepositoryError>;

    /// Scrutins qui ont mis aux voix les amendements demandes, chronologiques.
    async fn scrutins_of_amendments(
        &self,
        amendments: &[AmendmentUid],
    ) -> Result<Vec<AmendmentVote>, RepositoryError>;
}
