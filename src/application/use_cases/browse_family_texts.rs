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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::fakes::theme_repository::{
        text_summary, InMemoryThemeRepository,
    };

    #[tokio::test]
    async fn serves_the_page_of_the_requested_family_with_its_total() {
        let repository = InMemoryThemeRepository::default();
        let [asked, other, ..] = FamilyCode::ALL;
        repository.family_pages.lock().unwrap().extend([
            (
                asked,
                TextPage {
                    items: vec![text_summary("plf-2026", "projet de loi de finances")],
                    total: 41,
                },
            ),
            (
                other,
                TextPage {
                    items: vec![text_summary("autre", "autre texte")],
                    total: 1,
                },
            ),
        ]);

        let page = BrowseFamilyTexts::new(&repository)
            .execute(asked, 20, 40)
            .await
            .unwrap();

        assert_eq!(page.total, 41);
        let keys: Vec<&str> = page.items.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, vec!["plf-2026"]);
        assert_eq!(*repository.page_requests.lock().unwrap(), vec![(20, 40)]);
    }

    #[tokio::test]
    async fn answers_an_empty_page_for_a_family_without_text() {
        let repository = InMemoryThemeRepository::default();

        let page = BrowseFamilyTexts::new(&repository)
            .execute(FamilyCode::ALL[0], 20, 0)
            .await
            .unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }
}
