use async_trait::async_trait;

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::domain::theme::ProposedFamily;

/// Classifieur de repli quand aucune cle n'est fournie (BYOK).
///
/// Il echoue franchement plutot que de rendre une liste vide: un texte sans
/// famille faute de cle ne doit pas etre confondu avec un texte que le modele
/// n'a pas su rattacher. La page methode distingue les deux causes.
pub struct UnavailableThemeClassifier;

#[async_trait]
impl ThemeClassifier for UnavailableThemeClassifier {
    async fn propose_batch(
        &self,
        _labels: &[String],
    ) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
        Err(ClassifierError::Unavailable(
            "clé du fournisseur LLM absente".to_string(),
        ))
    }

    fn model(&self) -> &str {
        "aucun"
    }

    fn prompt_version(&self) -> &str {
        "aucune"
    }

    fn batch_size(&self) -> usize {
        1
    }
}
