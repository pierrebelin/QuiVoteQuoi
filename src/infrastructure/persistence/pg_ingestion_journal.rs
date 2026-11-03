//! Depot Postgres du journal d'ingestion.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::application::ports::RepositoryError;
use chrono::{DateTime, Utc};

use crate::domain::ingestion::{
    CheckOutcome, IngestionSource, LastAttempt, SourceCheck, SourceHistory, JOURNAL_RETENTION,
};

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

    async fn histories(&self) -> Result<Vec<SourceHistory>, RepositoryError> {
        let rows: Vec<HistoryRow> = sqlx::query_as(
            "SELECT c.source,
                    l.attempted_at AS last_attempt_at,
                    l.succeeded AS last_attempt_succeeded,
                    s.last_success_at,
                    (SELECT COUNT(*) FROM ingestion_checks f
                     WHERE f.source = c.source AND NOT f.succeeded
                       AND (s.last_success_at IS NULL OR f.attempted_at > s.last_success_at)
                    ) AS failures
             FROM (SELECT DISTINCT source FROM ingestion_checks) c
             CROSS JOIN LATERAL (
                 SELECT attempted_at, succeeded FROM ingestion_checks
                 WHERE source = c.source
                 ORDER BY attempted_at DESC, id DESC LIMIT 1) l
             CROSS JOIN LATERAL (
                 SELECT MAX(attempted_at) AS last_success_at FROM ingestion_checks
                 WHERE source = c.source AND succeeded) s",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(invalid)?;
        rows.into_iter()
            .map(|row| {
                history_from_row(
                    &row.source,
                    row.last_attempt_at,
                    row.last_attempt_succeeded,
                    row.last_success_at,
                    row.failures,
                )
            })
            .collect()
    }
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    source: String,
    last_attempt_at: DateTime<Utc>,
    last_attempt_succeeded: bool,
    last_success_at: Option<DateTime<Utc>>,
    failures: i64,
}

/// Ligne agregee du journal -> historique d'une source.
fn history_from_row(
    source_code: &str,
    last_attempt_at: DateTime<Utc>,
    last_attempt_succeeded: bool,
    last_success_at: Option<DateTime<Utc>>,
    failures: i64,
) -> Result<SourceHistory, RepositoryError> {
    let source = IngestionSource::parse(source_code)
        .ok_or_else(|| invalid(format!("unknown ingestion source {source_code}")))?;
    let failures = u32::try_from(failures).map_err(invalid)?;
    SourceHistory::new(
        source,
        LastAttempt::new(last_attempt_at, last_attempt_succeeded),
        last_success_at,
        failures,
    )
    .map_err(invalid)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, hour, 0, 0).unwrap()
    }

    fn message(error: RepositoryError) -> String {
        let RepositoryError::Database(message) = error;
        message
    }

    #[test]
    fn should_map_row_to_history_when_source_code_is_known() {
        let history = history_from_row("scrutins", at(12), false, Some(at(10)), 2).unwrap();

        assert_eq!(history.source(), IngestionSource::Scrutins);
        assert_eq!(history.last_success_at(), Some(at(10)));
        assert_eq!(history.last_attempt(), Some(&LastAttempt::new(at(12), false)));
        assert_eq!(history.failures_since_last_success(), 2);
    }

    #[test]
    fn should_refuse_row_when_source_code_is_unknown() {
        let error = history_from_row("senate", at(12), true, Some(at(12)), 0).unwrap_err();

        assert!(message(error).starts_with("unknown ingestion source"));
    }

    #[test]
    fn should_refuse_row_when_history_is_inconsistent() {
        let error = history_from_row("scrutins", at(10), true, Some(at(12)), 0).unwrap_err();

        assert_eq!(message(error), "last success follows last attempt");
    }
}
