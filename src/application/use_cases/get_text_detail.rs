use crate::application::ports::theme_repository::{RepositoryError, TextSummary, ThemeRepository};
use crate::domain::theme::{SubjectRef, TextKey, ThemeAssignment, ThemeProposal};

/// Detail d'un texte: ses familles courantes, son historique, la proposition
/// qui l'a produit.
#[derive(Debug, Clone)]
pub struct TextDetail {
    pub summary: TextSummary,
    pub history: Vec<ThemeAssignment>,
    pub proposal: Option<ThemeProposal>,
}

/// CU-04 — Fiche d'un texte.
pub struct GetTextDetail<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> GetTextDetail<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, key: &TextKey) -> Result<Option<TextDetail>, RepositoryError> {
        let Some(summary) = self.repository.text_by_key(key).await? else {
            return Ok(None);
        };
        let subject = SubjectRef::Text(key.clone());
        let history = self.repository.assignment_history(&subject).await?;
        let proposal = self.repository.latest_proposal(&subject).await?;
        Ok(Some(TextDetail {
            summary,
            history,
            proposal,
        }))
    }
}
