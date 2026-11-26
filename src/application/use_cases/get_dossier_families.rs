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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::fakes::theme_repository::{
        assigned, InMemoryThemeRepository,
    };
    use crate::domain::theme::FamilyCode;
    use chrono::NaiveDate;

    #[tokio::test]
    async fn gives_the_dossier_its_current_families() {
        let repository = InMemoryThemeRepository::default();
        let on = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let [first, second, ..] = FamilyCode::ALL;
        repository.dossier_families.lock().unwrap().extend([
            ("DLR5L17N1".to_string(), vec![assigned(first, on)]),
            ("DLR5L17N2".to_string(), vec![assigned(second, on)]),
        ]);

        let families = GetDossierFamilies::new(&repository)
            .execute("DLR5L17N1")
            .await
            .unwrap();

        let codes: Vec<FamilyCode> = families.iter().map(|f| f.family).collect();
        assert_eq!(codes, vec![first]);
    }

    #[tokio::test]
    async fn gives_no_family_to_an_unassigned_dossier() {
        let repository = InMemoryThemeRepository::default();

        let families = GetDossierFamilies::new(&repository)
            .execute("DLR5L17N9")
            .await
            .unwrap();

        assert!(families.is_empty());
    }
}
