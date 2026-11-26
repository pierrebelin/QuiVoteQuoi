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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::fakes::theme_repository::{
        text_summary, InMemoryThemeRepository,
    };

    #[tokio::test]
    async fn serves_the_unassigned_texts_page_with_its_total() {
        let repository = InMemoryThemeRepository::default();
        *repository.unassigned_page.lock().unwrap() = Some(TextPage {
            items: vec![text_summary("ppl-chasse", "proposition de loi chasse")],
            total: 7,
        });

        let page = BrowseUnassignedTexts::new(&repository)
            .execute(5, 5)
            .await
            .unwrap();

        assert_eq!(page.total, 7);
        assert_eq!(page.items[0].key, "ppl-chasse");
        assert_eq!(*repository.page_requests.lock().unwrap(), vec![(5, 5)]);
    }
}
