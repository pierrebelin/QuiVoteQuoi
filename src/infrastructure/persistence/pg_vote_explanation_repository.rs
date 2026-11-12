//! Depot Postgres des explications de vote.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::db;
use crate::application::ports::vote_explanation_repository::{
    StoredSitting, StoredVersion, VoteExplanationRepository,
};
use crate::application::ports::RepositoryError;
use crate::domain::sitting_record::{SittingKeysFingerprint, SittingRecordUid};

pub struct PgVoteExplanationRepository {
    pool: PgPool,
}

impl PgVoteExplanationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn count(value: usize) -> Result<i32, RepositoryError> {
    i32::try_from(value).map_err(|e| RepositoryError::Database(e.to_string()))
}

#[async_trait]
impl VoteExplanationRepository for PgVoteExplanationRepository {
    async fn stored_versions(
        &self,
    ) -> Result<HashMap<SittingRecordUid, StoredVersion>, RepositoryError> {
        let rows: Vec<(String, DateTime<Utc>, String)> =
            sqlx::query_as("SELECT uid, generated_at, keys_fingerprint FROM sitting_records")
                .fetch_all(&self.pool)
                .await
                .map_err(db)?;
        rows.into_iter()
            .map(|(uid, generated_at, keys)| {
                let uid = SittingRecordUid::new(uid)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?;
                Ok((
                    uid,
                    StoredVersion {
                        generated_at,
                        keys: SittingKeysFingerprint::from_stored(keys),
                    },
                ))
            })
            .collect()
    }

    async fn replace_sitting(&self, sitting: &StoredSitting) -> Result<(), RepositoryError> {
        let record_uid = sitting.record_uid.as_str();
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("DELETE FROM sitting_records WHERE uid = $1")
            .bind(record_uid)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO sitting_records
                (uid, sitting_ref, sitting_date, version, generated_at, keys_fingerprint,
                 results_attached, results_unmatched, results_ambiguous,
                 blocks_unattached, headings_unrecognized)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(record_uid)
        .bind(sitting.sitting_ref.as_str())
        .bind(sitting.date)
        .bind(&sitting.version)
        .bind(sitting.generated_at)
        .bind(sitting.keys.as_str())
        .bind(count(sitting.counters.results_attached)?)
        .bind(count(sitting.counters.results_unmatched)?)
        .bind(count(sitting.counters.results_ambiguous)?)
        .bind(count(sitting.block_counters.blocks_unattached)?)
        .bind(count(sitting.block_counters.headings_unrecognized)?)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (scrutin_uid, has_block) in &sitting.attached {
            sqlx::query(
                "INSERT INTO sitting_record_scrutins (sitting_record_uid, scrutin_uid, has_block)
                 VALUES ($1, $2, $3)",
            )
            .bind(record_uid)
            .bind(scrutin_uid.as_str())
            .bind(has_block)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        for explanation in &sitting.explanations {
            sqlx::query(
                "INSERT INTO vote_explanations
                    (sitting_record_uid, scrutin_uid, rank, speaker_uid, speaker_name,
                     group_basis, group_uid, anchor, heading, text)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(record_uid)
            .bind(explanation.scrutin_uid().as_str())
            .bind(i32::from(explanation.rank()))
            .bind(explanation.speaker_uid().as_str())
            .bind(explanation.speaker_name())
            .bind(explanation.group().as_str())
            .bind(explanation.group().group_uid().map(|group| group.as_str()))
            .bind(explanation.anchor().as_str())
            .bind(explanation.heading())
            .bind(explanation.text())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)
    }
}
