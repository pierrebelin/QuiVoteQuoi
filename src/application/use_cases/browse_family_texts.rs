use crate::application::ports::theme_repository::{RepositoryError, TextPage, ThemeRepository};
use crate::domain::theme::FamilyCode;

/// CU-04 — Textes d'une famille, page par page.
pub struct BrowseFamilyTexts<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> BrowseFamilyTexts<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        family: FamilyCode,
        limit: i64,
        offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        self.repository.texts_by_family(family, limit, offset).await
    }
}
