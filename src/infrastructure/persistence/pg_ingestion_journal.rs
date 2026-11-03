//! Depot Postgres du journal d'ingestion.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::application::ports::RepositoryError;
use crate::domain::ingestion::{CheckOutcome, SourceCheck, JOURNAL_RETENTION};

pub struct PgIngestionJournal {
    pool: PgPool,
}

impl PgIngestionJournal {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn invalid<E: std::fmt::Display>(error: E) -> RepositoryError {
    RepositoryError::Database(error.to_string())
}

#[async_trait]
impl IngestionJournal for PgIngestionJournal {
    /// Insere la ligne puis purge les lignes de la meme source plus anciennes
    /// que la retention (RM-F8), sauf la plus recente reussite.
    async fn record(&self, check: &SourceCheck) -> Result<(), RepositoryError> {
        let (succeeded, items_read, reason) = match check.outcome() {
            CheckOutcome::Succeeded { items_read } => (true, Some(*items_read as i64), None),
            CheckOutcome::Failed { reason } => (false, None, Some(reason.as_str())),
        };
        let source = check.source().as_str();

        let mut tx = self.pool.begin().await.map_err(invalid)?;
        sqlx::query(
            "INSERT INTO ingestion_checks (source, attempted_at, succeeded, items_read, failure_reason)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(source)
        .bind(check.attempted_at())
        .bind(succeeded)
        .bind(items_read)
        .bind(reason)
        .execute(&mut *tx)
        .await
        .map_err(invalid)?;
        sqlx::query(
            "DELETE FROM ingestion_checks
             WHERE source = $1 AND attempted_at < $2
               AND id IS DISTINCT FROM (
                   SELECT id FROM ingestion_checks
                   WHERE source = $1 AND succeeded
                   ORDER BY attempted_at DESC, id DESC LIMIT 1)",
        )
        .bind(source)
        .bind(check.attempted_at() - JOURNAL_RETENTION)
        .execute(&mut *tx)
        .await
        .map_err(invalid)?;
        tx.commit().await.map_err(invalid)
    }
}
