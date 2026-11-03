use async_trait::async_trait;

use crate::domain::dossier::DossierUid;
use crate::domain::law_text::VersionArticle;
use crate::domain::text_version::{BriefCandidate, DocumentUid, TextVersion};

pub use super::RepositoryError;

/// Résultat de la lecture du texte d'une version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextReading {
    /// Les articles reconnus : la version est lue.
    Articles(Vec<VersionArticle>),
    /// Texte introuvable ou sans aucun article reconnu : la version est notée
    /// manquante, définitivement (CU-01, Erreurs).
    Missing,
}

/// Versions enregistrées, et articles de celles dont le texte a été lu.
#[async_trait]
pub trait TextVersionRepository: Send + Sync {
    /// Enregistre les versions, une ligne par référence. Une version déjà
    /// connue voit ses métadonnées mises à jour ; son état de lecture et ses
    /// articles ne changent jamais ici.
    async fn save_versions(&self, versions: &[TextVersion]) -> Result<(), RepositoryError>;

    /// Les versions d'un dossier, avec leur état de lecture.
    async fn versions_of(&self, dossier: &DossierUid) -> Result<Vec<TextVersion>, RepositoryError>;

    /// Les dossiers votés (RM-15), en une requête : un dossier est voté s'il
    /// porte un scrutin ou si un texte qu'il débat en porte.
    async fn voted_dossiers(&self) -> Result<Vec<BriefCandidate>, RepositoryError>;

    /// Les versions de tous les dossiers votés, en une requête.
    async fn versions_of_voted(&self) -> Result<Vec<TextVersion>, RepositoryError>;

    /// Les articles d'une version, dans l'ordre. `None` : texte jamais lu ou
    /// noté manquant.
    async fn articles_of(
        &self,
        document_uid: &DocumentUid,
    ) -> Result<Option<Vec<VersionArticle>>, RepositoryError>;

    /// Enregistre le résultat de la lecture d'une version : ses articles
    /// remplacés et son état de lecture changé, ensemble.
    async fn save_reading(
        &self,
        document_uid: &DocumentUid,
        reading: &TextReading,
    ) -> Result<(), RepositoryError>;
}
