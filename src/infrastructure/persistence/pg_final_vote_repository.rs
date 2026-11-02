use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use super::{db, FINAL_VOTE_PREDICATE};
use crate::application::ports::final_vote_repository::{
    FinalVoteFilter, FinalVotePage, FinalVoteRecord, FinalVoteRepository, FinalVoteTotals,
    GroupOption, GroupTallyRecord, RepositoryError,
};
use crate::application::ports::theme_repository::AssignedFamily;
use crate::domain::final_vote::OfficialTextVersion;
use crate::domain::scrutin::VoteTally;
use crate::domain::theme::FamilyCode;

pub struct PgFinalVoteRepository {
    pool: PgPool,
}

impl PgFinalVoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Ventilations publiees des groupes demandes, pour les scrutins de la page.
    async fn tallies_for(
        &self,
        scrutin_uids: &[String],
        group_uids: &[String],
    ) -> Result<HashMap<String, Vec<GroupTallyRecord>>, RepositoryError> {
        if scrutin_uids.is_empty() || group_uids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query(
            "SELECT t.scrutin_uid, t.group_uid, g.abbrev, g.label, g.color,
                    t.member_count, t.majority_position,
                    t.votes_for, t.votes_against, t.abstentions,
                    t.not_voting, t.voluntary_not_voting
             FROM scrutin_group_tallies t
             JOIN parliamentary_groups g ON g.uid = t.group_uid
             WHERE t.scrutin_uid = ANY($1) AND t.group_uid = ANY($2)",
        )
        .bind(scrutin_uids)
        .bind(group_uids)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut by_scrutin: HashMap<String, Vec<GroupTallyRecord>> = HashMap::new();
        for row in &rows {
            by_scrutin
                .entry(row.get("scrutin_uid"))
                .or_default()
                .push(GroupTallyRecord {
                    group_uid: row.get("group_uid"),
                    abbrev: row.get("abbrev"),
                    label: row.get("label"),
                    color: row.get("color"),
                    member_count: row.get::<Option<i16>, _>("member_count").map(|c| c as u16),
                    majority_position: row.get("majority_position"),
                    tally: tally_from_row(row),
                });
        }
        Ok(by_scrutin)
    }

    /// Familles courantes des textes de la page (RM-06).
    async fn families_for(
        &self,
        text_keys: &[String],
    ) -> Result<HashMap<String, Vec<AssignedFamily>>, RepositoryError> {
        if text_keys.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = sqlx::query(
            "SELECT subject_id, family_code, opened_on, motive
             FROM theme_assignments
             WHERE subject_kind = 'text' AND closed_on IS NULL AND subject_id = ANY($1)
             ORDER BY opened_on, id",
        )
        .bind(text_keys)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut by_key: HashMap<String, Vec<AssignedFamily>> = HashMap::new();
        for row in &rows {
            let code: String = row.get("family_code");
            let Ok(family) = FamilyCode::parse(&code) else {
                continue;
            };
            by_key
                .entry(row.get("subject_id"))
                .or_default()
                .push(AssignedFamily {
                    family,
                    opened_on: row.get("opened_on"),
                    motive: row.get("motive"),
                });
        }
        Ok(by_key)
    }
}

fn tally_from_row(row: &sqlx::postgres::PgRow) -> VoteTally {
    VoteTally {
        votes_for: row.get::<i16, _>("votes_for") as u16,
        votes_against: row.get::<i16, _>("votes_against") as u16,
        abstentions: row.get::<i16, _>("abstentions") as u16,
        not_voting: row.get::<i16, _>("not_voting") as u16,
        voluntary_not_voting: row.get::<i16, _>("voluntary_not_voting") as u16,
    }
}

/// Reconstitue la référence seulement si les colonnes de la jointure sont
/// toutes présentes. Une ligne partielle ne doit jamais devenir un texte
/// affichable : le lecteur doit alors voir la limite de couverture.
fn official_text_from_row(row: &sqlx::postgres::PgRow) -> Option<OfficialTextVersion> {
    Some(OfficialTextVersion {
        document_uid: row.get("official_document_uid"),
        document_title: row.get("official_document_title"),
        version_label: row.get("official_version_label"),
        document_published_on: row.get("official_document_published_on"),
        official_url: row.get("official_document_url"),
        mapping_source_url: row.get("official_mapping_source_url"),
        source_producer: row.get("official_source_producer"),
        source_license: row.get("official_source_license"),
        source_metadata_fingerprint: row.get("official_source_metadata_fingerprint"),
        source_retrieved_at: row.get("official_source_retrieved_at"),
    })
}

#[async_trait]
impl FinalVoteRepository for PgFinalVoteRepository {
    async fn list_final_votes(
        &self,
        filter: &FinalVoteFilter,
    ) -> Result<FinalVotePage, RepositoryError> {
        // `count(*) OVER ()` porte le total du filtre sans seconde requete.
        // La jointure sur `scrutin_debated_texts` n'ecarte rien: les 222 votes
        // sur l'ensemble nomment tous un texte (mesure du 06/08/2026).
        let sql = format!(
            "SELECT s.uid, s.number, s.scrutin_date, s.subject, s.ballot_type_label,
                    s.outcome_code, s.outcome_label,
                    s.votes_for, s.votes_against, s.abstentions,
                    s.not_voting, s.voluntary_not_voting,
                    s.dossier_uid, s.dossier_label,
                    dt.text_key, dt.label AS text_label,
                    fvtv.document_uid AS official_document_uid,
                    fvtv.document_title AS official_document_title,
                    fvtv.version_label AS official_version_label,
                    fvtv.document_published_on AS official_document_published_on,
                    fvtv.official_url AS official_document_url,
                    fvtv.mapping_source_url AS official_mapping_source_url,
                    fvtv.source_producer AS official_source_producer,
                    fvtv.source_license AS official_source_license,
                    fvtv.source_metadata_fingerprint AS official_source_metadata_fingerprint,
                    fvtv.source_retrieved_at AS official_source_retrieved_at,
                    count(*) OVER () AS total
             FROM scrutins s
             JOIN scrutin_debated_texts sdt ON sdt.scrutin_uid = s.uid
             JOIN debated_texts dt ON dt.text_key = sdt.text_key
             LEFT JOIN final_vote_text_versions fvtv ON fvtv.scrutin_uid = s.uid
             WHERE {FINAL_VOTE_PREDICATE}
               AND ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM theme_assignments ta
                     WHERE ta.subject_kind = 'text'
                       AND ta.subject_id = dt.text_key
                       AND ta.closed_on IS NULL
                       AND ta.family_code = $1))
             ORDER BY s.scrutin_date DESC, s.number DESC
             LIMIT $2 OFFSET $3"
        );

        let rows = sqlx::query(&sql)
            .bind(filter.family.map(|f| f.as_str()))
            .bind(filter.limit)
            .bind(filter.offset)
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;

        let total = rows
            .first()
            .map(|row| row.get::<i64, _>("total"))
            .unwrap_or(0);

        let scrutin_uids: Vec<String> = rows.iter().map(|row| row.get("uid")).collect();
        let text_keys: Vec<String> = rows.iter().map(|row| row.get("text_key")).collect();

        let mut tallies = self.tallies_for(&scrutin_uids, &filter.group_uids).await?;
        let mut families = self.families_for(&text_keys).await?;

        let items = rows
            .iter()
            .map(|row| {
                let uid: String = row.get("uid");
                let text_key: String = row.get("text_key");
                let official_text = row
                    .get::<Option<String>, _>("official_document_uid")
                    .and_then(|_| official_text_from_row(row));
                FinalVoteRecord {
                    number: row.get("number"),
                    date: row.get("scrutin_date"),
                    subject: row.get("subject"),
                    ballot_type_label: row.get("ballot_type_label"),
                    outcome_code: row.get("outcome_code"),
                    outcome_label: row.get("outcome_label"),
                    text_label: row.get("text_label"),
                    dossier_uid: row.get("dossier_uid"),
                    dossier_label: row.get("dossier_label"),
                    official_text,
                    synthesis: tally_from_row(row),
                    families: families.remove(&text_key).unwrap_or_default(),
                    tallies: tallies.remove(&uid).unwrap_or_default(),
                    scrutin_uid: uid,
                    text_key,
                }
            })
            .collect();

        Ok(FinalVotePage { items, total })
    }

    async fn groups(&self) -> Result<Vec<GroupOption>, RepositoryError> {
        // Un groupe cree en cours de legislature n'a pas de ligne sur les votes
        // anterieurs: `final_vote_count` porte cette couverture a l'ecran.
        let sql = format!(
            "SELECT g.uid, g.abbrev, g.label, g.color, count(*) AS final_vote_count
             FROM parliamentary_groups g
             JOIN scrutin_group_tallies t ON t.group_uid = g.uid
             JOIN scrutins s ON s.uid = t.scrutin_uid
             WHERE {FINAL_VOTE_PREDICATE}
             GROUP BY g.uid, g.abbrev, g.label, g.color
             ORDER BY count(*) DESC, g.abbrev"
        );

        let rows = sqlx::query(&sql)
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| GroupOption {
                uid: row.get("uid"),
                abbrev: row.get("abbrev"),
                label: row.get("label"),
                color: row.get("color"),
                final_vote_count: row.get("final_vote_count"),
            })
            .collect())
    }

    async fn totals(&self) -> Result<FinalVoteTotals, RepositoryError> {
        let sql = format!(
            "SELECT count(*) AS total,
                    count(*) FILTER (WHERE EXISTS (
                        SELECT 1 FROM theme_assignments ta
                        WHERE ta.subject_kind = 'text'
                          AND ta.subject_id = sdt.text_key
                          AND ta.closed_on IS NULL)) AS with_family
             FROM scrutins s
             JOIN scrutin_debated_texts sdt ON sdt.scrutin_uid = s.uid
             WHERE {FINAL_VOTE_PREDICATE}"
        );

        let row = sqlx::query(&sql)
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;

        Ok(FinalVoteTotals {
            total: row.get("total"),
            with_family: row.get("with_family"),
        })
    }
}
