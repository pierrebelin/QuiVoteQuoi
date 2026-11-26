use crate::application::ports::theme_repository::{RepositoryError, TextPage, ThemeRepository};

/// CU-05 — Les non rattaches restent atteignables depuis toute page de theme
/// (RM-01).
pub struct BrowseUnassignedTexts<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> BrowseUnassignedTexts<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, limit: i64, offset: i64) -> Result<TextPage, RepositoryError> {
        self.repository.unassigned_texts(limit, offset).await
    }
}
