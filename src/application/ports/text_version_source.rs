use async_trait::async_trait;

use crate::domain::law_text::VersionArticle;
use crate::domain::text_version::TextVersion;

pub use super::SourceError;

/// Source des versions d'un texte de loi et de leur texte intégral
/// (SPEC-en-bref CU-01).
///
/// La source publie les documents de tous les dossiers en une archive : il n'y
/// a pas de requête par dossier. Le texte d'une version, lui, se lit version par
/// version.
#[async_trait]
pub trait TextVersionSource: Send + Sync {
    /// Les versions de tous les dossiers (RM-01), non lues.
    async fn fetch_versions(&self) -> Result<Vec<TextVersion>, SourceError>;

    /// Les articles du dispositif d'une version publiée par l'Assemblée
    /// (RM-04). `None` : l'Assemblée ne publie pas ce texte.
    async fn version_articles(
        &self,
        version: &TextVersion,
    ) -> Result<Option<Vec<VersionArticle>>, SourceError>;
}
