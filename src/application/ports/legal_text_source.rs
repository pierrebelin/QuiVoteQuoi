use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::cited_article::{CitedArticle, CitedWording};

pub use super::SourceError;

/// Source de la rédaction en vigueur des articles de code et de loi cités
/// (SPEC-en-bref RM-07).
///
/// Lecture seule : la rédaction est demandée au moment où un article est
/// résumé, rien n'est conservé.
#[async_trait]
pub trait LegalTextSource: Send + Sync {
    /// La rédaction de `cited` en vigueur le `on`, ou `Created` quand le texte
    /// crée l'article, ou `NotFound` quand la source ne le connaît pas.
    /// `SourceError` : la source n'a pas répondu, l'article citant reste en
    /// attente (RM-08).
    async fn wording(
        &self,
        cited: &CitedArticle,
        on: NaiveDate,
    ) -> Result<CitedWording, SourceError>;
}
