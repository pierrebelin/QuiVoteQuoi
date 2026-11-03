use async_trait::async_trait;

use super::RepositoryError;
use crate::domain::ingestion::{SourceCheck, SourceHistory};

#[async_trait]
pub trait IngestionJournal: Send + Sync {
    async fn record(&self, check: &SourceCheck) -> Result<(), RepositoryError>;

    /// Une entree par source ayant au moins une tentative, ordre libre.
    async fn histories(&self) -> Result<Vec<SourceHistory>, RepositoryError>;
}
