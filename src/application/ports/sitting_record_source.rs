use async_trait::async_trait;

use crate::domain::sitting_record::{SittingRecord, SittingRecordUid};

pub use super::SourceError;

/// Séances de l'archive, lues une à une: une séance illisible n'arrête pas les autres.
pub type SittingRecords = Box<dyn Iterator<Item = Result<SittingRecord, SourceError>> + Send>;

/// Source des comptes rendus de séance.
#[async_trait]
pub trait SittingRecordSource: Send + Sync {
    async fn fetch_sitting_records(&self, legislature: u16) -> Result<SittingRecords, SourceError>;

    /// Un compte rendu lu à part, dans sa dernière version publiée: l'archive
    /// n'est republiée qu'une fois par jour, ce fichier suit le compte rendu
    /// provisoire à mesure qu'il se complète. Un compte rendu introuvable est
    /// une erreur: on ne le demande que s'il a déjà été lu dans l'archive.
    async fn fetch_sitting_record(
        &self,
        uid: &SittingRecordUid,
    ) -> Result<SittingRecord, SourceError>;
}
