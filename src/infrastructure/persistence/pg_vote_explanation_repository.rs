//! Depot Postgres des explications de vote.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::db;
use crate::application::ports::vote_explanation_repository::{
    ScrutinExplanationsRead, StoredReport, StoredSitting, StoredVersion, VoteExplanationRepository,
};
use crate::application::ports::RepositoryError;
use crate::domain::actor::{ActorUid, GroupUid};
use crate::domain::scrutin::ScrutinUid;
use crate::domain::sitting_record::{
    SittingKeysFingerprint, SittingRecordUid, SittingRef, SpeechAnchor,
};
use crate::domain::vote_explanation::{NewVoteExplanation, SpeakerGroup, VoteExplanation};

pub struct PgVoteExplanationRepository {
    pool: PgPool,
}

impl PgVoteExplanationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Ligne de lecture: séance, rattachement du scrutin, puis une explication éventuelle.
type ExplanationRow = (
    String,
    String,
    Option<bool>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn corrupt(error: impl ToString) -> RepositoryError {
    RepositoryError::Database(error.to_string())
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

    async fn explanations_for(
        &self,
        scrutin: &ScrutinUid,
        sitting_ref: &SittingRef,
    ) -> Result<ScrutinExplanationsRead, RepositoryError> {
        let rows: Vec<ExplanationRow> = sqlx::query_as(
            "SELECT r.uid, r.version, s.has_block, e.rank, e.speaker_uid, e.speaker_name,
                    e.group_basis, e.group_uid, e.anchor, e.heading, e.text
             FROM sitting_records r
             LEFT JOIN sitting_record_scrutins s
                    ON s.sitting_record_uid = r.uid AND s.scrutin_uid = $1
             LEFT JOIN vote_explanations e
                    ON e.sitting_record_uid = r.uid AND e.scrutin_uid = $1
             WHERE r.sitting_ref = $2
             ORDER BY r.generated_at DESC, r.uid, e.rank",
        )
        .bind(scrutin.as_str())
        .bind(sitting_ref.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let Some(first) = rows.first() else {
            return Ok(ScrutinExplanationsRead::default());
        };
        let record_uid = SittingRecordUid::new(first.0.clone()).map_err(corrupt)?;
        let report = StoredReport {
            record_uid: record_uid.clone(),
            version: first.1.clone(),
        };
        let attached = first.2;
        let mut explanations = Vec::new();
        for row in rows.into_iter().filter(|row| row.0 == record_uid.as_str()) {
            if let (Some(rank), Some(speaker), Some(basis), Some(anchor), Some(heading), Some(text)) =
                (row.3, row.4, row.6, row.8, row.9, row.10)
            {
                let group_uid = row.7.map(GroupUid::new).transpose().map_err(corrupt)?;
                let group = SpeakerGroup::parse(&basis, group_uid)
                    .ok_or_else(|| corrupt("unknown speaker group basis"))?;
                explanations.push(VoteExplanation::new(NewVoteExplanation {
                    scrutin_uid: scrutin.clone(),
                    sitting_record_uid: record_uid.clone(),
                    rank: u16::try_from(rank).map_err(corrupt)?,
                    speaker_uid: ActorUid::new(speaker).map_err(corrupt)?,
                    speaker_name: row.5,
                    group,
                    anchor: SpeechAnchor::new(anchor).map_err(corrupt)?,
                    heading,
                    text,
                }));
            }
        }
        Ok(ScrutinExplanationsRead {
            report: Some(report),
            attached,
            explanations,
        })
    }
}
