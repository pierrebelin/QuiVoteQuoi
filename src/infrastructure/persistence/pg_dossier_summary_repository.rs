use async_trait::async_trait;
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::dossier_group_actions_repository::{
    DossierSummaryRepository, GeneratedGroupSummary, RepositoryError,
};
use crate::domain::actor::GroupUid;
use crate::domain::dossier_summary::{StoredGroupSummary, SummarySource, SummaryStatus};

pub struct PgDossierSummaryRepository {
    pool: PgPool,
}

impl PgDossierSummaryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DossierSummaryRepository for PgDossierSummaryRepository {
    async fn summaries_for(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<StoredGroupSummary>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT dossier_uid, group_uid, status, paragraph, facts_fingerprint,
                    model, prompt_version, generated_at
               FROM dossier_group_summaries
              WHERE dossier_uid = $1
              ORDER BY group_uid",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let sources = sqlx::query(
            "SELECT group_uid, source_id, source_kind, source_uid,
                    source_label, official_url
               FROM dossier_group_summary_sources
              WHERE dossier_uid = $1
              ORDER BY group_uid, ordinal",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut by_group: std::collections::HashMap<String, Vec<SummarySource>> =
            std::collections::HashMap::new();
        for row in sources {
            let source_uid: String = row.get("source_uid");
            let kind: String = row.get("source_kind");
            by_group
                .entry(row.get("group_uid"))
                .or_default()
                .push(SummarySource {
                    source_id: row.get("source_id"),
                    kind,
                    uid: source_uid,
                    label: row.get("source_label"),
                    official_url: row.get("official_url"),
                });
        }

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let raw_group_uid: String = row.get("group_uid");
                // Une synthese ecartee est simplement regeneree: elle n'est
                // jamais reputee a jour.
                let group_uid = match GroupUid::new(raw_group_uid.clone()) {
                    Ok(group_uid) => group_uid,
                    Err(error) => {
                        tracing::warn!(
                            "Skipping a stored summary with an unusable group uid: {error}"
                        );
                        return None;
                    }
                };
                Some(StoredGroupSummary {
                    group_uid,
                    status: match row.get::<String, _>("status").as_str() {
                        "ready" => SummaryStatus::Ready,
                        _ => SummaryStatus::Pending,
                    },
                    paragraph: row.get("paragraph"),
                    facts_fingerprint: row.get("facts_fingerprint"),
                    model: row.get("model"),
                    prompt_version: row.get("prompt_version"),
                    generated_at: row.get("generated_at"),
                    sources: by_group.remove(&raw_group_uid).unwrap_or_default(),
                })
            })
            .collect())
    }

    async fn mark_pending(
        &self,
        dossier_uid: &str,
        group_uids: &[String],
        facts_fingerprint: &str,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(db)?;
        for group_uid in group_uids {
            sqlx::query(
                "INSERT INTO dossier_group_summaries
                    (dossier_uid, group_uid, status, paragraph, facts_fingerprint,
                     model, prompt_version, generated_at, updated_at)
                 VALUES ($1, $2, 'pending', NULL, $3, NULL, NULL, NULL, NOW())
                 ON CONFLICT (dossier_uid, group_uid) DO UPDATE SET
                    status = 'pending', paragraph = NULL, facts_fingerprint = EXCLUDED.facts_fingerprint,
                    model = NULL, prompt_version = NULL, generated_at = NULL, updated_at = NOW()",
            )
            .bind(dossier_uid)
            .bind(group_uid)
            .bind(facts_fingerprint)
            .execute(&mut *transaction)
            .await
            .map_err(db)?;
        }
        transaction.commit().await.map_err(db)
    }

    async fn save_ready(
        &self,
        dossier_uid: &str,
        facts_fingerprint: &str,
        model: &str,
        prompt_version: &str,
        summaries: &[GeneratedGroupSummary],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(db)?;
        for summary in summaries {
            sqlx::query(
                "INSERT INTO dossier_group_summaries
                    (dossier_uid, group_uid, status, paragraph, facts_fingerprint,
                     model, prompt_version, generated_at, updated_at)
                 VALUES ($1, $2, 'ready', $3, $4, $5, $6, NOW(), NOW())
                 ON CONFLICT (dossier_uid, group_uid) DO UPDATE SET
                    status = 'ready', paragraph = EXCLUDED.paragraph,
                    facts_fingerprint = EXCLUDED.facts_fingerprint,
                    model = EXCLUDED.model, prompt_version = EXCLUDED.prompt_version,
                    generated_at = EXCLUDED.generated_at, updated_at = NOW()",
            )
            .bind(dossier_uid)
            .bind(&summary.group_uid)
            .bind(&summary.paragraph)
            .bind(facts_fingerprint)
            .bind(model)
            .bind(prompt_version)
            .execute(&mut *transaction)
            .await
            .map_err(db)?;

            sqlx::query(
                "DELETE FROM dossier_group_summary_sources
                  WHERE dossier_uid = $1 AND group_uid = $2",
            )
            .bind(dossier_uid)
            .bind(&summary.group_uid)
            .execute(&mut *transaction)
            .await
            .map_err(db)?;

            for (ordinal, source) in summary.sources.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO dossier_group_summary_sources
                        (dossier_uid, group_uid, ordinal, source_id, source_kind,
                         source_uid, source_label, official_url)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(dossier_uid)
                .bind(&summary.group_uid)
                .bind(ordinal as i16)
                .bind(&source.source_id)
                .bind(&source.kind)
                .bind(&source.uid)
                .bind(&source.label)
                .bind(&source.official_url)
                .execute(&mut *transaction)
                .await
                .map_err(db)?;
            }
        }
        transaction.commit().await.map_err(db)
    }
}
