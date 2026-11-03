use async_trait::async_trait;

use crate::domain::dossier::DossierUid;
use crate::domain::law_text::ArticleFingerprint;
use crate::domain::text_brief::{BriefProgress, StoredBriefLine};
use crate::domain::text_version::DocumentUid;

pub use super::RepositoryError;

/// Lignes « En bref » enregistrées, échecs du modèle par version et avancement
/// de la passe par dossier (SPEC-en-bref CU-02, RM-05, RM-16).
#[async_trait]
pub trait BriefLineRepository: Send + Sync {
    /// Les lignes que portent ces empreintes, en une requête. Une empreinte
    /// sans ligne est absente du résultat.
    async fn lines_for(
        &self,
        fingerprints: &[ArticleFingerprint],
    ) -> Result<Vec<StoredBriefLine>, RepositoryError>;

    /// Enregistre les lignes, une par empreinte. Une ligne refusée est
    /// remplacée ; une ligne publiable n'est jamais réécrite (RM-05, RM-17).
    async fn save_lines(&self, lines: &[StoredBriefLine]) -> Result<(), RepositoryError>;

    /// Passes consécutives où le modèle a échoué pour cette version.
    async fn failures_of(&self, document_uid: &DocumentUid) -> Result<u8, RepositoryError>;

    /// Compte un échec de plus pour la version, avec son message.
    async fn record_failure(
        &self,
        document_uid: &DocumentUid,
        error: &str,
    ) -> Result<(), RepositoryError>;

    /// Remet à zéro les échecs de la version.
    async fn clear_failures(&self, document_uid: &DocumentUid) -> Result<(), RepositoryError>;

    /// Enregistre l'avancement de la passe sur un dossier, en remplaçant le
    /// précédent.
    async fn save_progress(
        &self,
        dossier: &DossierUid,
        progress: &BriefProgress,
    ) -> Result<(), RepositoryError>;

    /// L'avancement de tous les dossiers déjà traités, en une requête.
    async fn progress(&self) -> Result<Vec<(DossierUid, BriefProgress)>, RepositoryError>;
}
