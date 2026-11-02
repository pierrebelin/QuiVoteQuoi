use async_trait::async_trait;

use crate::application::ports::dossier_group_actions_repository::GeneratedGroupSummary;
use crate::application::ports::dossier_summary_generator::{
    DossierSummaryGenerator, SummaryGeneratorError,
};
use crate::domain::dossier_group_facts::DossierGroupFacts;

pub struct UnavailableDossierSummaryGenerator;

#[async_trait]
impl DossierSummaryGenerator for UnavailableDossierSummaryGenerator {
    async fn generate(
        &self,
        _: &DossierGroupFacts,
    ) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
        Err(SummaryGeneratorError::Unavailable(
            "clé du fournisseur LLM absente".into(),
        ))
    }

    fn model(&self) -> &str {
        "aucun"
    }
    fn prompt_version(&self) -> &str {
        "aucune"
    }
}
