use std::collections::HashSet;

use crate::application::ports::candidate_repository::{
    CandidateParliamentaryGroupRecord, CandidateProgramProposalRecord, CandidateRecord,
    CandidateRepository, RepositoryError,
};
use crate::domain::candidate::CandidateId;
use crate::domain::theme::FamilyCode;

pub const MAX_COMPARED_CANDIDATES: usize = 4;

#[derive(Debug, Clone, Default)]
pub struct BrowseCandidatesCommand {
    pub candidate_ids: Vec<String>,
    pub family: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowseCandidatesError {
    #[error("unknown candidate: {0}")]
    UnknownCandidate(String),
    #[error("at most {MAX_COMPARED_CANDIDATES} candidates can be compared")]
    TooManyCandidates,
    #[error("unknown theme family: {0}")]
    UnknownFamily(String),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Clone)]
pub struct CandidateComparisonView {
    pub candidates: Vec<CandidateRecord>,
    pub selected: Vec<CandidateRecord>,
    pub proposals: Vec<CandidateProgramProposalRecord>,
    pub parliamentary_groups: Vec<CandidateParliamentaryGroupRecord>,
    pub selected_family: Option<FamilyCode>,
}

pub struct BrowseCandidates<'a> {
    repository: &'a dyn CandidateRepository,
}

impl<'a> BrowseCandidates<'a> {
    pub fn new(repository: &'a dyn CandidateRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        command: BrowseCandidatesCommand,
    ) -> Result<CandidateComparisonView, BrowseCandidatesError> {
        let selected_family = command
            .family
            .filter(|value| !value.is_empty())
            .map(|value| {
                FamilyCode::parse(&value).map_err(|_| BrowseCandidatesError::UnknownFamily(value))
            })
            .transpose()?;
        let candidates = self.repository.list_candidates().await?;

        let mut seen = HashSet::new();
        let requested: Vec<String> = command
            .candidate_ids
            .into_iter()
            .filter(|id| !id.is_empty() && seen.insert(id.clone()))
            .collect();
        if requested.len() > MAX_COMPARED_CANDIDATES {
            return Err(BrowseCandidatesError::TooManyCandidates);
        }

        let selected: Vec<CandidateRecord> = requested
            .iter()
            .map(|requested_id| {
                candidates
                    .iter()
                    .find(|candidate| candidate.id.as_str() == requested_id)
                    .cloned()
                    .ok_or_else(|| BrowseCandidatesError::UnknownCandidate(requested_id.clone()))
            })
            .collect::<Result<_, _>>()?;
        let selected_ids: Vec<CandidateId> = selected
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect();

        let (proposals, parliamentary_groups) = if selected_ids.is_empty() {
            // Sans sélection, un sujet donne les extraits de toutes les
            // candidatures : la vue « un sujet » les liste toutes, la borne
            // ne vaut que pour le face-à-face.
            match selected_family {
                Some(family) => {
                    let all_ids: Vec<CandidateId> = candidates
                        .iter()
                        .map(|candidate| candidate.id.clone())
                        .collect();
                    let proposals = if all_ids.is_empty() {
                        Vec::new()
                    } else {
                        self.repository
                            .program_proposals(&all_ids, Some(family))
                            .await?
                    };
                    (proposals, Vec::new())
                }
                None => (Vec::new(), Vec::new()),
            }
        } else {
            tokio::try_join!(
                self.repository
                    .program_proposals(&selected_ids, selected_family),
                self.repository.parliamentary_groups(&selected_ids),
            )?
        };

        Ok(CandidateComparisonView {
            candidates,
            selected,
            proposals,
            parliamentary_groups,
            selected_family,
        })
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::NaiveDate;

    use super::*;
    use crate::application::ports::candidate_repository::{
        ChangedSourceRecord, PoliticalOrganizationRecord,
    };

    #[derive(Default)]
    struct InMemoryCandidateRepository {
        requested_proposals: std::sync::Mutex<Vec<(Vec<String>, Option<FamilyCode>)>>,
    }

    fn candidate(id: &str) -> CandidateRecord {
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

    const REGISTRY: [&str; 6] = [
        "candidate-a",
        "candidate-b",
        "candidate-c",
        "candidate-d",
        "candidate-e",
        "candidate-f",
    ];

    #[async_trait]
    impl CandidateRepository for InMemoryCandidateRepository {
        async fn changed_sources(
            &self,
            _: chrono::DateTime<chrono::Utc>,
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

    #[tokio::test]
    async fn deduplicates_requested_candidates_without_selecting_anyone_else() {
        let view = BrowseCandidates::new(&InMemoryCandidateRepository::default())
            .execute(BrowseCandidatesCommand {
                candidate_ids: vec!["candidate-a".into(), "candidate-a".into()],
                family: None,
            })
            .await
            .unwrap();
        assert_eq!(view.selected.len(), 1);
        assert_eq!(view.proposals.len(), 0);
    }

    #[tokio::test]
    async fn rejects_a_candidate_not_in_the_declared_registry() {
        let error = BrowseCandidates::new(&InMemoryCandidateRepository::default())
            .execute(BrowseCandidatesCommand {
                candidate_ids: vec!["unknown".into()],
                family: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(error, BrowseCandidatesError::UnknownCandidate(_)));
    }

    #[tokio::test]
    async fn a_theme_without_selection_reads_the_excerpts_of_every_candidate() {
        let repository = InMemoryCandidateRepository::default();
        let view = BrowseCandidates::new(&repository)
            .execute(BrowseCandidatesCommand {
                candidate_ids: Vec::new(),
                family: Some("protection-sociale-retraites".into()),
            })
            .await
            .unwrap();
        assert!(view.selected.is_empty());
        let requested = repository.requested_proposals.lock().unwrap();
        assert_eq!(requested.len(), 1);
        assert_eq!(requested[0].0, REGISTRY.map(String::from).to_vec());
        assert_eq!(
            requested[0].1.map(|family| family.as_str().to_string()),
            Some("protection-sociale-retraites".to_string())
        );
    }

    #[tokio::test]
    async fn no_theme_and_no_selection_reads_no_excerpt() {
        let repository = InMemoryCandidateRepository::default();
        BrowseCandidates::new(&repository)
            .execute(BrowseCandidatesCommand::default())
            .await
            .unwrap();
        assert!(repository.requested_proposals.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_face_to_face_stays_bounded() {
        let error = BrowseCandidates::new(&InMemoryCandidateRepository::default())
            .execute(BrowseCandidatesCommand {
                candidate_ids: REGISTRY[..MAX_COMPARED_CANDIDATES + 1]
                    .iter()
                    .map(|id| id.to_string())
                    .collect(),
                family: Some("protection-sociale-retraites".into()),
            })
            .await
            .unwrap_err();
        assert!(matches!(error, BrowseCandidatesError::TooManyCandidates));
    }
}
