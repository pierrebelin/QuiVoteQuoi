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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::fakes::theme_repository::{
        assigned, InMemoryThemeRepository,
    };
    use crate::domain::theme::FamilyCode;
    use chrono::NaiveDate;

    #[tokio::test]
    async fn gives_each_requested_scrutin_its_families_and_nothing_else() {
        let repository = InMemoryThemeRepository::default();
        let on = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let [first, second, ..] = FamilyCode::ALL;
        repository.scrutin_families.lock().unwrap().extend([
            ("VTANR5L17V1".to_string(), vec![assigned(first, on)]),
            ("VTANR5L17V2".to_string(), vec![assigned(second, on)]),
        ]);

        let families = GetScrutinFamilies::new(&repository)
            .execute(&["VTANR5L17V1".to_string()])
            .await
            .unwrap();

        assert_eq!(families.len(), 1);
        assert_eq!(families["VTANR5L17V1"][0].family, first);
    }

    #[tokio::test]
    async fn answers_an_empty_map_for_an_empty_page() {
        let repository = InMemoryThemeRepository::default();
        repository.scrutin_families.lock().unwrap().insert(
            "VTANR5L17V1".to_string(),
            vec![assigned(
                FamilyCode::ALL[0],
                NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            )],
        );

        let families = GetScrutinFamilies::new(&repository)
            .execute(&[])
            .await
            .unwrap();

        assert!(families.is_empty());
    }
}
