//! Synthèses de groupe d'un dossier : lecture, mise en attente, écriture.

use async_trait::async_trait;

use crate::domain::dossier_summary::{StoredGroupSummary, SummarySource};

pub use super::RepositoryError;

#[async_trait]
pub trait DossierSummaryRepository: Send + Sync {
    async fn summaries_for(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<StoredGroupSummary>, RepositoryError>;
    async fn mark_pending(
        &self,
        dossier_uid: &str,
        group_uids: &[String],
        facts_fingerprint: &str,
    ) -> Result<(), RepositoryError>;
    async fn save_ready(
        &self,
        dossier_uid: &str,
        facts_fingerprint: &str,
        model: &str,
        prompt_version: &str,
        summaries: &[GeneratedGroupSummary],
    ) -> Result<(), RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct GeneratedGroupSummary {
    pub group_uid: String,
    pub paragraph: String,
    pub sources: Vec<SummarySource>,
}
