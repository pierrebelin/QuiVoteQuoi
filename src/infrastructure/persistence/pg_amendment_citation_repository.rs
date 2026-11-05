//! Depot Postgres des citations d'amendement.

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::amendment_citation_repository::{
    AmendmentCitationRepository, AmendmentVote, CitationRow, CitedAmendmentRow, CitingScrutin, StoredCitation,
};
use crate::application::ports::amendment_repository::AmendmentSummary;
use crate::application::ports::RepositoryError;
use crate::domain::amendment::{AmendmentNumber, AmendmentTarget, AmendmentUid};
use crate::domain::amendment_citation::{
    CandidateAmendment, CandidateAuthor, CitationCount, CitationResult, LinkMethod, UnlinkedCause,
};
use crate::domain::dossier::DossierUid;
use crate::domain::scrutin::{ScrutinUid, VoteTally};

/// Lignes par instruction `UNNEST`.
const ROW_BATCH: usize = 4000;

pub struct PgAmendmentCitationRepository {
    pool: PgPool,
}

impl PgAmendmentCitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn invalid<E: std::fmt::Display>(error: E) -> RepositoryError {
    RepositoryError::Database(error.to_string())
}

#[async_trait]
impl AmendmentCitationRepository for PgAmendmentCitationRepository {
    async fn scrutins_of_amendments(
        &self,
        amendments: &[AmendmentUid],
    ) -> Result<Vec<AmendmentVote>, RepositoryError> {
        if amendments.is_empty() {
            return Ok(vec![]);
        }
        let uids: Vec<&str> = amendments.iter().map(AmendmentUid::as_str).collect();
        let rows = sqlx::query(
            "SELECT DISTINCT c.amendment_uid, s.uid, s.number, s.scrutin_date,
                    s.votes_for, s.votes_against, s.abstentions, s.not_voting,
                    s.voluntary_not_voting
             FROM scrutin_amendment_citations c
             JOIN scrutins s ON s.uid = c.scrutin_uid
             WHERE c.amendment_uid = ANY($1)
             ORDER BY s.scrutin_date, s.number, s.uid",
        )
        .bind(&uids)
        .fetch_all(&self.pool)
        .await
        .map_err(invalid)?;

        let mut votes = Vec::with_capacity(rows.len());
        for row in rows {
            let amendment = AmendmentUid::new(row.get("amendment_uid")).map_err(invalid)?;
            let scrutin_uid = ScrutinUid::new(row.get("uid")).map_err(invalid)?;
            votes.push(AmendmentVote {
                amendment,
                scrutin_uid,
                scrutin_number: row.get("number"),
                date: row.get("scrutin_date"),
                tally: VoteTally {
                    votes_for: row.get::<i16, _>("votes_for") as u16,
                    votes_against: row.get::<i16, _>("votes_against") as u16,
                    abstentions: row.get::<i16, _>("abstentions") as u16,
                    not_voting: row.get::<i16, _>("not_voting") as u16,
                    voluntary_not_voting: row.get::<i16, _>("voluntary_not_voting") as u16,
                },
            });
        }
        Ok(votes)
    }

    async fn citing_scrutins(&self) -> Result<Vec<CitingScrutin>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT uid, scrutin_date, subject, coalesce(dossier_uid, '') AS dossier_uid
             FROM scrutins ORDER BY uid",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter()
            .map(|row| {
                let uid = ScrutinUid::new(row.get("uid")).map_err(invalid)?;
                let date: NaiveDate = row.get("scrutin_date");
                let dossier: String = row.get("dossier_uid");
                Ok(citing_scrutin_from_row(uid, date, row.get("subject"), &dossier))
            })
            .collect()
    }

    async fn candidates_in(
        &self,
        dossiers: &[DossierUid],
    ) -> Result<Vec<CandidateAmendment>, RepositoryError> {
        if dossiers.is_empty() {
            return Ok(Vec::new());
        }
        let uids: Vec<&str> = dossiers.iter().map(|d| d.as_str()).collect();
        let rows = sqlx::query(
            "SELECT DISTINCT a.uid, d.dossier_uid, a.number, a.target_title, a.target_kind,
                    a.deposited_on, a.author_label,
                    NULLIF(TRIM(ac.first_name || ' ' || ac.last_name), '') AS author_full_name
             FROM amendments a
             JOIN dossier_documents d ON d.document_uid = a.text_ref
             LEFT JOIN actors ac ON ac.uid = a.author_actor_uid
             WHERE d.dossier_uid = ANY($1::text[])",
        )
        .bind(&uids)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter()
            .map(|row| {
                let target = AmendmentTarget::new(row.get("target_title"), row.get("target_kind"))
                    .map_err(invalid)?;
                // `author_label` n'est renseigne que pour un auteur institutionnel
                // (contrainte `amendments_author_label`).
                let author = match row.get::<Option<String>, _>("author_label") {
                    Some(label) => CandidateAuthor::Institutional { label },
                    None => CandidateAuthor::Deputy {
                        name: row.get("author_full_name"),
                    },
                };
                Ok(CandidateAmendment::new(
                    AmendmentUid::new(row.get("uid")).map_err(invalid)?,
                    DossierUid::new(row.get("dossier_uid")).map_err(invalid)?,
                    AmendmentNumber::new(row.get("number")).map_err(invalid)?,
                    &target,
                    row.get("deposited_on"),
                    author,
                ))
            })
            .collect()
    }

    async fn replace_citations(
        &self,
        citations: &[StoredCitation],
    ) -> Result<usize, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("DELETE FROM scrutin_amendment_citations")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for batch in citations.chunks(ROW_BATCH) {
            let scrutins: Vec<&str> = batch.iter().map(|c| c.scrutin.as_str()).collect();
            let ranks: Vec<i16> = batch.iter().map(|c| c.outcome.rank as i16).collect();
            let numbers: Vec<Option<&str>> = batch
                .iter()
                .map(|c| c.outcome.cited_number.as_ref().map(|n| n.as_str()))
                .collect();
            let amendments: Vec<Option<&str>> = batch
                .iter()
                .map(|c| match &c.outcome.result {
                    CitationResult::Linked { amendment, .. } => Some(amendment.as_str()),
                    CitationResult::Unlinked(_) => None,
                })
                .collect();
            let causes: Vec<Option<&str>> = batch
                .iter()
                .map(|c| match &c.outcome.result {
                    CitationResult::Linked { .. } => None,
                    CitationResult::Unlinked(cause) => Some(cause.as_str()),
                })
                .collect();
            let by_article: Vec<bool> = batch
                .iter()
                .map(|c| match &c.outcome.result {
                    CitationResult::Linked { method, .. } => method.by_cited_article,
                    CitationResult::Unlinked(_) => false,
                })
                .collect();
            sqlx::query(
                "INSERT INTO scrutin_amendment_citations
                     (scrutin_uid, rank, cited_number, amendment_uid, cause, by_cited_article)
                 SELECT * FROM UNNEST($1::text[], $2::smallint[], $3::text[], $4::text[],
                                      $5::text[], $6::boolean[])",
            )
            .bind(&scrutins)
            .bind(&ranks)
            .bind(&numbers)
            .bind(&amendments)
            .bind(&causes)
            .bind(&by_article)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(citations.len())
    }

    async fn citation_counts(
        &self,
    ) -> Result<Vec<CitationCount>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT cause, by_cited_article, COUNT(*) AS total
             FROM scrutin_amendment_citations
             GROUP BY cause, by_cited_article",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter()
            .map(|row| {
                let raw: Option<String> = row.get("cause");
                let cause = match raw {
                    None => None,
                    Some(raw) => Some(UnlinkedCause::parse(&raw).ok_or_else(|| {
                        RepositoryError::Database(format!("cause inconnue: {raw}"))
                    })?),
                };
                Ok(CitationCount {
                    cause,
                    by_cited_article: row.get("by_cited_article"),
                    count: row.get("total"),
                })
            })
            .collect()
    }

    async fn citations_of(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<CitationRow>, RepositoryError> {
        if scrutins.is_empty() {
            return Ok(Vec::new());
        }
        let uids: Vec<&str> = scrutins.iter().map(|s| s.as_str()).collect();
        let rows = sqlx::query(
            "SELECT c.scrutin_uid, c.rank, c.cited_number, c.cause, c.by_cited_article,
                    coalesce(s.dossier_uid, '') AS dossier_uid,
                    a.uid AS a_uid, a.number, a.target_title, a.target_kind,
                    a.author_kind, a.author_actor_uid, a.author_label,
                    a.author_group_uid, a.author_group_origin, a.author_group_ambiguous,
                    a.fate_code, a.fate_label, a.state_label, a.deposited_on, a.summary,
                    (SELECT count(*) FROM amendment_signatories g
                      WHERE g.amendment_uid = a.uid AND g.role = 'cosignatory') AS cosignatory_count
             FROM scrutin_amendment_citations c
             JOIN scrutins s ON s.uid = c.scrutin_uid
             LEFT JOIN amendments a ON a.uid = c.amendment_uid
             WHERE c.scrutin_uid = ANY($1::text[])
             ORDER BY c.scrutin_uid, c.rank",
        )
        .bind(&uids)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        rows.iter().map(citation_row_from).collect()
    }
}

fn citation_row_from(row: &sqlx::postgres::PgRow) -> Result<CitationRow, RepositoryError> {
    let cause = match row.get::<Option<String>, _>("cause") {
        None => None,
        Some(raw) => Some(
            UnlinkedCause::parse(&raw)
                .ok_or_else(|| RepositoryError::Database(format!("cause inconnue: {raw}")))?,
        ),
    };
    let amendment = match row.get::<Option<String>, _>("a_uid") {
        None => None,
        Some(uid) => Some(CitedAmendmentRow {
            dossier_uid: DossierUid::new(row.get("dossier_uid")).map_err(invalid)?,
            summary: AmendmentSummary {
                uid,
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
            },
        }),
    };
    let rank: i16 = row.get("rank");
    Ok(CitationRow {
        scrutin: ScrutinUid::new(row.get("scrutin_uid")).map_err(invalid)?,
        rank: u16::try_from(rank).map_err(invalid)?,
        cited_number: row.get("cited_number"),
        amendment,
        cause,
        method: LinkMethod {
            by_cited_article: row.get("by_cited_article"),
        },
    })
}

pub(crate) fn citing_scrutin_from_row(
    uid: ScrutinUid,
    date: NaiveDate,
    subject: String,
    dossier_uid: &str,
) -> CitingScrutin {
    // Dossier vide: la source n'en rattache aucun, ce n'est pas une erreur.
    let published_dossier = DossierUid::new(dossier_uid.to_string()).ok();
    if published_dossier.is_none() {
        tracing::warn!(scrutin = %uid, "scrutin carries no published dossier");
    }
    CitingScrutin {
        uid,
        date,
        subject,
        published_dossier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scrutin_uid() -> ScrutinUid {
        ScrutinUid::new("VTANR5L17V490".to_string()).unwrap()
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 12).unwrap()
    }

    #[test]
    fn should_keep_published_dossier_when_row_carries_one() {
        let citing = citing_scrutin_from_row(
            scrutin_uid(),
            day(),
            "l'amendement n° 12 de M. Martin".to_string(),
            "DLR5L17N1",
        );
        assert_eq!(
            citing,
            CitingScrutin {
                uid: scrutin_uid(),
                date: day(),
                subject: "l'amendement n° 12 de M. Martin".to_string(),
                published_dossier: Some(DossierUid::new("DLR5L17N1".to_string()).unwrap()),
            }
        );
    }

    #[test]
    fn should_read_no_published_dossier_when_row_carries_an_empty_one() {
        let citing = citing_scrutin_from_row(
            scrutin_uid(),
            day(),
            "l'amendement n° 12 de M. Martin".to_string(),
            "",
        );
        assert_eq!(citing.published_dossier, None);
        assert_eq!(citing.uid, scrutin_uid());
        assert_eq!(citing.subject, "l'amendement n° 12 de M. Martin");
    }
}
