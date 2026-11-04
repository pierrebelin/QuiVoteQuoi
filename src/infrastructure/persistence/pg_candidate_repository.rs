use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::candidate_repository::{
    CandidateParliamentaryGroupRecord, CandidateProgramProposalRecord, CandidateRecord,
    CandidateRepository, ChangedSourceRecord, PoliticalOrganizationRecord, RepositoryError,
};
use crate::domain::candidate::CandidateId;
use crate::domain::theme::FamilyCode;

pub struct PgCandidateRepository {
    pool: PgPool,
}

impl PgCandidateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn candidate_id(raw: String) -> Result<CandidateId, RepositoryError> {
    CandidateId::new(raw).map_err(|error| {
        RepositoryError::Database(format!("invalid candidate id in database: {error}"))
    })
}

#[async_trait]
impl CandidateRepository for PgCandidateRepository {
    async fn changed_sources(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<ChangedSourceRecord>, RepositoryError> {
        let rows = sqlx::query(
            "WITH usage AS (
                 SELECT declaration_source_url AS url, display_name FROM presidential_candidates
                 UNION SELECT official_site_url, display_name FROM presidential_candidates
                 UNION SELECT program_url, display_name FROM presidential_candidates
                 UNION SELECT link.source_url, candidate.display_name
                         FROM candidate_political_organizations link
                         JOIN presidential_candidates candidate ON candidate.id = link.candidate_id
                 UNION SELECT proposal.source_url, candidate.display_name
                         FROM candidate_program_proposals proposal
                         JOIN presidential_candidates candidate ON candidate.id = proposal.candidate_id
             )
             SELECT observation.source_url, observation.last_changed_at,
                    COALESCE(array_agg(DISTINCT usage.display_name)
                             FILTER (WHERE usage.display_name IS NOT NULL), '{}') AS candidate_names
               FROM candidate_program_source_observations observation
               LEFT JOIN usage ON usage.url = observation.source_url
              WHERE observation.last_changed_at > observation.first_observed_at
                AND observation.last_changed_at > $1
              GROUP BY observation.source_url, observation.last_changed_at
              ORDER BY observation.last_changed_at DESC, observation.source_url",
        )
        .persistent(false)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.into_iter()
            .map(|row| {
                Ok(ChangedSourceRecord {
                    source_url: row.try_get("source_url").map_err(db)?,
                    changed_at: row.try_get("last_changed_at").map_err(db)?,
                    candidate_names: row.try_get("candidate_names").map_err(db)?,
                })
            })
            .collect()
    }

    async fn list_candidates(&self) -> Result<Vec<CandidateRecord>, RepositoryError> {
        let candidate_rows = sqlx::query(
            "SELECT id, display_name, declared_on, declaration_source_url,
                    declaration_source_label, official_site_url, program_url
               FROM presidential_candidates
              ORDER BY declared_on, display_name, id",
        )
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let organization_rows = sqlx::query(
            "SELECT link.candidate_id, organization.label, organization.official_url,
                    link.source_url, link.source_label
               FROM candidate_political_organizations link
               JOIN political_organizations organization ON organization.id = link.organization_id
              ORDER BY link.candidate_id, organization.label",
        )
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        let mut organizations: HashMap<String, Vec<PoliticalOrganizationRecord>> = HashMap::new();
        for row in organization_rows {
            organizations
                .entry(row.get("candidate_id"))
                .or_default()
                .push(PoliticalOrganizationRecord {
                    label: row.get("label"),
                    official_url: row.get("official_url"),
                    source_url: row.get("source_url"),
                    source_label: row.get("source_label"),
                });
        }

        candidate_rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                Ok(CandidateRecord {
                    organizations: organizations.remove(&id).unwrap_or_default(),
                    id: candidate_id(id)?,
                    display_name: row.get("display_name"),
                    declared_on: row.get("declared_on"),
                    declaration_source_url: row.get("declaration_source_url"),
                    declaration_source_label: row.get("declaration_source_label"),
                    official_site_url: row.get("official_site_url"),
                    program_url: row.get("program_url"),
                })
            })
            .collect()
    }

    async fn program_proposals(
        &self,
        candidate_ids: &[CandidateId],
        family: Option<FamilyCode>,
    ) -> Result<Vec<CandidateProgramProposalRecord>, RepositoryError> {
        let ids: Vec<&str> = candidate_ids.iter().map(CandidateId::as_str).collect();
        let rows = sqlx::query(
            "SELECT candidate_id, family_code, excerpt, source_url, source_label, source_published_on
               FROM candidate_program_proposals
              WHERE candidate_id = ANY($1)
                AND ($2::text IS NULL OR family_code = $2)
              ORDER BY candidate_id, source_published_on NULLS LAST, id",
        )
        .bind(ids)
        .bind(family.map(|value| value.as_str()))
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.into_iter()
            .map(|row| {
                let raw_family: String = row.get("family_code");
                let family = FamilyCode::parse(&raw_family).map_err(|_| {
                    RepositoryError::Database(format!(
                        "unknown candidate proposal family in database: {raw_family}"
                    ))
                })?;
                Ok(CandidateProgramProposalRecord {
                    candidate_id: candidate_id(row.get("candidate_id"))?,
                    family,
                    excerpt: row.get("excerpt"),
                    source_url: row.get("source_url"),
                    source_label: row.get("source_label"),
                    source_published_on: row.get("source_published_on"),
                })
            })
            .collect()
    }

    async fn parliamentary_groups(
        &self,
        candidate_ids: &[CandidateId],
    ) -> Result<Vec<CandidateParliamentaryGroupRecord>, RepositoryError> {
        let ids: Vec<&str> = candidate_ids.iter().map(CandidateId::as_str).collect();
        let rows = sqlx::query(
            "SELECT link.candidate_id, link.group_uid, group_ref.abbrev, group_ref.label,
                    group_ref.color, link.linked_on, link.source_url, link.source_label
               FROM candidate_parliamentary_groups link
               JOIN parliamentary_groups group_ref ON group_ref.uid = link.group_uid
              WHERE link.candidate_id = ANY($1)
              ORDER BY link.candidate_id, group_ref.abbrev",
        )
        .bind(ids)
        .persistent(false)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        rows.into_iter()
            .map(|row| {
                Ok(CandidateParliamentaryGroupRecord {
                    candidate_id: candidate_id(row.get("candidate_id"))?,
                    group_uid: row.get("group_uid"),
                    abbrev: row.get("abbrev"),
                    label: row.get("label"),
                    color: row.get("color"),
                    linked_on: row.get("linked_on"),
                    source_url: row.get("source_url"),
                    source_label: row.get("source_label"),
                })
            })
            .collect()
    }
}
