use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::group_repository::{
    GroupRecord, GroupRepository, GroupStatisticsRecord, RepositoryError,
};
use crate::domain::group_profile::{ParticipationCounts, QualityCount};
use crate::domain::legislature::CURRENT_LEGISLATURE;

/// Appartenances valides a une date donnee. La fin est inclusive, comme dans
/// `domain::actor::MembershipPeriod`.
const ACTIVE_ON: &str = "m.start_date <= $2 AND (m.end_date IS NULL OR m.end_date >= $2)";

pub struct PgGroupRepository {
    pool: PgPool,
}

impl PgGroupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GroupRepository for PgGroupRepository {
    async fn list_groups(&self, today: NaiveDate) -> Result<Vec<GroupRecord>, RepositoryError> {
        // `LEAST(g.end_date, $1)` compte l'effectif d'un groupe dissous a son
        // dernier jour. Postgres ignore les NULL dans LEAST: un groupe actif
        // retombe sur $1. Le compter a la date du jour donnerait zero membre,
        // ce qui se lit comme une donnee manquante plutot que comme une
        // dissolution.
        //
        // La jointure sur les ventilations est externe: un groupe sans une
        // seule ligne de vote reste liste, couverture nulle affichee comme
        // telle (README.md §2).
        let rows = sqlx::query(
            "SELECT g.uid, g.legislature, g.label, g.abbrev, g.color,
                    g.start_date, g.end_date,
                    LEAST(g.end_date, $1::date) AS reference_date,
                    (SELECT count(DISTINCT m.actor_uid)
                       FROM group_memberships m
                      WHERE m.group_uid = g.uid
                        AND m.start_date <= LEAST(g.end_date, $1::date)
                        AND (m.end_date IS NULL OR m.end_date >= LEAST(g.end_date, $1::date))
                    ) AS member_count,
                    coalesce(t.scrutin_count, 0) AS scrutin_count,
                    t.first_scrutin_date,
                    t.last_scrutin_date
               FROM parliamentary_groups g
               LEFT JOIN (
                    SELECT tal.group_uid,
                           count(*) AS scrutin_count,
                           min(s.scrutin_date) AS first_scrutin_date,
                           max(s.scrutin_date) AS last_scrutin_date
                      FROM scrutin_group_tallies tal
                      JOIN scrutins s ON s.uid = tal.scrutin_uid
                     GROUP BY tal.group_uid
               ) t ON t.group_uid = g.uid
              WHERE g.legislature = $2::smallint
              ORDER BY g.abbrev",
        )
        .bind(today)
        .bind(CURRENT_LEGISLATURE as i16)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| GroupRecord {
                uid: row.get("uid"),
                legislature: row.get("legislature"),
                label: row.get("label"),
                abbrev: row.get("abbrev"),
                color: row.get("color"),
                start_date: row.get("start_date"),
                end_date: row.get("end_date"),
                reference_date: row.get("reference_date"),
                member_count: row.get("member_count"),
                scrutin_count: row.get("scrutin_count"),
                first_scrutin_date: row.get("first_scrutin_date"),
                last_scrutin_date: row.get("last_scrutin_date"),
            })
            .collect())
    }

    async fn statistics(
        &self,
        group_uids: &[String],
        reference_date: NaiveDate,
    ) -> Result<GroupStatisticsRecord, RepositoryError> {
        let uids = group_uids.to_vec();

        // `= ANY($1)` porte tous les identifiants de la lignee: un groupe
        // renomme garde les chiffres d'avant son changement de nom, que le
        // referentiel range sous un autre identifiant.
        let tallies = sqlx::query(
            "SELECT count(*) AS line_count,
                    coalesce(sum(votes_for), 0)::bigint AS votes_for,
                    coalesce(sum(votes_against), 0)::bigint AS votes_against,
                    coalesce(sum(abstentions), 0)::bigint AS abstentions,
                    coalesce(sum(not_voting), 0)::bigint AS not_voting,
                    coalesce(sum(voluntary_not_voting), 0)::bigint AS voluntary_not_voting,
                    count(*) FILTER (WHERE origin = 'reconstructed') AS reconstructed_count,
                    count(*) FILTER (WHERE votes_for = 0 AND votes_against = 0
                                       AND abstentions = 0 AND not_voting = 0
                                       AND voluntary_not_voting = 0) AS silent_line_count,
                    min(member_count) AS min_member_count,
                    max(member_count) AS max_member_count
               FROM scrutin_group_tallies
              WHERE group_uid = ANY($1)",
        )
        .bind(&uids)
        .persistent(false)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;

        // Un depute passe du groupe a son renommage compte pour un: le DISTINCT
        // porte sur la lignee entiere, pas sur chaque identifiant.
        let members = sqlx::query(
            "SELECT count(DISTINCT m.actor_uid) AS total_member_count
               FROM group_memberships m
              WHERE m.group_uid = ANY($1)",
        )
        .bind(&uids)
        .persistent(false)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;

        let quality_rows = sqlx::query(&format!(
            "SELECT m.quality, count(DISTINCT m.actor_uid) AS members
               FROM group_memberships m
              WHERE m.group_uid = ANY($1) AND {ACTIVE_ON}
              GROUP BY m.quality
              ORDER BY count(DISTINCT m.actor_uid) DESC, m.quality"
        ))
        .bind(&uids)
        .bind(reference_date)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(GroupStatisticsRecord {
            total_member_count: members.get("total_member_count"),
            qualities: quality_rows
                .iter()
                .map(|row| QualityCount {
                    quality: row.get("quality"),
                    members: row.get("members"),
                })
                .collect(),
            min_published_member_count: tallies
                .get::<Option<i16>, _>("min_member_count")
                .map(|c| c as u16),
            max_published_member_count: tallies
                .get::<Option<i16>, _>("max_member_count")
                .map(|c| c as u16),
            line_count: tallies.get("line_count"),
            reconstructed_count: tallies.get("reconstructed_count"),
            silent_line_count: tallies.get("silent_line_count"),
            counts: ParticipationCounts {
                votes_for: tallies.get::<i64, _>("votes_for") as u64,
                votes_against: tallies.get::<i64, _>("votes_against") as u64,
                abstentions: tallies.get::<i64, _>("abstentions") as u64,
                not_voting: tallies.get::<i64, _>("not_voting") as u64,
                voluntary_not_voting: tallies.get::<i64, _>("voluntary_not_voting") as u64,
            },
        })
    }
}
