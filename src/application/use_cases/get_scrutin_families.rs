use std::collections::HashMap;

use crate::application::ports::theme_repository::{
    AssignedFamily, RepositoryError, ThemeRepository,
};

/// Familles de chaque scrutin d'une page, en une seule lecture.
pub struct GetScrutinFamilies<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> GetScrutinFamilies<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        scrutin_uids: &[String],
    ) -> Result<HashMap<String, Vec<AssignedFamily>>, RepositoryError> {
        if scrutin_uids.is_empty() {
            return Ok(HashMap::new());
        }
        self.repository.families_of_scrutins(scrutin_uids).await
    }
}
