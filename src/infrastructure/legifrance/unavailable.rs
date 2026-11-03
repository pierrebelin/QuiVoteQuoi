use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::legal_text_source::{LegalTextSource, SourceError};
use crate::domain::cited_article::{CitedArticle, CitedWording};

/// Source mise en place quand les identifiants Légifrance sont absents : toute
/// demande échoue, les articles qui citent un article reconnu restent en
/// attente (RM-08).
pub struct UnavailableLegalTextSource;

#[async_trait]
impl LegalTextSource for UnavailableLegalTextSource {
    async fn wording(
        &self,
        _cited: &CitedArticle,
        _on: NaiveDate,
    ) -> Result<CitedWording, SourceError> {
        Err(SourceError::Download("legifrance not configured".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cited_article::CitedText;

    #[tokio::test]
    async fn should_fail_download_when_legifrance_is_not_configured() {
        let cited = CitedArticle::new(
            CitedText::Law("2024-322".to_string()),
            "51".to_string(),
            "51".to_string(),
            false,
        );
        let on = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();

        let result = UnavailableLegalTextSource.wording(&cited, on).await;

        assert!(
            matches!(&result, Err(SourceError::Download(reason)) if reason == "legifrance not configured"),
            "{result:?}"
        );
    }
}
