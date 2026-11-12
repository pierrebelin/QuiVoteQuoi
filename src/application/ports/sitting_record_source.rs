use async_trait::async_trait;

use crate::domain::sitting_record::SittingRecord;

pub use super::SourceError;

/// Séances de l'archive, lues une à une: une séance illisible n'arrête pas les autres.
pub type SittingRecords = Box<dyn Iterator<Item = Result<SittingRecord, SourceError>> + Send>;

/// Source des comptes rendus de séance.
#[async_trait]
pub trait SittingRecordSource: Send + Sync {
    async fn fetch_sitting_records(&self, legislature: u16)
        -> Result<SittingRecords, SourceError>;
}
