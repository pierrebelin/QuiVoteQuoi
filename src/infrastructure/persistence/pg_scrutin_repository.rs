use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use super::db;
use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinFilter, ScrutinPage, ScrutinRepository, ScrutinSummary,
};
use crate::domain::actor::{ActorUid, GroupUid};
use crate::domain::dataset::{CodeCount, DatasetShape};
use crate::domain::dossier::DossierUid;
use crate::domain::scrutin::{
    BallotType, DossierReference, GroupTally, NewScrutin, NominalVote, NonVotingCause, Outcome,
    Scrutin, ScrutinUid, TallyOrigin, VoteCorrection, VotePosition, VoteSynthesis, VoteTally,
};

/// Scrutins par transaction. Chaque lot supprime puis reecrit ses lignes filles:
/// borner la transaction garde l'ecriture reprenable sur la base locale.
const SCRUTIN_BATCH: usize = 100;
/// Lignes par instruction `UNNEST`. Un scrutin porte jusqu'a 574 positions
/// nominales, un lot de 100 scrutins en porte donc jusqu'a 57 400.
const ROW_BATCH: usize = 4000;
/// Bornes de la taille de page d'une liste de scrutins.
const PAGE_LIMIT_MIN: i64 = 1;
const PAGE_LIMIT_MAX: i64 = 200;

pub struct PgScrutinRepository {
    pool: PgPool,
}

impl PgScrutinRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Répartition des scrutins sur un couple code/libellé de la table. Les
    /// deux noms de colonnes sont des constantes du code, jamais une saisie.
    async fn code_counts(
        &self,
        code_column: &str,
        label_column: &str,
    ) -> Result<Vec<CodeCount>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {code_column} AS code,
                    min({label_column}) AS label,
                    count(*) AS total
             FROM scrutins
             GROUP BY {code_column}
             ORDER BY count(*) DESC, {code_column}"
        ))
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .iter()
            .map(|row| CodeCount {
                code: row.get("code"),
                label: row.get("label"),
                count: row.get("total"),
            })
            .collect())
    }
}

#[async_trait]
impl ScrutinRepository for PgScrutinRepository {
    async fn save_scrutins(&self, scrutins: &[Scrutin]) -> Result<usize, RepositoryError> {
        let mut written = 0usize;

        for batch in scrutins.chunks(SCRUTIN_BATCH) {
            let mut tx = self.pool.begin().await.map_err(db)?;
            let uids: Vec<&str> = batch.iter().map(|s| s.uid().as_str()).collect();

            // Reecriture complete des lignes filles: une mise au point ajoutee
            // apres coup, ou une repartition reconstruite differemment, doit
            // remplacer l'ancienne plutot que s'y ajouter.
            for table in [
                "scrutin_vote_corrections",
                "scrutin_votes",
                "scrutin_group_tallies",
            ] {
                sqlx::query(&format!("DELETE FROM {table} WHERE scrutin_uid = ANY($1)"))
                    .bind(&uids)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }

            sqlx::query(
                "INSERT INTO scrutins (
                    uid, number, legislature, scrutin_date, session_ref, sitting_ref, place,
                    ballot_type_code, ballot_type_label, majority_label,
                    outcome_code, outcome_label, requester, subject,
                    voters, expressed, required, announcement,
                    votes_for, votes_against, abstentions, not_voting, voluntary_not_voting,
                    dossier_uid, dossier_label
                 )
                 SELECT * FROM UNNEST(
                    $1::text[], $2::text[], $3::smallint[], $4::date[], $5::text[], $6::text[], $7::text[],
                    $8::text[], $9::text[], $10::text[],
                    $11::text[], $12::text[], $13::text[], $14::text[],
                    $15::smallint[], $16::smallint[], $17::smallint[], $18::text[],
                    $19::smallint[], $20::smallint[], $21::smallint[], $22::smallint[], $23::smallint[],
                    $24::text[], $25::text[]
                 )
                 ON CONFLICT (uid) DO UPDATE SET
                    number = EXCLUDED.number,
                    legislature = EXCLUDED.legislature,
                    scrutin_date = EXCLUDED.scrutin_date,
                    session_ref = EXCLUDED.session_ref,
                    sitting_ref = EXCLUDED.sitting_ref,
                    place = EXCLUDED.place,
                    ballot_type_code = EXCLUDED.ballot_type_code,
                    ballot_type_label = EXCLUDED.ballot_type_label,
                    majority_label = EXCLUDED.majority_label,
                    outcome_code = EXCLUDED.outcome_code,
                    outcome_label = EXCLUDED.outcome_label,
                    requester = EXCLUDED.requester,
                    subject = EXCLUDED.subject,
                    voters = EXCLUDED.voters,
                    expressed = EXCLUDED.expressed,
                    required = EXCLUDED.required,
                    announcement = EXCLUDED.announcement,
                    votes_for = EXCLUDED.votes_for,
                    votes_against = EXCLUDED.votes_against,
                    abstentions = EXCLUDED.abstentions,
                    not_voting = EXCLUDED.not_voting,
                    voluntary_not_voting = EXCLUDED.voluntary_not_voting,
                    dossier_uid = EXCLUDED.dossier_uid,
                    dossier_label = EXCLUDED.dossier_label,
                    updated_at = NOW()",
            )
            .bind(&uids)
            .bind(batch.iter().map(|s| s.number()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.legislature() as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.date()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.session_ref()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.sitting_ref()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.place()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.ballot_type().code()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.ballot_type().label()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.ballot_type().majority()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.outcome().code()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.outcome().label()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.requester()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.subject()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().voters as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().expressed as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().required as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().announcement.as_str()).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().tally.votes_for as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().tally.votes_against as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().tally.abstentions as i16).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.synthesis().tally.not_voting as i16).collect::<Vec<_>>())
            .bind(
                batch
                    .iter()
                    .map(|s| s.synthesis().tally.voluntary_not_voting as i16)
                    .collect::<Vec<_>>(),
            )
            .bind(batch.iter().map(|s| s.dossier().map(|d| d.uid.as_str())).collect::<Vec<_>>())
            .bind(batch.iter().map(|s| s.dossier().map(|d| d.label.as_str())).collect::<Vec<_>>())
            .execute(&mut *tx)
            .await
            .map_err(db)?;

            let tallies: Vec<(&str, &GroupTally)> = batch
                .iter()
                .flat_map(|s| s.group_tallies().iter().map(move |t| (s.uid().as_str(), t)))
                .collect();
            for chunk in tallies.chunks(ROW_BATCH) {
                sqlx::query(
                    "INSERT INTO scrutin_group_tallies (
                        scrutin_uid, group_uid, member_count, majority_position,
                        votes_for, votes_against, abstentions, not_voting, voluntary_not_voting, origin
                     )
                     SELECT * FROM UNNEST(
                        $1::text[], $2::text[], $3::smallint[], $4::text[],
                        $5::smallint[], $6::smallint[], $7::smallint[], $8::smallint[], $9::smallint[], $10::text[]
                     )",
                )
                .bind(chunk.iter().map(|(uid, _)| *uid).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, t)| t.group_uid.as_str()).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, t)| t.member_count.map(|c| c as i16)).collect::<Vec<_>>())
                .bind(
                    chunk
                        .iter()
                        .map(|(_, t)| t.majority_position.map(|p| p.as_str()))
                        .collect::<Vec<_>>(),
                )
                .bind(chunk.iter().map(|(_, t)| t.tally.votes_for as i16).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, t)| t.tally.votes_against as i16).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, t)| t.tally.abstentions as i16).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, t)| t.tally.not_voting as i16).collect::<Vec<_>>())
                .bind(
                    chunk
                        .iter()
                        .map(|(_, t)| t.tally.voluntary_not_voting as i16)
                        .collect::<Vec<_>>(),
                )
                .bind(chunk.iter().map(|(_, t)| t.origin.as_str()).collect::<Vec<_>>())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            let votes: Vec<(&str, &NominalVote)> = batch
                .iter()
                .flat_map(|s| s.nominal_votes().iter().map(move |v| (s.uid().as_str(), v)))
                .collect();
            for chunk in votes.chunks(ROW_BATCH) {
                sqlx::query(
                    "INSERT INTO scrutin_votes (
                        scrutin_uid, actor_uid, group_uid, position, cause_code, by_delegation, seat
                     )
                     SELECT * FROM UNNEST(
                        $1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::bool[], $7::smallint[]
                     )",
                )
                .bind(chunk.iter().map(|(uid, _)| *uid).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, v)| v.actor_uid.as_str()).collect::<Vec<_>>())
                .bind(
                    chunk
                        .iter()
                        .map(|(_, v)| v.group_uid.as_ref().map(|g| g.as_str()))
                        .collect::<Vec<_>>(),
                )
                .bind(chunk.iter().map(|(_, v)| v.position.as_str()).collect::<Vec<_>>())
                .bind(
                    chunk
                        .iter()
                        .map(|(_, v)| v.cause.as_ref().map(|c| c.as_str()))
                        .collect::<Vec<_>>(),
                )
                .bind(chunk.iter().map(|(_, v)| v.by_delegation).collect::<Vec<_>>())
                .bind(chunk.iter().map(|(_, v)| v.seat.map(|s| s as i16)).collect::<Vec<_>>())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            let corrections: Vec<(&str, &VoteCorrection)> = batch
                .iter()
                .flat_map(|s| s.corrections().iter().map(move |c| (s.uid().as_str(), c)))
                .collect();
            for chunk in corrections.chunks(ROW_BATCH) {
                sqlx::query(
                    "INSERT INTO scrutin_vote_corrections (
                        scrutin_uid, actor_uid, claimed_position, malfunction
                     )
                     SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::bool[])",
                )
                .bind(chunk.iter().map(|(uid, _)| *uid).collect::<Vec<_>>())
                .bind(
                    chunk
                        .iter()
                        .map(|(_, c)| c.actor_uid.as_str())
                        .collect::<Vec<_>>(),
                )
                .bind(
                    chunk
                        .iter()
                        .map(|(_, c)| c.claimed_position.as_str())
                        .collect::<Vec<_>>(),
                )
                .bind(chunk.iter().map(|(_, c)| c.malfunction).collect::<Vec<_>>())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            tx.commit().await.map_err(db)?;
            written += batch.len();
        }

        Ok(written)
    }

    async fn list(&self, filter: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
        let mut count_query: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM scrutins s");
        push_conditions(&mut count_query, filter);
        let total: i64 = count_query
            .build()
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?
            .get(0);

        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(SUMMARY_SELECT);
        push_conditions(&mut query, filter);
        query.push(" ORDER BY s.scrutin_date DESC, s.uid DESC LIMIT ");
        query.push_bind(filter.limit.clamp(PAGE_LIMIT_MIN, PAGE_LIMIT_MAX));
        query.push(" OFFSET ");
        query.push_bind(filter.offset.max(0));

        let rows = query
            .build()
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        Ok(ScrutinPage {
            items: rows.iter().map(summary_from_row).collect(),
            total,
        })
    }

    async fn by_uid(&self, uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
        let Some(row) = sqlx::query(
            "SELECT uid, number, legislature, scrutin_date, session_ref, sitting_ref, place,
                    ballot_type_code, ballot_type_label, majority_label,
                    outcome_code, outcome_label, requester, subject,
                    voters, expressed, required, announcement,
                    votes_for, votes_against, abstentions, not_voting, voluntary_not_voting,
                    dossier_uid, dossier_label
             FROM scrutins WHERE uid = $1",
        )
        .persistent(false)
        .bind(uid.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };

        let tally_rows = sqlx::query(
            "SELECT group_uid, member_count, majority_position,
                    votes_for, votes_against, abstentions, not_voting, voluntary_not_voting, origin
             FROM scrutin_group_tallies WHERE scrutin_uid = $1
             ORDER BY votes_for + votes_against + abstentions + not_voting DESC, group_uid",
        )
        .persistent(false)
        .bind(uid.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let vote_rows = sqlx::query(
            "SELECT actor_uid, group_uid, position, cause_code, by_delegation, seat
             FROM scrutin_votes WHERE scrutin_uid = $1 ORDER BY actor_uid",
        )
        .persistent(false)
        .bind(uid.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let correction_rows = sqlx::query(
            "SELECT actor_uid, claimed_position, malfunction
             FROM scrutin_vote_corrections WHERE scrutin_uid = $1 ORDER BY actor_uid",
        )
        .persistent(false)
        .bind(uid.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let group_tallies = tally_rows
            .iter()
            .filter_map(|r| {
                Some(GroupTally {
                    group_uid: GroupUid::new(r.get::<String, _>("group_uid")).ok()?,
                    member_count: r.get::<Option<i16>, _>("member_count").map(|c| c as u16),
                    majority_position: r
                        .get::<Option<String>, _>("majority_position")
                        .and_then(|p| VotePosition::parse(&p)),
                    tally: tally_from_row(r),
                    origin: TallyOrigin::parse(&r.get::<String, _>("origin"))
                        .unwrap_or(TallyOrigin::Published),
                })
            })
            .collect();

        let nominal_votes = vote_rows
            .iter()
            .filter_map(|r| {
                Some(NominalVote {
                    actor_uid: ActorUid::new(r.get::<String, _>("actor_uid")).ok()?,
                    group_uid: r
                        .get::<Option<String>, _>("group_uid")
                        .and_then(|g| GroupUid::new(g).ok()),
                    position: VotePosition::parse(&r.get::<String, _>("position"))?,
                    cause: r
                        .get::<Option<String>, _>("cause_code")
                        .and_then(|c| NonVotingCause::new(c).ok()),
                    by_delegation: r.get("by_delegation"),
                    seat: r.get::<Option<i16>, _>("seat").map(|s| s as u16),
                })
            })
            .collect();

        let corrections = correction_rows
            .iter()
            .filter_map(|r| {
                Some(VoteCorrection {
                    actor_uid: ActorUid::new(r.get::<String, _>("actor_uid")).ok()?,
                    claimed_position: VotePosition::parse(&r.get::<String, _>("claimed_position"))?,
                    malfunction: r.get("malfunction"),
                })
            })
            .collect();

        let scrutin = Scrutin::new(NewScrutin {
            uid: ScrutinUid::new(row.get::<String, _>("uid")).map_err(|e| {
                RepositoryError::Database(format!("stored scrutin has an unusable uid: {e}"))
            })?,
            number: row.get("number"),
            legislature: row.get::<i16, _>("legislature") as u16,
            date: row.get("scrutin_date"),
            session_ref: row.get("session_ref"),
            sitting_ref: row.get("sitting_ref"),
            place: row.get("place"),
            ballot_type: BallotType::new(
                row.get("ballot_type_code"),
                row.get("ballot_type_label"),
                row.get("majority_label"),
            )
            .map_err(|e| RepositoryError::Database(e.to_string()))?,
            outcome: Outcome::new(row.get("outcome_code"), row.get("outcome_label"))
                .map_err(|e| RepositoryError::Database(e.to_string()))?,
            requester: row.get("requester"),
            subject: row.get("subject"),
            synthesis: VoteSynthesis {
                voters: row.get::<i16, _>("voters") as u16,
                expressed: row.get::<i16, _>("expressed") as u16,
                required: row.get::<i16, _>("required") as u16,
                announcement: row.get("announcement"),
                tally: tally_from_row(&row),
            },
            group_tallies,
            nominal_votes,
            corrections,
            dossier: match (
                row.get::<Option<String>, _>("dossier_uid"),
                row.get::<Option<String>, _>("dossier_label"),
            ) {
                (Some(uid), label) => Some(DossierReference {
                    uid: DossierUid::new(uid).map_err(|e| {
                        RepositoryError::Database(format!(
                            "stored scrutin has an unusable dossier uid: {e}"
                        ))
                    })?,
                    label: label.unwrap_or_default(),
                }),
                (None, _) => None,
            },
        })
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(Some(scrutin))
    }

    async fn by_dossier(&self, dossier_uid: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(SUMMARY_SELECT);
        query.push(" WHERE s.dossier_uid = ");
        query.push_bind(dossier_uid.to_string());
        query.push(" ORDER BY s.scrutin_date DESC, s.uid DESC");

        let rows = query
            .build()
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        Ok(rows.iter().map(summary_from_row).collect())
    }

    async fn between(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(SUMMARY_SELECT);
        query.push(" WHERE s.scrutin_date BETWEEN ");
        query.push_bind(from);
        query.push(" AND ");
        query.push_bind(to);
        query.push(" ORDER BY s.scrutin_date, length(s.number), s.number");

        let rows = query
            .build()
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        Ok(rows.iter().map(summary_from_row).collect())
    }

    async fn count_between(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<i64, RepositoryError> {
        sqlx::query_scalar("SELECT count(*) FROM scrutins WHERE scrutin_date BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)
    }

    async fn latest_scrutin_date(&self) -> Result<Option<chrono::NaiveDate>, RepositoryError> {
        sqlx::query_scalar("SELECT max(scrutin_date) FROM scrutins")
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)
    }

    async fn sitting_keys(
        &self,
    ) -> Result<Vec<crate::domain::sitting_record::ScrutinSittingKey>, RepositoryError> {
        use crate::domain::sitting_record::{AnnouncedResult, ScrutinSittingKey, SittingRef};

        let rows = sqlx::query(
            "SELECT uid, sitting_ref, subject, voters, expressed, votes_for, votes_against
             FROM scrutins ORDER BY uid",
        )
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut keys = Vec::new();
        for row in &rows {
            let uid = row.get::<String, _>("uid");
            let Some(sitting_ref) = row
                .get::<Option<String>, _>("sitting_ref")
                .and_then(|raw| SittingRef::new(raw).ok())
            else {
                continue;
            };
            let Ok(uid) = ScrutinUid::new(uid) else {
                tracing::warn!("Scrutin with a blank uid skipped for sitting keys");
                continue;
            };
            keys.push(ScrutinSittingKey::new(
                uid,
                sitting_ref,
                AnnouncedResult {
                    voters: u32::from(row.get::<i16, _>("voters") as u16),
                    expressed: u32::from(row.get::<i16, _>("expressed") as u16),
                    votes_for: u32::from(row.get::<i16, _>("votes_for") as u16),
                    votes_against: u32::from(row.get::<i16, _>("votes_against") as u16),
                },
                &row.get::<String, _>("subject"),
            ));
        }
        Ok(keys)
    }

    async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
        let totals = sqlx::query(
            "SELECT count(*) AS scrutins_total,
                    count(*) FILTER (WHERE dossier_uid IS NOT NULL) AS with_dossier,
                    min(scrutin_date) AS first_date,
                    max(scrutin_date) AS last_date
             FROM scrutins",
        )
        .persistent(false)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;

        let legislatures =
            sqlx::query("SELECT DISTINCT legislature FROM scrutins ORDER BY legislature")
                .persistent(false)
                .fetch_all(&self.pool)
                .await
                .map_err(db)?
                .iter()
                .map(|row| row.get::<i16, _>("legislature") as i64)
                .collect();

        let dossiers_total: i64 = sqlx::query("SELECT count(*) FROM legislative_dossiers")
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?
            .try_get(0)
            .map_err(db)?;

        Ok(DatasetShape {
            scrutins_total: totals.get("scrutins_total"),
            scrutins_with_dossier: totals.get("with_dossier"),
            first_scrutin_date: totals.get("first_date"),
            last_scrutin_date: totals.get("last_date"),
            legislatures,
            outcomes: self.code_counts("outcome_code", "outcome_label").await?,
            ballot_types: self
                .code_counts("ballot_type_code", "ballot_type_label")
                .await?,
            dossiers_total,
        })
    }
}

const SUMMARY_SELECT: &str = "SELECT s.uid, s.number, s.legislature, s.scrutin_date, s.subject,
        s.ballot_type_label, s.outcome_code, s.outcome_label,
        s.votes_for, s.votes_against, s.abstentions, s.not_voting, s.voluntary_not_voting,
        s.dossier_uid, s.dossier_label,
        EXISTS (
            SELECT 1 FROM scrutin_group_tallies t
            WHERE t.scrutin_uid = s.uid AND t.origin = 'reconstructed'
        ) AS has_reconstructed
     FROM scrutins s";

fn push_conditions(query: &mut QueryBuilder<'_, Postgres>, filter: &ScrutinFilter) {
    let mut separated = false;
    let open = |q: &mut QueryBuilder<'_, Postgres>, separated: &mut bool| {
        q.push(if *separated { " AND " } else { " WHERE " });
        *separated = true;
    };

    if let Some(from) = filter.from {
        open(query, &mut separated);
        query.push("s.scrutin_date >= ");
        query.push_bind(from);
    }
    if let Some(to) = filter.to {
        open(query, &mut separated);
        query.push("s.scrutin_date <= ");
        query.push_bind(to);
    }
    if let Some(code) = filter.outcome_code.as_ref() {
        open(query, &mut separated);
        query.push("s.outcome_code = ");
        query.push_bind(code.clone());
    }
    if let Some(code) = filter.ballot_type_code.as_ref() {
        open(query, &mut separated);
        query.push("s.ballot_type_code = ");
        query.push_bind(code.clone());
    }
    if let Some(with_dossier) = filter.with_dossier {
        open(query, &mut separated);
        query.push(if with_dossier {
            "s.dossier_uid IS NOT NULL"
        } else {
            "s.dossier_uid IS NULL"
        });
    }
    if let Some(dossier_uid) = filter.dossier_uid.as_ref() {
        open(query, &mut separated);
        query.push("s.dossier_uid = ");
        query.push_bind(dossier_uid.clone());
    }
    if let Some(search) = filter.search.as_ref() {
        open(query, &mut separated);
        query.push("s.subject ILIKE ");
        query.push_bind(format!("%{search}%"));
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

fn summary_from_row(row: &sqlx::postgres::PgRow) -> ScrutinSummary {
    ScrutinSummary {
        uid: row.get("uid"),
        number: row.get("number"),
        legislature: row.get::<i16, _>("legislature") as u16,
        date: row.get::<NaiveDate, _>("scrutin_date"),
        subject: row.get("subject"),
        ballot_type_label: row.get("ballot_type_label"),
        outcome_code: row.get("outcome_code"),
        outcome_label: row.get("outcome_label"),
        tally: tally_from_row(row),
        dossier_uid: row.get("dossier_uid"),
        dossier_label: row.get("dossier_label"),
        has_reconstructed_tallies: row.get("has_reconstructed"),
    }
}
