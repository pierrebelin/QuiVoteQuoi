//! Doublures partagees par les tests des use cases de rapprochement.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::amendment_citation_repository::{
    AmendmentCitationRepository, AmendmentVote, CitationRow, CitingScrutin, StoredCitation,
};
use crate::domain::amendment::AmendmentUid;
use crate::domain::scrutin::ScrutinUid;
use crate::application::ports::RepositoryError;
use crate::domain::amendment_citation::{CandidateAmendment, CitationCount, CitationResult};
use crate::domain::dossier::DossierUid;

#[derive(Default)]
pub struct InMemoryCitationRepository {
    pub scrutins: Mutex<Vec<CitingScrutin>>,
    pub amendments: Mutex<Vec<CandidateAmendment>>,
    pub citations: Mutex<Vec<StoredCitation>>,
    /// Dossiers demandes a chaque lecture de candidats, dans l'ordre.
    pub candidate_reads: Mutex<Vec<Vec<DossierUid>>>,
    pub candidates_unreadable: Mutex<bool>,
    /// Citations semees, servies par `citations_of`.
    pub rows: Mutex<Vec<CitationRow>>,
    /// Votes semes, servis par `scrutins_of_amendments`.
    pub votes: Mutex<Vec<AmendmentVote>>,
}

#[async_trait]
impl AmendmentCitationRepository for InMemoryCitationRepository {
    async fn citing_scrutins(&self) -> Result<Vec<CitingScrutin>, RepositoryError> {
        Ok(self.scrutins.lock().unwrap().clone())
    }

    async fn candidates_in(
        &self,
        dossiers: &[DossierUid],
    ) -> Result<Vec<CandidateAmendment>, RepositoryError> {
        self.candidate_reads.lock().unwrap().push(dossiers.to_vec());
        if *self.candidates_unreadable.lock().unwrap() {
            return Err(RepositoryError::Database("lecture impossible".to_string()));
        }
        Ok(self.amendments.lock().unwrap().clone())
    }

    async fn replace_citations(
        &self,
        citations: &[StoredCitation],
    ) -> Result<usize, RepositoryError> {
        *self.citations.lock().unwrap() = citations.to_vec();
        Ok(citations.len())
    }

    async fn citation_counts(&self) -> Result<Vec<CitationCount>, RepositoryError> {
        let counts = self
            .citations
            .lock()
            .unwrap()
            .iter()
            .map(|c| match &c.outcome.result {
                CitationResult::Linked { method, .. } => CitationCount {
                    cause: None,
                    by_cited_article: method.by_cited_article,
                    count: 1,
                },
                CitationResult::Unlinked(cause) => CitationCount {
                    cause: Some(*cause),
                    by_cited_article: false,
                    count: 1,
                },
            })
            .collect();
        Ok(counts)
    }

    async fn citations_of(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<CitationRow>, RepositoryError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .filter(|row| scrutins.contains(&row.scrutin))
            .cloned()
            .collect())
    }

    async fn scrutins_of_amendments(
        &self,
        amendments: &[AmendmentUid],
    ) -> Result<Vec<AmendmentVote>, RepositoryError> {
        let votes = self.votes.lock().unwrap();
        Ok(votes
            .iter()
            .filter(|vote| amendments.contains(&vote.amendment))
            .cloned()
            .collect())
    }
}
