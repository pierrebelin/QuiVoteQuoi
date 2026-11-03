use async_trait::async_trait;

use super::RepositoryError;
use crate::domain::ingestion::SourceCheck;

#[async_trait]
pub trait IngestionJournal: Send + Sync {
    async fn record(&self, check: &SourceCheck) -> Result<(), RepositoryError>;
}
