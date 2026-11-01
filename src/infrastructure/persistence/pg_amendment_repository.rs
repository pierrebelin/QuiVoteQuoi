use async_trait::async_trait;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use super::db;
use crate::application::ports::amendment_repository::{
    AmendmentGroupOption, AmendmentPage, AmendmentPageRequest, AmendmentRepository,
    AmendmentSummary, DossierAmendmentCoverage, RepositoryError, SignatoryRow,
};
use crate::domain::amendment::{Amendment, Author};

/// Amendements par transaction. Plus large que `SCRUTIN_BATCH` (100): un
/// amendement porte quelques signataires la ou un scrutin en porte jusqu'a 574.
/// A recaler sur la volumetrie reelle (SPEC-amendements H3, H9).
const AMENDMENT_BATCH: usize = 500;

pub struct PgAmendmentRepository {
    pool: PgPool,
}

impl PgAmendmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Amendements rattaches a un dossier.
///
/// RM-05: le lien passe par l'identifiant de texte legislatif publie, joint aux
/// documents du dossier. Un identifiant des deux cotes, aucun rapprochement par
/// similarite de libelle. La jointure est faite a la lecture et non figee a
/// l'ingestion: les dossiers arrivent en incremental, et une jointure se repare
/// d'elle-meme quand le dossier finit par etre ingere.
const FROM_DOSSIER: &str = "FROM amendments a
     WHERE a.text_ref IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM dossier_documents d
           WHERE d.dossier_uid = $1 AND d.document_uid = a.text_ref
       )";

fn push_filters(query: &mut QueryBuilder<'_, Postgres>, page: &AmendmentPageRequest) {
    if let Some(search) = page.search.as_deref() {
        query.push(" AND (a.number ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push(" OR a.target_title ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push(" OR COALESCE(a.author_label, '') ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push(" OR EXISTS (SELECT 1 FROM actors actor WHERE actor.uid = a.author_actor_uid AND (actor.first_name ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push(" OR actor.last_name ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push("))");
        query.push(" OR COALESCE(a.summary, '') ILIKE ");
        query.push_bind(format!("%{search}%"));
        query.push(")");
    }
    if let Some(fate) = page.fate_code.as_deref() {
        if fate == "other" {
            query.push(" AND a.fate_code NOT IN ('adopted', 'rejected')");
        } else {
            query.push(" AND a.fate_code = ");
            query.push_bind(fate.to_string());
        }
    }
    if let Some(group_uid) = page.author_group_uid.as_deref() {
        query.push(" AND a.author_group_uid = ");
        query.push_bind(group_uid.to_string());
    }
}

#[async_trait]
impl AmendmentRepository for PgAmendmentRepository {
    async fn save_amendments(&self, amendments: &[Amendment]) -> Result<usize, RepositoryError> {
        let mut written = 0usize;

        for batch in amendments.chunks(AMENDMENT_BATCH) {
            let mut tx = self.pool.begin().await.map_err(db)?;
            let uids: Vec<&str> = batch.iter().map(|a| a.uid().as_str()).collect();

            // Reecriture complete des signataires: un cosignataire retire a la
            // source doit disparaitre, pas s'ajouter a l'ancienne liste.
            sqlx::query("DELETE FROM amendment_signatories WHERE amendment_uid = ANY($1)")
                .bind(&uids)
                .execute(&mut *tx)
                .await
                .map_err(db)?;

            let mut legislatures = Vec::with_capacity(batch.len());
            let mut numbers = Vec::with_capacity(batch.len());
            let mut number_keys = Vec::with_capacity(batch.len());
            let mut text_refs: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut examination_refs: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut target_titles = Vec::with_capacity(batch.len());
            let mut target_kinds: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut author_kinds = Vec::with_capacity(batch.len());
            let mut author_actor_uids: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut author_labels: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut author_group_uids: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut author_group_origins = Vec::with_capacity(batch.len());
            let mut author_group_ambiguous = Vec::with_capacity(batch.len());
            let mut fate_codes = Vec::with_capacity(batch.len());
            let mut fate_labels = Vec::with_capacity(batch.len());
            let mut state_labels: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut deposited = Vec::with_capacity(batch.len());
            let mut parents: Vec<Option<&str>> = Vec::with_capacity(batch.len());
            let mut summaries: Vec<Option<&str>> = Vec::with_capacity(batch.len());

            let mut signatory_amendments: Vec<&str> = Vec::new();
            let mut signatory_actors: Vec<&str> = Vec::new();
            let mut signatory_roles: Vec<&str> = Vec::new();
            let mut signatory_ranks: Vec<i16> = Vec::new();
            let mut signatory_groups: Vec<Option<&str>> = Vec::new();
            let mut signatory_origins: Vec<&str> = Vec::new();
            let mut signatory_ambiguous: Vec<bool> = Vec::new();
            let mut signatory_dates = Vec::new();

            for amendment in batch {
                legislatures.push(amendment.legislature() as i16);
                numbers.push(amendment.number().as_str());
                number_keys.push(amendment.number().key());
                text_refs.push(amendment.text_ref().map(|r| r.as_str()));
                examination_refs.push(amendment.examination_ref());
                target_titles.push(amendment.target().title.as_str());
                target_kinds.push(amendment.target().kind.as_deref());
                fate_codes.push(amendment.fate().code().as_str());
                fate_labels.push(amendment.fate().label());
                state_labels.push(amendment.state_label());
                deposited.push(amendment.deposited_on());
                parents.push(amendment.parent_uid().map(|p| p.as_str()));
                summaries.push(amendment.summary());

                match amendment.author() {
                    Author::Deputy(signatory) => {
                        author_kinds.push("deputy");
                        author_actor_uids.push(Some(signatory.actor_uid.as_str()));
                        author_labels.push(None);
                        author_group_uids.push(signatory.group_uid.as_ref().map(|g| g.as_str()));
                        author_group_origins.push(signatory.group_origin.as_str());
                        author_group_ambiguous.push(signatory.group_ambiguous);

                        signatory_amendments.push(amendment.uid().as_str());
                        signatory_actors.push(signatory.actor_uid.as_str());
                        signatory_roles.push(signatory.role.as_str());
                        signatory_ranks.push(signatory.rank as i16);
                        signatory_groups.push(signatory.group_uid.as_ref().map(|g| g.as_str()));
                        signatory_origins.push(signatory.group_origin.as_str());
                        signatory_ambiguous.push(signatory.group_ambiguous);
                        signatory_dates.push(amendment.deposited_on());
                    }
                    Author::Institutional { label } => {
                        author_kinds.push("institutional");
                        author_actor_uids.push(None);
                        author_labels.push(Some(label.as_str()));
                        author_group_uids.push(None);
                        author_group_origins.push("unknown");
                        author_group_ambiguous.push(false);
                    }
                }

                for signatory in amendment.cosignatories() {
                    signatory_amendments.push(amendment.uid().as_str());
                    signatory_actors.push(signatory.actor_uid.as_str());
                    signatory_roles.push(signatory.role.as_str());
                    signatory_ranks.push(signatory.rank as i16);
                    signatory_groups.push(signatory.group_uid.as_ref().map(|g| g.as_str()));
                    signatory_origins.push(signatory.group_origin.as_str());
                    signatory_ambiguous.push(signatory.group_ambiguous);
                    signatory_dates.push(amendment.deposited_on());
                }
            }

            sqlx::query(
                "INSERT INTO amendments (
                    uid, legislature, number, number_key, text_ref, examination_ref,
                    target_title, target_kind,
                    author_kind, author_actor_uid, author_label,
                    author_group_uid, author_group_origin, author_group_ambiguous,
                    fate_code, fate_label, state_label, deposited_on, parent_uid, summary
                 )
                 SELECT * FROM UNNEST(
                    $1::text[], $2::smallint[], $3::text[], $4::text[], $5::text[], $6::text[],
                    $7::text[], $8::text[],
                    $9::text[], $10::text[], $11::text[],
                    $12::text[], $13::text[], $14::bool[],
                    $15::text[], $16::text[], $17::text[], $18::date[], $19::text[], $20::text[]
                 )
                 ON CONFLICT (uid) DO UPDATE SET
                    legislature = EXCLUDED.legislature,
                    number = EXCLUDED.number,
                    number_key = EXCLUDED.number_key,
                    text_ref = EXCLUDED.text_ref,
                    examination_ref = EXCLUDED.examination_ref,
                    target_title = EXCLUDED.target_title,
                    target_kind = EXCLUDED.target_kind,
                    author_kind = EXCLUDED.author_kind,
                    author_actor_uid = EXCLUDED.author_actor_uid,
                    author_label = EXCLUDED.author_label,
                    author_group_uid = EXCLUDED.author_group_uid,
                    author_group_origin = EXCLUDED.author_group_origin,
                    author_group_ambiguous = EXCLUDED.author_group_ambiguous,
                    fate_code = EXCLUDED.fate_code,
                    fate_label = EXCLUDED.fate_label,
                    state_label = EXCLUDED.state_label,
                    deposited_on = EXCLUDED.deposited_on,
                    parent_uid = EXCLUDED.parent_uid,
                    summary = EXCLUDED.summary,
                    updated_at = NOW()",
            )
            .bind(&uids)
            .bind(&legislatures)
            .bind(&numbers)
            .bind(&number_keys)
            .bind(&text_refs)
            .bind(&examination_refs)
            .bind(&target_titles)
            .bind(&target_kinds)
            .bind(&author_kinds)
            .bind(&author_actor_uids)
            .bind(&author_labels)
            .bind(&author_group_uids)
            .bind(&author_group_origins)
            .bind(&author_group_ambiguous)
            .bind(&fate_codes)
            .bind(&fate_labels)
            .bind(&state_labels)
            .bind(&deposited)
            .bind(&parents)
            .bind(&summaries)
            .execute(&mut *tx)
            .await
            .map_err(db)?;

            if !signatory_amendments.is_empty() {
                sqlx::query(
                    "INSERT INTO amendment_signatories (
                        amendment_uid, actor_uid, role, rank,
                        group_uid, group_origin, group_ambiguous, deposited_on
                     )
                     SELECT * FROM UNNEST(
                        $1::text[], $2::text[], $3::text[], $4::smallint[],
                        $5::text[], $6::text[], $7::bool[], $8::date[]
                     )
                     ON CONFLICT (amendment_uid, actor_uid) DO NOTHING",
                )
                .bind(&signatory_amendments)
                .bind(&signatory_actors)
                .bind(&signatory_roles)
                .bind(&signatory_ranks)
                .bind(&signatory_groups)
                .bind(&signatory_origins)
                .bind(&signatory_ambiguous)
                .bind(&signatory_dates)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            tx.commit().await.map_err(db)?;
            written += batch.len();
        }

        Ok(written)
    }

    async fn by_dossier(
        &self,
        dossier_uid: &str,
        page: &AmendmentPageRequest,
    ) -> Result<AmendmentPage, RepositoryError> {
        let mut count_query: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT count(*) FROM amendments a
             WHERE a.text_ref IS NOT NULL
               AND EXISTS (SELECT 1 FROM dossier_documents d
                           WHERE d.dossier_uid = ",
        );
        count_query.push_bind(dossier_uid);
        count_query.push(" AND d.document_uid = a.text_ref)");
        push_filters(&mut count_query, page);
        let total: i64 = count_query
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;

        // RM-07: ordre de depot, mecanique et annonce. Trier sur le numero
        // publie melerait « 100 » et « 99 »; trier sur le nombre de
        // cosignataires serait un classement (README.md §6). Les amendements
        // sans date publiee ferment la liste plutot que d'ouvrir dessus.
        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT a.uid, a.number, a.target_title, a.target_kind,
                    a.author_kind, a.author_actor_uid, a.author_label,
                    a.author_group_uid, a.author_group_origin, a.author_group_ambiguous,
                    a.fate_code, a.fate_label, a.state_label, a.deposited_on, a.summary,
                    (SELECT count(*) FROM amendment_signatories s
                      WHERE s.amendment_uid = a.uid AND s.role = 'cosignatory') AS cosignatory_count
             FROM amendments a
             WHERE a.text_ref IS NOT NULL
               AND EXISTS (SELECT 1 FROM dossier_documents d
                           WHERE d.dossier_uid = ",
        );
        query.push_bind(dossier_uid);
        query.push(" AND d.document_uid = a.text_ref)");
        push_filters(&mut query, page);
        query.push(" ORDER BY a.deposited_on DESC NULLS LAST, a.uid DESC LIMIT ");
        query.push_bind(page.limit);
        query.push(" OFFSET ");
        query.push_bind(page.offset);
        let rows = query.build().fetch_all(&self.pool).await.map_err(db)?;

        let items = rows
            .iter()
            .map(|row| AmendmentSummary {
                uid: row.get("uid"),
                number: row.get("number"),
                target_title: row.get("target_title"),
                target_kind: row.get("target_kind"),
                author_kind: row.get("author_kind"),
                author_actor_uid: row.get("author_actor_uid"),
                author_label: row.get("author_label"),
                author_group_uid: row.get("author_group_uid"),
                author_group_origin: row.get("author_group_origin"),
                author_group_ambiguous: row.get("author_group_ambiguous"),
                fate_code: row.get("fate_code"),
                fate_label: row.get("fate_label"),
                state_label: row.get("state_label"),
                deposited_on: row.get("deposited_on"),
                summary: row.get("summary"),
                cosignatory_count: row.get("cosignatory_count"),
            })
            .collect();

        Ok(AmendmentPage { items, total })
    }

    async fn groups_by_dossier(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<AmendmentGroupOption>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT DISTINCT g.uid, g.label, g.abbrev
             FROM amendments a
             JOIN dossier_documents d ON d.document_uid = a.text_ref
             JOIN parliamentary_groups g ON g.uid = a.author_group_uid
             WHERE d.dossier_uid = $1
             ORDER BY g.label, g.uid"
        ))
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| AmendmentGroupOption {
                uid: row.get("uid"),
                label: row.get("label"),
                abbrev: row.get("abbrev"),
            })
            .collect())
    }

    async fn dossier_coverage(
        &self,
        dossier_uid: &str,
    ) -> Result<DossierAmendmentCoverage, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT count(*) AS total,
                    count(*) FILTER (WHERE a.summary IS NULL) AS without_summary,
                    count(*) FILTER (WHERE a.fate_code = 'other') AS unknown_fates
             {FROM_DOSSIER}"
        ))
        .bind(dossier_uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;

        let base_total: i64 = sqlx::query_scalar("SELECT count(*) FROM amendments")
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;

        Ok(DossierAmendmentCoverage {
            base_total,
            total: row.get("total"),
            without_summary: row.get("without_summary"),
            unknown_fates: row.get("unknown_fates"),
        })
    }

    async fn signatories_of(
        &self,
        amendment_uid: &str,
    ) -> Result<Vec<SignatoryRow>, RepositoryError> {
        // Ordre publie: auteur d'abord, puis les cosignataires par rang. Aucun
        // tri alphabetique ni par groupe, qui serait un classement.
        let rows = sqlx::query(
            "SELECT actor_uid, role, rank, group_uid, group_origin, group_ambiguous
             FROM amendment_signatories
             WHERE amendment_uid = $1
             ORDER BY (role = 'author') DESC, rank ASC, actor_uid ASC",
        )
        .bind(amendment_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| SignatoryRow {
                actor_uid: row.get("actor_uid"),
                role: row.get("role"),
                rank: row.get("rank"),
                group_uid: row.get("group_uid"),
                group_origin: row.get("group_origin"),
                group_ambiguous: row.get("group_ambiguous"),
            })
            .collect())
    }

    async fn last_archive_id(&self, label: &str) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar("SELECT digest FROM source_archives WHERE label = $1")
            .bind(label)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)
    }

    async fn remember_archive(&self, label: &str, id: &str) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO source_archives (label, digest)
             VALUES ($1, $2)
             ON CONFLICT (label) DO UPDATE SET digest = EXCLUDED.digest, ingested_at = NOW()",
        )
        .bind(label)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(db)?;
        Ok(())
    }
}
