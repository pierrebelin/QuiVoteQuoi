//! Dépôt Postgres des lignes « En bref », des échecs du modèle par version et de
//! l'avancement de la passe par dossier (SPEC-en-bref RM-05, RM-16, CU-02).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sqlx::{PgPool, Row};

use super::{db, invalid_row};
use crate::application::ports::brief_line_repository::BriefLineRepository;
use crate::application::ports::RepositoryError;
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::ArticleFingerprint;
use crate::domain::text_brief::{BriefProgress, BriefStatus, StoredBriefLine};
use crate::domain::text_version::DocumentUid;

/// Lignes par instruction `UNNEST`.
const ROW_BATCH: usize = 1000;

pub struct PgBriefLineRepository {
    pool: PgPool,
}

impl PgBriefLineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BriefLineRepository for PgBriefLineRepository {
    async fn lines_for(
        &self,
        fingerprints: &[ArticleFingerprint],
    ) -> Result<Vec<StoredBriefLine>, RepositoryError> {
        let keys: Vec<&str> = fingerprints
            .iter()
            .map(ArticleFingerprint::as_str)
            .collect();
        let rows = sqlx::query(
            "SELECT fingerprint, status, line, rejection, rejection_detail, rejected_line,
                    model, prompt_version, generated_at, cited_sources::text AS cited_sources
               FROM brief_lines
              WHERE fingerprint = ANY($1)",
        )
        .bind(&keys)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.iter()
            .map(|row| {
                let status: String = row.get("status");
                let cited_sources: String = row.get("cited_sources");
                Ok(StoredBriefLine {
                    fingerprint: ArticleFingerprint::new(row.get("fingerprint")),
                    status: BriefStatus::parse(&status)
                        .ok_or_else(|| invalid_row(format!("brief status {status}")))?,
                    line: row.get("line"),
                    rejection: row.get("rejection"),
                    rejection_detail: row.get("rejection_detail"),
                    rejected_line: row.get("rejected_line"),
                    model: row.get("model"),
                    prompt_version: row.get("prompt_version"),
                    generated_at: row.get::<DateTime<Utc>, _>("generated_at").naive_utc(),
                    cited_sources: serde_json::from_str(&cited_sources)
                        .map_err(|error| invalid_row(format!("cited sources: {error}")))?,
                })
            })
            .collect()
    }

    async fn save_lines(&self, lines: &[StoredBriefLine]) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(db)?;
        for batch in lines.chunks(ROW_BATCH) {
            let fingerprints: Vec<&str> = batch.iter().map(|l| l.fingerprint.as_str()).collect();
            let statuses: Vec<&str> = batch.iter().map(|l| l.status.as_str()).collect();
            let texts: Vec<Option<&str>> = batch.iter().map(|l| l.line.as_deref()).collect();
            let rejections: Vec<Option<&str>> =
                batch.iter().map(|l| l.rejection.as_deref()).collect();
            let details: Vec<Option<&str>> =
                batch.iter().map(|l| l.rejection_detail.as_deref()).collect();
            let rejected: Vec<Option<&str>> =
                batch.iter().map(|l| l.rejected_line.as_deref()).collect();
            let models: Vec<&str> = batch.iter().map(|l| l.model.as_str()).collect();
            let prompts: Vec<&str> = batch.iter().map(|l| l.prompt_version.as_str()).collect();
            let generated: Vec<DateTime<Utc>> = batch
                .iter()
                .map(|l| Utc.from_utc_datetime(&l.generated_at))
                .collect();
            let cited: Vec<String> = batch
                .iter()
                .map(|l| {
                    serde_json::to_string(&l.cited_sources)
                        .map_err(|error| invalid_row(format!("cited sources: {error}")))
                })
                .collect::<Result<_, _>>()?;
            // Une ligne publiable n'est jamais réécrite (RM-05, RM-17): seul un
            // refus est remplacé. Une empreinte par instruction.
            sqlx::query(
                "INSERT INTO brief_lines
                    (fingerprint, status, line, rejection, model, prompt_version,
                     generated_at, cited_sources, rejection_detail, rejected_line)
                 SELECT fingerprint, status, line, rejection, model, prompt_version,
                        generated_at, cited_sources::jsonb, rejection_detail, rejected_line
                   FROM UNNEST(
                        $1::text[], $2::text[], $3::text[], $4::text[], $5::text[],
                        $6::text[], $7::timestamptz[], $8::text[], $9::text[], $10::text[])
                        AS t(fingerprint, status, line, rejection, model, prompt_version,
                             generated_at, cited_sources, rejection_detail, rejected_line)
                 ON CONFLICT (fingerprint) DO UPDATE SET
                    status = EXCLUDED.status,
                    line = EXCLUDED.line,
                    rejection = EXCLUDED.rejection,
                    model = EXCLUDED.model,
                    prompt_version = EXCLUDED.prompt_version,
                    generated_at = EXCLUDED.generated_at,
                    cited_sources = EXCLUDED.cited_sources,
                    rejection_detail = EXCLUDED.rejection_detail,
                    rejected_line = EXCLUDED.rejected_line
                  WHERE brief_lines.status <> 'ready'",
            )
            .bind(&fingerprints)
            .bind(&statuses)
            .bind(&texts)
            .bind(&rejections)
            .bind(&models)
            .bind(&prompts)
            .bind(&generated)
            .bind(&cited)
            .bind(&details)
            .bind(&rejected)
            .execute(&mut *transaction)
            .await
            .map_err(db)?;
        }
        transaction.commit().await.map_err(db)
    }

    async fn failures_of(&self, document_uid: &DocumentUid) -> Result<u8, RepositoryError> {
        let failures: Option<i16> = sqlx::query_scalar(
            "SELECT failures FROM text_version_brief_attempts WHERE document_uid = $1",
        )
        .bind(document_uid.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        Ok(failures.map_or(0, |count| u8::try_from(count).unwrap_or(u8::MAX)))
    }

    async fn record_failure(
        &self,
        document_uid: &DocumentUid,
        error: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO text_version_brief_attempts (document_uid, failures, last_error)
             VALUES ($1, 1, $2)
             ON CONFLICT (document_uid) DO UPDATE SET
                failures = text_version_brief_attempts.failures + 1,
                last_error = EXCLUDED.last_error,
                updated_at = NOW()",
        )
        .bind(document_uid.as_str())
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(db)?;
        Ok(())
    }

    async fn clear_failures(&self, document_uid: &DocumentUid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM text_version_brief_attempts WHERE document_uid = $1")
            .bind(document_uid.as_str())
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(())
    }

    async fn save_progress(
        &self,
        dossier: &DossierUid,
        progress: &BriefProgress,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO dossier_brief_progress
                (dossier_uid, document_uid, prompt_version, pending_count,
                 rejected_count, attempted_on)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (dossier_uid) DO UPDATE SET
                document_uid = EXCLUDED.document_uid,
                prompt_version = EXCLUDED.prompt_version,
                pending_count = EXCLUDED.pending_count,
                rejected_count = EXCLUDED.rejected_count,
                attempted_on = EXCLUDED.attempted_on",
        )
        .bind(dossier.as_str())
        .bind(progress.document_uid.as_str())
        .bind(&progress.prompt_version)
        .bind(i32::try_from(progress.pending_count).unwrap_or(i32::MAX))
        .bind(i32::try_from(progress.rejected_count).unwrap_or(i32::MAX))
        .bind(progress.attempted_on)
        .execute(&self.pool)
        .await
        .map_err(db)?;
        Ok(())
    }

    async fn progress(&self) -> Result<Vec<(DossierUid, BriefProgress)>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT dossier_uid, document_uid, prompt_version, pending_count,
                    rejected_count, attempted_on
               FROM dossier_brief_progress",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.iter()
            .map(|row| {
                let pending: i32 = row.get("pending_count");
                let rejected: i32 = row.get("rejected_count");
                Ok((
                    DossierUid::new(row.get("dossier_uid")).map_err(invalid_row)?,
                    BriefProgress {
                        document_uid: DocumentUid::new(row.get("document_uid"))
                            .map_err(invalid_row)?,
                        prompt_version: row.get("prompt_version"),
                        pending_count: usize::try_from(pending)
                            .map_err(|_| invalid_row(format!("pending count {pending}")))?,
                        rejected_count: usize::try_from(rejected)
                            .map_err(|_| invalid_row(format!("rejected count {rejected}")))?,
                        attempted_on: row.get::<NaiveDate, _>("attempted_on"),
                    },
                ))
            })
            .collect()
    }
}
