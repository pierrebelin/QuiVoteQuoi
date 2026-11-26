use crate::application::ports::theme_repository::{
    AssignedFamily, RepositoryError, ThemeRepository,
};

/// Familles d'un dossier, pour sa fiche.
pub struct GetDossierFamilies<'a> {
    repository: &'a dyn ThemeRepository,
}

impl<'a> GetDossierFamilies<'a> {
    pub fn new(repository: &'a dyn ThemeRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, dossier_uid: &str) -> Result<Vec<AssignedFamily>, RepositoryError> {
        self.repository.families_of_dossier(dossier_uid).await
    }
}
