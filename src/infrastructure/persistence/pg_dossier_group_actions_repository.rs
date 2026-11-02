use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use super::{db, FINAL_VOTE_PREDICATE};
use crate::application::ports::dossier_group_actions_repository::{
    DossierGroupActionsRepository, RepositoryError,
};
use crate::domain::actor::GroupUid;
use crate::domain::amendment::AmendmentUid;
use crate::domain::dossier::{DossierPeriod, DossierUid};
use crate::domain::dossier_group_facts::{
    AmendmentFact, DossierGroupFacts, FinalVoteFact, GroupFacts,
};
use crate::domain::final_vote::reading_of;
use crate::domain::scrutin::{ScrutinUid, VoteTally};

pub struct PgDossierGroupActionsRepository {
    pool: PgPool,
}

impl PgDossierGroupActionsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn load_one(
        &self,
        dossier_uid: &str,
    ) -> Result<Option<DossierGroupFacts>, RepositoryError> {
        let Some(dossier) = sqlx::query(
            "SELECT uid, title, legislature, url, deposit_date, last_activity_date
             FROM legislative_dossiers WHERE uid = $1",
        )
        .bind(dossier_uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };

        let legislature: i16 = dossier.get("legislature");
        let deposit_date: Option<NaiveDate> = dossier.get("deposit_date");
        let last_activity_date: NaiveDate = dossier.get("last_activity_date");
        let document_dates = sqlx::query(
            "SELECT min(doc_date) AS first_date FROM dossier_documents WHERE dossier_uid = $1",
        )
        .bind(dossier_uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        let first_document_date: Option<NaiveDate> = document_dates.get("first_date");
        let act_dates = sqlx::query(
            "SELECT min(act_date) AS first_date FROM legislative_acts WHERE dossier_uid = $1",
        )
        .bind(dossier_uid)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        let first_act_date: Option<NaiveDate> = act_dates.get("first_date");
        let period = DossierPeriod::from_dates(
            deposit_date,
            first_document_date,
            first_act_date,
            last_activity_date,
        );

        let group_rows = sqlx::query(
            "SELECT uid, abbrev, label, color, start_date, end_date
               FROM parliamentary_groups
              WHERE legislature = $1
                AND (start_date IS NULL OR start_date <= $2)
                AND (end_date IS NULL OR end_date >= $3)
              ORDER BY start_date NULLS FIRST, abbrev, uid",
        )
        .bind(legislature)
        .bind(period.end())
        .bind(period.start())
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut groups: HashMap<String, GroupFacts> = group_rows
            .iter()
            .filter_map(|row| {
                let raw_uid: String = row.get("uid");
                let uid = match GroupUid::new(raw_uid.clone()) {
                    Ok(uid) => uid,
                    Err(error) => {
                        tracing::warn!("Skipping a stored group with an unusable uid: {error}");
                        return None;
                    }
                };
                Some((
                    raw_uid,
                    GroupFacts {
                        uid,
                        abbrev: row.get("abbrev"),
                        label: row.get("label"),
                        color: row.get("color"),
                        start_date: row.get("start_date"),
                        end_date: row.get("end_date"),
                        final_votes: Vec::new(),
                        amendments: Vec::new(),
                    },
                ))
            })
            .collect();

        let vote_rows = sqlx::query(&format!(
            "SELECT s.uid, s.number, s.scrutin_date, s.legislature, s.subject,
                    s.outcome_code, s.outcome_label,
                    dt.label AS text_label,
                    t.group_uid, t.member_count, t.majority_position,
                    t.votes_for, t.votes_against, t.abstentions,
                    t.not_voting, t.voluntary_not_voting
               FROM scrutins s
               JOIN scrutin_debated_texts sdt ON sdt.scrutin_uid = s.uid
               JOIN debated_texts dt ON dt.text_key = sdt.text_key
               JOIN scrutin_group_tallies t ON t.scrutin_uid = s.uid
              WHERE s.dossier_uid = $1 AND {FINAL_VOTE_PREDICATE}
              ORDER BY s.scrutin_date, s.number, t.group_uid"
        ))
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        for row in vote_rows {
            let group_uid: String = row.get("group_uid");
            let Some(group) = groups.get_mut(&group_uid) else {
                continue;
            };
            let scrutin_uid = match ScrutinUid::new(row.get("uid")) {
                Ok(uid) => uid,
                Err(error) => {
                    tracing::warn!("Skipping a stored final vote with an unusable uid: {error}");
                    continue;
                }
            };
            let subject: String = row.get("subject");
            let text_label: String = row.get("text_label");
            group.final_votes.push(FinalVoteFact {
                scrutin_uid,
                number: row.get("number"),
                date: row.get("scrutin_date"),
                legislature: row.get::<i16, _>("legislature") as u16,
                reading: reading_of(&subject, &text_label),
                subject,
                text_label,
                outcome_code: row.get("outcome_code"),
                outcome_label: row.get("outcome_label"),
                majority_position: row.get("majority_position"),
                member_count: row.get::<Option<i16>, _>("member_count").map(|v| v as u16),
                tally: VoteTally {
                    votes_for: row.get::<i16, _>("votes_for") as u16,
                    votes_against: row.get::<i16, _>("votes_against") as u16,
                    abstentions: row.get::<i16, _>("abstentions") as u16,
                    not_voting: row.get::<i16, _>("not_voting") as u16,
                    voluntary_not_voting: row.get::<i16, _>("voluntary_not_voting") as u16,
                },
            });
        }

        let amendment_rows = sqlx::query(
            "SELECT DISTINCT ON (a.uid) a.uid, a.number, a.target_title, a.target_kind,
                    a.author_group_uid, a.fate_code, a.fate_label, a.deposited_on,
                    (a.summary IS NOT NULL) AS summary_available
               FROM amendments a
               JOIN dossier_documents d ON d.document_uid = a.text_ref
              WHERE d.dossier_uid = $1
              ORDER BY a.uid, d.id",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        for row in amendment_rows {
            let Some(group_uid) = row.get::<Option<String>, _>("author_group_uid") else {
                continue;
            };
            let Some(group) = groups.get_mut(&group_uid) else {
                continue;
            };
            let uid = match AmendmentUid::new(row.get("uid")) {
                Ok(uid) => uid,
                Err(error) => {
                    tracing::warn!("Skipping a stored amendment with an unusable uid: {error}");
                    continue;
                }
            };
            group.amendments.push(AmendmentFact {
                uid,
                number: row.get("number"),
                target_title: row.get("target_title"),
                target_kind: row.get("target_kind"),
                fate_code: row.get("fate_code"),
                fate_label: row.get("fate_label"),
                deposited_on: row.get("deposited_on"),
                summary_available: row.get("summary_available"),
            });
        }

        let mut groups: Vec<GroupFacts> = groups.into_values().collect();
        groups.sort_by(|left, right| {
            left.start_date
                .cmp(&right.start_date)
                .then_with(|| left.abbrev.cmp(&right.abbrev))
                .then_with(|| left.uid.cmp(&right.uid))
        });

        let dossier_uid = DossierUid::new(dossier.get("uid")).map_err(|error| {
            RepositoryError::Database(format!("stored dossier has an unusable uid: {error}"))
        })?;

        Ok(Some(DossierGroupFacts {
            dossier_uid,
            title: dossier.get("title"),
            official_url: dossier.get("url"),
            legislature: legislature as u16,
            period_start: Some(period.start()),
            period_end: Some(period.end()),
            groups,
        }))
    }
}

#[async_trait]
impl DossierGroupActionsRepository for PgDossierGroupActionsRepository {
    async fn load_facts(
        &self,
        dossier_uid: &str,
    ) -> Result<Option<DossierGroupFacts>, RepositoryError> {
        self.load_one(dossier_uid).await
    }

    async fn list_facts(&self, limit: usize) -> Result<Vec<DossierGroupFacts>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT uid FROM legislative_dossiers ORDER BY last_activity_date DESC, uid LIMIT $1",
        )
        .bind(limit.max(1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut facts = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(value) = self.load_one(&row.get::<String, _>("uid")).await? {
                facts.push(value);
            }
        }
        Ok(facts)
    }
}
