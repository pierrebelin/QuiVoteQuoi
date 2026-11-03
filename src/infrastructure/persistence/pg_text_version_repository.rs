//! Dépôt Postgres des versions d'un texte et de leurs articles
//! (SPEC-en-bref RM-01, RM-03, RM-04, RM-15).

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use super::{db, invalid_row};
use crate::application::ports::text_version_repository::{TextReading, TextVersionRepository};
use crate::application::ports::RepositoryError;
use crate::domain::dossier::DossierUid;
use crate::domain::law_text::{ArticleFingerprint, NewVersionArticle, VersionArticle};
use crate::domain::text_version::{
    BriefCandidate, Chamber, DocumentUid, NewTextVersion, ReadingStatus, TextVersion, VersionKind,
};

/// Lignes par instruction `UNNEST`.
const ROW_BATCH: usize = 1000;

/// Colonnes d'une version, dans l'ordre lu par `version_from_row`.
const VERSION_COLUMNS: &str =
    "document_uid, dossier_uid, kind, chamber, deposited_on, title, reading_status";

/// Dossiers votés (RM-15) et date de leur dernier scrutin : un dossier est voté
/// s'il est nommé par un scrutin (`scrutins.dossier_uid`) ou s'il est relié à
/// un texte débattu dont des scrutins existent (`dossier_debated_texts`).
const VOTED_DOSSIERS_CTE: &str = "
    voted AS (
        SELECT s.dossier_uid, MAX(s.scrutin_date) AS last_vote_on
          FROM scrutins s
         WHERE s.dossier_uid IS NOT NULL
         GROUP BY s.dossier_uid
        UNION ALL
        SELECT d.dossier_uid, MAX(s.scrutin_date) AS last_vote_on
          FROM dossier_debated_texts d
          JOIN scrutin_debated_texts l ON l.text_key = d.text_key
          JOIN scrutins s ON s.uid = l.scrutin_uid
         GROUP BY d.dossier_uid
    )";

pub struct PgTextVersionRepository {
    pool: PgPool,
}

impl PgTextVersionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn version_from_row(row: &PgRow) -> Result<TextVersion, RepositoryError> {
    let kind: String = row.get("kind");
    let chamber: String = row.get("chamber");
    let reading: String = row.get("reading_status");
    Ok(TextVersion::new(NewTextVersion {
        document_uid: DocumentUid::new(row.get("document_uid")).map_err(invalid_row)?,
        dossier_uid: DossierUid::new(row.get("dossier_uid")).map_err(invalid_row)?,
        kind: VersionKind::parse(&kind).ok_or_else(|| invalid_row(format!("kind {kind}")))?,
        chamber: Chamber::parse(&chamber)
            .ok_or_else(|| invalid_row(format!("chamber {chamber}")))?,
        deposited_on: row.get::<NaiveDate, _>("deposited_on"),
        title: row.get("title"),
        reading: ReadingStatus::parse(&reading)
            .ok_or_else(|| invalid_row(format!("reading status {reading}")))?,
    }))
}

#[async_trait]
impl TextVersionRepository for PgTextVersionRepository {
    async fn save_versions(&self, versions: &[TextVersion]) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(db)?;
        for batch in versions.chunks(ROW_BATCH) {
            let uids: Vec<&str> = batch.iter().map(|v| v.document_uid().as_str()).collect();
            let dossiers: Vec<&str> = batch.iter().map(|v| v.dossier_uid().as_str()).collect();
            let kinds: Vec<&str> = batch.iter().map(|v| v.kind().as_str()).collect();
            let chambers: Vec<&str> = batch.iter().map(|v| v.chamber().as_str()).collect();
            let dates: Vec<NaiveDate> = batch.iter().map(TextVersion::deposited_on).collect();
            let titles: Vec<&str> = batch.iter().map(TextVersion::title).collect();
            // Seules les métadonnées sont mises à jour: l'état de lecture, sa
            // date et les articles d'une version déjà lue ne bougent jamais ici.
            sqlx::query(
                "INSERT INTO text_versions
                    (document_uid, dossier_uid, kind, chamber, deposited_on, title)
                 SELECT * FROM UNNEST(
                    $1::text[], $2::text[], $3::text[], $4::text[], $5::date[], $6::text[])
                 ON CONFLICT (document_uid) DO UPDATE SET
                    dossier_uid = EXCLUDED.dossier_uid,
                    kind = EXCLUDED.kind,
                    chamber = EXCLUDED.chamber,
                    deposited_on = EXCLUDED.deposited_on,
                    title = EXCLUDED.title,
                    updated_at = NOW()",
            )
            .bind(&uids)
            .bind(&dossiers)
            .bind(&kinds)
            .bind(&chambers)
            .bind(&dates)
            .bind(&titles)
            .execute(&mut *transaction)
            .await
            .map_err(db)?;
        }
        transaction.commit().await.map_err(db)
    }

    async fn versions_of(&self, dossier: &DossierUid) -> Result<Vec<TextVersion>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {VERSION_COLUMNS}
               FROM text_versions
              WHERE dossier_uid = $1
              ORDER BY deposited_on, document_uid"
        ))
        .bind(dossier.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter().map(version_from_row).collect()
    }

    async fn voted_dossiers(&self) -> Result<Vec<BriefCandidate>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "WITH {VOTED_DOSSIERS_CTE}
             SELECT v.dossier_uid, MAX(v.last_vote_on) AS last_vote_on
               FROM voted v
              GROUP BY v.dossier_uid"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.iter()
            .map(|row| {
                Ok(BriefCandidate {
                    dossier_uid: DossierUid::new(row.get("dossier_uid")).map_err(invalid_row)?,
                    last_vote_on: row.get::<NaiveDate, _>("last_vote_on"),
                })
            })
            .collect()
    }

    async fn versions_of_voted(&self) -> Result<Vec<TextVersion>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "WITH {VOTED_DOSSIERS_CTE}
             SELECT {VERSION_COLUMNS}
               FROM text_versions
              WHERE dossier_uid IN (SELECT dossier_uid FROM voted)
              ORDER BY dossier_uid, deposited_on, document_uid"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter().map(version_from_row).collect()
    }

    async fn articles_of(
        &self,
        document_uid: &DocumentUid,
    ) -> Result<Option<Vec<VersionArticle>>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT rank, label, division, content, fingerprint
               FROM text_version_articles
              WHERE document_uid = $1
              ORDER BY rank",
        )
        .bind(document_uid.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        // Une version lue a au moins un article: aucune ligne, c'est un texte
        // jamais lu ou noté manquant.
        if rows.is_empty() {
            return Ok(None);
        }
        rows.iter()
            .map(|row| {
                let rank: i32 = row.get("rank");
                Ok(VersionArticle::new(NewVersionArticle {
                    label: row.get("label"),
                    division: row.get("division"),
                    rank: u16::try_from(rank).map_err(|_| invalid_row(format!("rank {rank}")))?,
                    content: row.get("content"),
                    fingerprint: ArticleFingerprint::new(row.get("fingerprint")),
                }))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    async fn save_reading(
        &self,
        document_uid: &DocumentUid,
        reading: &TextReading,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(db)?;
        sqlx::query("DELETE FROM text_version_articles WHERE document_uid = $1")
            .bind(document_uid.as_str())
            .execute(&mut *transaction)
            .await
            .map_err(db)?;

        let status = match reading {
            TextReading::Articles(articles) => {
                for batch in articles.chunks(ROW_BATCH) {
                    let ranks: Vec<i32> = batch.iter().map(|a| i32::from(a.rank())).collect();
                    let labels: Vec<&str> = batch.iter().map(VersionArticle::label).collect();
                    let divisions: Vec<Option<&str>> =
                        batch.iter().map(VersionArticle::division).collect();
                    let contents: Vec<&str> = batch.iter().map(VersionArticle::content).collect();
                    let fingerprints: Vec<&str> =
                        batch.iter().map(|a| a.fingerprint().as_str()).collect();
                    sqlx::query(
                        "INSERT INTO text_version_articles
                            (document_uid, rank, label, division, content, fingerprint)
                         SELECT $1::text, * FROM UNNEST(
                            $2::int[], $3::text[], $4::text[], $5::text[], $6::text[])",
                    )
                    .bind(document_uid.as_str())
                    .bind(&ranks)
                    .bind(&labels)
                    .bind(&divisions)
                    .bind(&contents)
                    .bind(&fingerprints)
                    .execute(&mut *transaction)
                    .await
                    .map_err(db)?;
                }
                ReadingStatus::Read
            }
            TextReading::Missing => ReadingStatus::Missing,
        };

        sqlx::query(
            "UPDATE text_versions
                SET reading_status = $2, read_at = NOW(), updated_at = NOW()
              WHERE document_uid = $1",
        )
        .bind(document_uid.as_str())
        .bind(status.as_str())
        .execute(&mut *transaction)
        .await
        .map_err(db)?;
        transaction.commit().await.map_err(db)
    }
}
