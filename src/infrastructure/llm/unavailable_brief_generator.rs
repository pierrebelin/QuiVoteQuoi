use async_trait::async_trait;

use crate::application::ports::brief_generator::{
    BriefGenerator, BriefRequest, GeneratedLine, SummaryGeneratorError,
};

use super::brief_prompt::PROMPT_VERSION;

/// Résumeur « En bref » sans clé de fournisseur : la passe lit les textes et
/// reprend les lignes connues, mais n'envoie rien. Ce n'est pas un échec du
/// modèle : les articles à résumer attendent la clé (CU-02).
pub struct UnavailableBriefGenerator;

#[async_trait]
impl BriefGenerator for UnavailableBriefGenerator {
    async fn generate(
        &self,
        _: &BriefRequest,
    ) -> Result<Vec<GeneratedLine>, SummaryGeneratorError> {
        Err(SummaryGeneratorError::Unavailable(
            "clé du fournisseur LLM absente".into(),
        ))
    }

    fn model(&self) -> &str {
        "aucun"
    }

    /// La version du prompt courant, même sans clé : l'avancement des dossiers
    /// reste lisible quand la clé arrive.
    fn prompt_version(&self) -> &str {
        PROMPT_VERSION
    }
}
