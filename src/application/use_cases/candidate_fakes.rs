//! Doublure partagee de `CandidateRepository` : etat configurable, appels notes.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::application::ports::candidate_repository::{
    CandidateParliamentaryGroupRecord, CandidateProgramProposalRecord, CandidateRecord,
    CandidateRepository, ChangedSourceRecord, PoliticalOrganizationRecord, ProposalChange,
    ProposalChangeKind, RepositoryError,
};
use crate::domain::candidate::CandidateId;
use crate::domain::theme::FamilyCode;

pub const REGISTRY: [&str; 6] = [
    "candidate-a",
    "candidate-b",
    "candidate-c",
    "candidate-d",
    "candidate-e",
    "candidate-f",
];

#[derive(Default)]
pub struct InMemoryCandidates {
    /// Historique des publications, servi par `proposal_changes_between`.
    pub changes: Vec<ProposalChange>,
    pub requested_proposals: Mutex<Vec<(Vec<String>, Option<FamilyCode>)>>,
    pub requested_periods: Mutex<Vec<(NaiveDate, NaiveDate)>>,
}

pub fn candidate(id: &str) -> CandidateRecord {
    CandidateRecord {
        id: CandidateId::new(id.into()).unwrap(),
        display_name: format!("Candidate {id}"),
        declared_on: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        declaration_source_url: "https://example.test/declaration".into(),
        declaration_source_label: "Déclaration".into(),
        official_site_url: None,
        program_url: None,
        has_program_excerpts: true,
        organizations: vec![PoliticalOrganizationRecord {
            label: "Organisation A".into(),
            official_url: None,
            source_url: "https://example.test/organisation".into(),
            source_label: "Source".into(),
        }],
    }
}

pub fn proposal_change(
    id: &str,
    name: &str,
    family: FamilyCode,
    change: ProposalChangeKind,
    published_on: NaiveDate,
) -> ProposalChange {
    ProposalChange {
        candidate_id: CandidateId::new(id.into()).unwrap(),
        candidate_name: name.into(),
        family_code: family,
        change,
        published_on,
    }
}

#[async_trait]
impl CandidateRepository for InMemoryCandidates {
    async fn proposal_changes_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ProposalChange>, RepositoryError> {
        self.requested_periods.lock().unwrap().push((from, to));
        Ok(self
            .changes
            .iter()
            .filter(|change| from <= change.published_on && change.published_on <= to)
            .cloned()
            .collect())
    }

    async fn changed_sources(
        &self,
        _: DateTime<Utc>,
    ) -> Result<Vec<ChangedSourceRecord>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn list_candidates(&self) -> Result<Vec<CandidateRecord>, RepositoryError> {
        Ok(REGISTRY.iter().map(|id| candidate(id)).collect())
    }

    async fn program_proposals(
        &self,
        candidate_ids: &[CandidateId],
        family: Option<FamilyCode>,
    ) -> Result<Vec<CandidateProgramProposalRecord>, RepositoryError> {
        self.requested_proposals.lock().unwrap().push((
            candidate_ids.iter().map(|id| id.as_str().to_string()).collect(),
            family,
        ));
        Ok(Vec::new())
    }

    async fn parliamentary_groups(
        &self,
        _: &[CandidateId],
    ) -> Result<Vec<CandidateParliamentaryGroupRecord>, RepositoryError> {
        Ok(Vec::new())
    }
}
