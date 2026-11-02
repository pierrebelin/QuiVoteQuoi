use async_trait::async_trait;

use super::dossier_group_actions_repository::GeneratedGroupSummary;
use crate::domain::dossier_group_facts::DossierGroupFacts;

#[derive(Debug, thiserror::Error)]
pub enum SummaryGeneratorError {
    #[error("summary generator unavailable: {0}")]
    Unavailable(String),
    #[error("summary generator call failed: {0}")]
    Call(String),
    #[error("summary generator answer unusable: {0}")]
    Answer(String),
}

#[async_trait]
pub trait DossierSummaryGenerator: Send + Sync {
    async fn generate(
        &self,
        facts: &DossierGroupFacts,
    ) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError>;
    fn model(&self) -> &str;
    fn prompt_version(&self) -> &str;
}
