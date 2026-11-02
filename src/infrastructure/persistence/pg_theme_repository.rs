use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::theme_repository::{
    AssignedFamily, AttemptOutcome, FamilyCoverage, MethodReport, PendingDossier, RepositoryError,
    ScrutinSubject, TextLink, TextPage, TextScrutin, TextSummary, ThemeRepository,
};
use crate::domain::dossier::DossierUid;
use crate::domain::theme::{
    DebatedText, FamilyCode, ProposedFamily, SubjectRef, TextKey, ThemeAssignment, ThemeProposal,
};

/// Lignes par instruction `UNNEST`. 8 434 scrutins passent en trois lots.
const ROW_BATCH: usize = 4000;

pub struct PgThemeRepository {
    pool: PgPool,
}

impl PgThemeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Colonnes d'une ligne de liste de texte. Les agregats viennent de la base,
/// jamais du modele (RM-10).
const TEXT_SUMMARY_COLUMNS: &str = "
    SELECT t.text_key, t.label, t.last_attempt_outcome,
           coalesce(agg.scrutin_count, 0) AS scrutin_count,
           agg.first_vote, agg.last_vote,
           d.dossier_uid, d.dossier_label
    FROM debated_texts t
    LEFT JOIN LATERAL (
        SELECT count(*) AS scrutin_count,
               min(s.scrutin_date) AS first_vote,
               max(s.scrutin_date) AS last_vote
        FROM scrutin_debated_texts l
        JOIN scrutins s ON s.uid = l.scrutin_uid
        WHERE l.text_key = t.text_key
    ) agg ON true
    LEFT JOIN LATERAL (
        SELECT dt.dossier_uid, ld.title AS dossier_label
        FROM dossier_debated_texts dt
        LEFT JOIN legislative_dossiers ld ON ld.uid = dt.dossier_uid
        WHERE dt.text_key = t.text_key
        ORDER BY dt.scrutin_count DESC, dt.dossier_uid
        LIMIT 1
    ) d ON true
";

fn summary_from_row(row: &sqlx::postgres::PgRow) -> TextSummary {
    TextSummary {
        key: row.get("text_key"),
        label: row.get("label"),
        scrutin_count: row.get("scrutin_count"),
        first_vote: row.get("first_vote"),
        last_vote: row.get("last_vote"),
        dossier_uid: row.get("dossier_uid"),
        dossier_label: row.get("dossier_label"),
        families: vec![],
        last_attempt_outcome: row.get("last_attempt_outcome"),
    }
}

fn assigned_from_row(row: &sqlx::postgres::PgRow) -> Option<AssignedFamily> {
    let code: String = row.get("family_code");
    Some(AssignedFamily {
        family: FamilyCode::parse(&code).ok()?,
        opened_on: row.get("opened_on"),
        motive: row.get("motive"),
    })
}

impl PgThemeRepository {
    /// Attache leurs familles courantes aux lignes de liste, en une requete.
    async fn attach_families(&self, items: &mut [TextSummary]) -> Result<(), RepositoryError> {
        if items.is_empty() {
            return Ok(());
        }
        let keys: Vec<String> = items.iter().map(|i| i.key.clone()).collect();
        let rows = sqlx::query(
            "SELECT subject_id, family_code, opened_on, motive
             FROM theme_assignments
             WHERE subject_kind = 'text' AND closed_on IS NULL AND subject_id = ANY($1)
             ORDER BY opened_on, id",
        )
        .bind(&keys)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut by_key: HashMap<String, Vec<AssignedFamily>> = HashMap::new();
        for row in &rows {
            if let Some(assigned) = assigned_from_row(row) {
                by_key
                    .entry(row.get("subject_id"))
                    .or_default()
                    .push(assigned);
            }
        }
        for item in items.iter_mut() {
            item.families = by_key.remove(&item.key).unwrap_or_default();
        }
        Ok(())
    }

    async fn count(&self, sql: &str) -> Result<i64, RepositoryError> {
        sqlx::query(sql)
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?
            .try_get(0)
            .map_err(db)
    }
}

#[async_trait]
impl ThemeRepository for PgThemeRepository {
    async fn scrutin_subjects(&self) -> Result<Vec<ScrutinSubject>, RepositoryError> {
        let rows = sqlx::query("SELECT uid, subject FROM scrutins ORDER BY uid")
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        Ok(rows
            .iter()
            .map(|row| ScrutinSubject {
                uid: row.get("uid"),
                subject: row.get("subject"),
            })
            .collect())
    }

    async fn save_texts(&self, texts: &[DebatedText]) -> Result<usize, RepositoryError> {
        let mut written = 0usize;
        for batch in texts.chunks(ROW_BATCH) {
            let keys: Vec<&str> = batch.iter().map(|t| t.key().as_str()).collect();
            let labels: Vec<&str> = batch.iter().map(|t| t.label()).collect();
            sqlx::query(
                "INSERT INTO debated_texts (text_key, label)
                 SELECT * FROM UNNEST($1::text[], $2::text[])
                 ON CONFLICT (text_key) DO UPDATE
                 SET label = EXCLUDED.label, updated_at = NOW()",
            )
            .bind(&keys)
            .bind(&labels)
            .execute(&self.pool)
            .await
            .map_err(db)?;
            written += batch.len();
        }
        Ok(written)
    }

    async fn link_scrutins(&self, links: &[TextLink]) -> Result<usize, RepositoryError> {
        let mut written = 0usize;
        for batch in links.chunks(ROW_BATCH) {
            let uids: Vec<&str> = batch.iter().map(|l| l.scrutin_uid.as_str()).collect();
            let keys: Vec<&str> = batch.iter().map(|l| l.text_key.as_str()).collect();
            sqlx::query(
                "INSERT INTO scrutin_debated_texts (scrutin_uid, text_key)
                 SELECT * FROM UNNEST($1::text[], $2::text[])
                 ON CONFLICT (scrutin_uid) DO UPDATE SET text_key = EXCLUDED.text_key",
            )
            .bind(&uids)
            .bind(&keys)
            .execute(&self.pool)
            .await
            .map_err(db)?;
            written += batch.len();
        }
        Ok(written)
    }

    async fn link_dossiers_through_scrutins(&self) -> Result<usize, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("DELETE FROM dossier_debated_texts")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let inserted = sqlx::query(
            "INSERT INTO dossier_debated_texts (dossier_uid, text_key, scrutin_count)
             SELECT s.dossier_uid, l.text_key, count(*)
             FROM scrutins s
             JOIN scrutin_debated_texts l ON l.scrutin_uid = s.uid
             WHERE s.dossier_uid IS NOT NULL
             GROUP BY s.dossier_uid, l.text_key",
        )
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        tx.commit().await.map_err(db)?;
        Ok(inserted as usize)
    }

    async fn texts_awaiting_proposal(
        &self,
        limit: i64,
    ) -> Result<Vec<DebatedText>, RepositoryError> {
        // Du plus vote au moins vote: l'arbitrage et la couverture portent
        // d'abord sur les textes qui pesent le plus de scrutins.
        let rows = sqlx::query(
            "SELECT t.text_key, t.label,
                    (SELECT count(*) FROM scrutin_debated_texts l WHERE l.text_key = t.text_key) AS weight
             FROM debated_texts t
             WHERE NOT EXISTS (
                 SELECT 1 FROM theme_assignments a
                 WHERE a.subject_kind = 'text' AND a.subject_id = t.text_key AND a.closed_on IS NULL
             )
             AND (t.last_attempt_outcome IS NULL OR t.last_attempt_outcome = 'failed')
             ORDER BY weight DESC, t.text_key
             LIMIT $1",
        )
        .bind(limit)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .filter_map(|row| DebatedText::new(row.get::<String, _>("label")).ok())
            .collect())
    }

    async fn dossiers_awaiting_proposal(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingDossier>, RepositoryError> {
        // Un dossier relie a un texte herite de ses familles (RM-06): le
        // soumettre en plus serait payer deux fois le meme rattachement.
        let rows = sqlx::query(
            "SELECT d.uid, d.title
             FROM legislative_dossiers d
             LEFT JOIN dossier_theme_attempts a ON a.dossier_uid = d.uid
             WHERE NOT EXISTS (
                 SELECT 1 FROM dossier_debated_texts l WHERE l.dossier_uid = d.uid
             )
             AND NOT EXISTS (
                 SELECT 1 FROM theme_assignments t
                 WHERE t.subject_kind = 'dossier' AND t.subject_id = d.uid
                   AND t.closed_on IS NULL
             )
             AND (a.last_attempt_outcome IS NULL OR a.last_attempt_outcome = 'failed')
             ORDER BY d.uid
             LIMIT $1",
        )
        .bind(limit)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                Some(PendingDossier {
                    uid: DossierUid::new(row.get("uid")).ok()?,
                    title: row.get("title"),
                })
            })
            .collect())
    }

    async fn record_attempt(
        &self,
        subject: &SubjectRef,
        on: NaiveDate,
        outcome: AttemptOutcome,
    ) -> Result<(), RepositoryError> {
        match subject {
            SubjectRef::Text(key) => {
                sqlx::query(
                    "UPDATE debated_texts SET last_attempt_on = $2, last_attempt_outcome = $3,
                            updated_at = NOW()
                     WHERE text_key = $1",
                )
                .bind(key.as_str())
                .bind(on)
                .bind(outcome.as_str())
                .execute(&self.pool)
                .await
                .map_err(db)?;
            }
            SubjectRef::Dossier(uid) => {
                sqlx::query(
                    "INSERT INTO dossier_theme_attempts
                         (dossier_uid, last_attempt_on, last_attempt_outcome)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (dossier_uid) DO UPDATE
                         SET last_attempt_on = EXCLUDED.last_attempt_on,
                             last_attempt_outcome = EXCLUDED.last_attempt_outcome",
                )
                .bind(uid.as_str())
                .bind(on)
                .bind(outcome.as_str())
                .execute(&self.pool)
                .await
                .map_err(db)?;
            }
        }
        Ok(())
    }

    async fn save_proposal(&self, proposal: &ThemeProposal) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let id: i64 = sqlx::query(
            "INSERT INTO theme_proposals (subject_kind, subject_id, model, prompt_version, produced_on)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(proposal.subject().kind())
        .bind(proposal.subject().identifier())
        .bind(proposal.model())
        .bind(proposal.prompt_version())
        .bind(proposal.produced_on())
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?
        .get(0);

        for (ordinal, family) in proposal.families().iter().enumerate() {
            sqlx::query(
                "INSERT INTO theme_proposal_families (proposal_id, family_code, ordinal, justification)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(id)
            .bind(family.family().as_str())
            .bind(ordinal as i16)
            .bind(family.justification())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn latest_proposal(
        &self,
        subject: &SubjectRef,
    ) -> Result<Option<ThemeProposal>, RepositoryError> {
        let Some(head) = sqlx::query(
            "SELECT id, model, prompt_version, produced_on
             FROM theme_proposals
             WHERE subject_kind = $1 AND subject_id = $2
             ORDER BY id DESC LIMIT 1",
        )
        .bind(subject.kind())
        .bind(subject.identifier())
        .persistent(false)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };

        let id: i64 = head.get("id");
        let rows = sqlx::query(
            "SELECT family_code, justification FROM theme_proposal_families
             WHERE proposal_id = $1 ORDER BY ordinal",
        )
        .bind(id)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let families: Vec<ProposedFamily> = rows
            .iter()
            .filter_map(|row| {
                let family = FamilyCode::parse(&row.get::<String, _>("family_code")).ok()?;
                ProposedFamily::new(family, row.get("justification")).ok()
            })
            .collect();

        Ok(ThemeProposal::new(
            subject.clone(),
            families,
            head.get("model"),
            head.get("prompt_version"),
            head.get("produced_on"),
        )
        .ok())
    }

    async fn replace_assignments(
        &self,
        subject: &SubjectRef,
        closed_on: NaiveDate,
        opened: &[ThemeAssignment],
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;

        // `GREATEST` protege la contrainte de dates quand un rattachement a ete
        // ouvert le jour meme: on ne clot jamais avant l'ouverture (RM-07).
        sqlx::query(
            "UPDATE theme_assignments SET closed_on = GREATEST($3, opened_on)
             WHERE subject_kind = $1 AND subject_id = $2 AND closed_on IS NULL",
        )
        .bind(subject.kind())
        .bind(subject.identifier())
        .bind(closed_on)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        for assignment in opened {
            sqlx::query(
                "INSERT INTO theme_assignments
                 (subject_kind, subject_id, family_code, opened_on, author, motive)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(assignment.subject().kind())
            .bind(assignment.subject().identifier())
            .bind(assignment.family().as_str())
            .bind(assignment.opened_on())
            .bind(assignment.author())
            .bind(assignment.motive())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn assignment_history(
        &self,
        subject: &SubjectRef,
    ) -> Result<Vec<ThemeAssignment>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT family_code, opened_on, closed_on, author, motive
             FROM theme_assignments
             WHERE subject_kind = $1 AND subject_id = $2
             ORDER BY opened_on DESC, id DESC",
        )
        .bind(subject.kind())
        .bind(subject.identifier())
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut history = Vec::with_capacity(rows.len());
        for row in &rows {
            let Ok(family) = FamilyCode::parse(&row.get::<String, _>("family_code")) else {
                continue;
            };
            let Ok(mut assignment) = ThemeAssignment::open(
                subject.clone(),
                family,
                row.get("opened_on"),
                row.get("author"),
                row.get("motive"),
            ) else {
                continue;
            };
            if let Some(closed_on) = row.get::<Option<NaiveDate>, _>("closed_on") {
                assignment
                    .close(closed_on)
                    .map_err(|e| RepositoryError::Database(e.to_string()))?;
            }
            history.push(assignment);
        }
        Ok(history)
    }

    async fn text_by_key(&self, key: &TextKey) -> Result<Option<TextSummary>, RepositoryError> {
        let sql = format!("{TEXT_SUMMARY_COLUMNS} WHERE t.text_key = $1");
        let Some(row) = sqlx::query(&sql)
            .bind(key.as_str())
            .persistent(false)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?
        else {
            return Ok(None);
        };
        let mut items = vec![summary_from_row(&row)];
        self.attach_families(&mut items).await?;
        Ok(items.pop())
    }

    async fn scrutins_of_text(
        &self,
        key: &TextKey,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TextScrutin>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT s.uid, s.number, s.scrutin_date, s.subject, s.outcome_label,
                    s.votes_for, s.votes_against, s.abstentions
             FROM scrutin_debated_texts l
             JOIN scrutins s ON s.uid = l.scrutin_uid
             WHERE l.text_key = $1
             ORDER BY s.scrutin_date DESC, s.uid DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(key.as_str())
        .bind(limit)
        .bind(offset)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| TextScrutin {
                uid: row.get("uid"),
                number: row.get("number"),
                date: row.get("scrutin_date"),
                subject: row.get("subject"),
                outcome_label: row.get("outcome_label"),
                votes_for: row.get("votes_for"),
                votes_against: row.get("votes_against"),
                abstentions: row.get("abstentions"),
            })
            .collect())
    }

    async fn texts_by_family(
        &self,
        family: FamilyCode,
        limit: i64,
        offset: i64,
    ) -> Result<TextPage, RepositoryError> {
        let condition = "EXISTS (
            SELECT 1 FROM theme_assignments a
            WHERE a.subject_kind = 'text' AND a.subject_id = t.text_key
              AND a.closed_on IS NULL AND a.family_code = $1
        )";

        let total: i64 = sqlx::query(&format!(
            "SELECT count(*) FROM debated_texts t WHERE {condition}"
        ))
        .bind(family.as_str())
        .persistent(false)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?
        .get(0);

        let rows = sqlx::query(&format!(
            "{TEXT_SUMMARY_COLUMNS} WHERE {condition}
             ORDER BY agg.last_vote DESC NULLS LAST, t.text_key
             LIMIT $2 OFFSET $3"
        ))
        .bind(family.as_str())
        .bind(limit)
        .bind(offset)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut items: Vec<TextSummary> = rows.iter().map(summary_from_row).collect();
        self.attach_families(&mut items).await?;
        Ok(TextPage { items, total })
    }

    async fn unassigned_texts(&self, limit: i64, offset: i64) -> Result<TextPage, RepositoryError> {
        let condition = "NOT EXISTS (
            SELECT 1 FROM theme_assignments a
            WHERE a.subject_kind = 'text' AND a.subject_id = t.text_key AND a.closed_on IS NULL
        )";

        let total: i64 = sqlx::query(&format!(
            "SELECT count(*) FROM debated_texts t WHERE {condition}"
        ))
        .persistent(false)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?
        .get(0);

        let rows = sqlx::query(&format!(
            "{TEXT_SUMMARY_COLUMNS} WHERE {condition}
             ORDER BY coalesce(agg.scrutin_count, 0) DESC, t.text_key
             LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(TextPage {
            items: rows.iter().map(summary_from_row).collect(),
            total,
        })
    }

    async fn families_of_scrutins(
        &self,
        scrutin_uids: &[String],
    ) -> Result<HashMap<String, Vec<AssignedFamily>>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT l.scrutin_uid, a.family_code, a.opened_on, a.motive
             FROM scrutin_debated_texts l
             JOIN theme_assignments a
               ON a.subject_kind = 'text' AND a.subject_id = l.text_key AND a.closed_on IS NULL
             WHERE l.scrutin_uid = ANY($1)
             ORDER BY a.opened_on, a.id",
        )
        .bind(scrutin_uids)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut by_scrutin: HashMap<String, Vec<AssignedFamily>> = HashMap::new();
        for row in &rows {
            if let Some(assigned) = assigned_from_row(row) {
                by_scrutin
                    .entry(row.get("scrutin_uid"))
                    .or_default()
                    .push(assigned);
            }
        }
        Ok(by_scrutin)
    }

    async fn families_of_dossier(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<AssignedFamily>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT a.family_code, a.opened_on, a.motive
             FROM dossier_debated_texts dt
             JOIN theme_assignments a
               ON a.subject_kind = 'text' AND a.subject_id = dt.text_key AND a.closed_on IS NULL
             WHERE dt.dossier_uid = $1
             UNION
             SELECT a.family_code, a.opened_on, a.motive
             FROM theme_assignments a
             WHERE a.subject_kind = 'dossier' AND a.subject_id = $1 AND a.closed_on IS NULL",
        )
        .bind(dossier_uid)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows.iter().filter_map(assigned_from_row).collect())
    }

    async fn text_count(&self) -> Result<i64, RepositoryError> {
        self.count("SELECT count(*) FROM debated_texts").await
    }

    async fn method_report(&self) -> Result<MethodReport, RepositoryError> {
        let rows = sqlx::query(
            "SELECT f.code,
                    count(DISTINCT a.subject_id) AS text_count,
                    coalesce((
                        SELECT count(*) FROM scrutin_debated_texts l
                        WHERE l.text_key IN (
                            SELECT b.subject_id FROM theme_assignments b
                            WHERE b.subject_kind = 'text' AND b.closed_on IS NULL
                              AND b.family_code = f.code
                        )
                    ), 0) AS scrutin_count
             FROM theme_families f
             LEFT JOIN theme_assignments a
               ON a.family_code = f.code AND a.subject_kind = 'text' AND a.closed_on IS NULL
             GROUP BY f.code, f.display_order
             ORDER BY f.display_order",
        )
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let families = rows
            .iter()
            .filter_map(|row| {
                Some(FamilyCoverage {
                    family: FamilyCode::parse(&row.get::<String, _>("code")).ok()?,
                    text_count: row.get("text_count"),
                    scrutin_count: row.get("scrutin_count"),
                })
            })
            .collect();

        let current_text_assignment = "SELECT 1 FROM theme_assignments a
             WHERE a.subject_kind = 'text' AND a.subject_id = t.text_key AND a.closed_on IS NULL";

        Ok(MethodReport {
            families,
            texts_total: self.count("SELECT count(*) FROM debated_texts").await?,
            texts_assigned: self
                .count(&format!(
                    "SELECT count(*) FROM debated_texts t WHERE EXISTS ({current_text_assignment})"
                ))
                .await?,
            texts_without_family: self
                .count(
                    "SELECT count(*) FROM debated_texts WHERE last_attempt_outcome = 'no_family'",
                )
                .await?,
            texts_attempt_failed: self
                .count("SELECT count(*) FROM debated_texts WHERE last_attempt_outcome = 'failed'")
                .await?,
            texts_never_attempted: self
                .count("SELECT count(*) FROM debated_texts WHERE last_attempt_on IS NULL")
                .await?,
            scrutins_total: self.count("SELECT count(*) FROM scrutins").await?,
            scrutins_with_text: self
                .count("SELECT count(*) FROM scrutin_debated_texts")
                .await?,
            scrutins_assigned: self
                .count(
                    "SELECT count(*) FROM scrutin_debated_texts l
                     WHERE EXISTS (
                         SELECT 1 FROM theme_assignments a
                         WHERE a.subject_kind = 'text' AND a.subject_id = l.text_key
                           AND a.closed_on IS NULL
                     )",
                )
                .await?,
            dossiers_total: self
                .count("SELECT count(*) FROM legislative_dossiers")
                .await?,
            dossiers_linked_to_text: self
                .count("SELECT count(DISTINCT dossier_uid) FROM dossier_debated_texts")
                .await?,
            // Un dossier est rattache soit par le texte que ses scrutins
            // nomment, soit — a defaut de scrutin — sur son propre titre
            // (RM-06). Les deux comptent.
            dossiers_assigned: self
                .count(
                    "SELECT count(*) FROM legislative_dossiers d
                     WHERE EXISTS (
                         SELECT 1 FROM dossier_debated_texts l
                         JOIN theme_assignments a
                           ON a.subject_kind = 'text' AND a.subject_id = l.text_key
                          AND a.closed_on IS NULL
                         WHERE l.dossier_uid = d.uid
                     )
                     OR EXISTS (
                         SELECT 1 FROM theme_assignments a
                         WHERE a.subject_kind = 'dossier' AND a.subject_id = d.uid
                           AND a.closed_on IS NULL
                     )",
                )
                .await?,
            dossiers_without_text: self
                .count(
                    "SELECT count(*) FROM legislative_dossiers d
                     WHERE NOT EXISTS (
                         SELECT 1 FROM dossier_debated_texts l WHERE l.dossier_uid = d.uid
                     )",
                )
                .await?,
        })
    }
}
