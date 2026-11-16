use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use super::db;
use crate::application::ports::candidate_repository::{
    CandidateParliamentaryGroupRecord, CandidateProgramProposalRecord, CandidateRecord,
    CandidateRepository, ChangeExcerpt, ChangedSourceRecord, PoliticalOrganizationRecord,
    RepositoryError,
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

/// Extrait d'un changement, absent quand la lecture précédente n'avait pas de
/// texte gardé : les deux textes et les deux totaux sont alors nuls.
fn change_excerpt(row: &PgRow) -> Result<Option<ChangeExcerpt>, RepositoryError> {
    let added: Option<String> = row.try_get("last_added_text").map_err(db)?;
    let removed: Option<String> = row.try_get("last_removed_text").map_err(db)?;
    let added_total: Option<i32> = row.try_get("last_added_count").map_err(db)?;
    let removed_total: Option<i32> = row.try_get("last_removed_count").map_err(db)?;
    let (Some(added), Some(removed), Some(added_total), Some(removed_total)) =
        (added, removed, added_total, removed_total)
    else {
        return Ok(None);
    };
    Ok(Some(ChangeExcerpt {
        added: fragments(&added),
        removed: fragments(&removed),
        added_total: count(added_total)?,
        removed_total: count(removed_total)?,
    }))
}

fn fragments(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn count(raw: i32) -> Result<u32, RepositoryError> {
    u32::try_from(raw).map_err(|_| {
        RepositoryError::Database(format!("negative fragment count in database: {raw}"))
    })
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
            // Seules les sources de programme sont surveillées ; le site
            // officiel n'est lu que pour repérer un nouveau PDF. Une adresse
            // qu'aucune candidature n'utilise plus n'est pas une alerte.
            "WITH usage AS (
                 SELECT split_part(program_url, '#', 1) AS url, display_name
                   FROM presidential_candidates
                 UNION SELECT official_site_url, display_name FROM presidential_candidates
                 UNION SELECT split_part(proposal.source_url, '#', 1), candidate.display_name
                         FROM candidate_program_proposals proposal
                         JOIN presidential_candidates candidate ON candidate.id = proposal.candidate_id
             )
             SELECT observation.source_url, observation.last_changed_at,
                    array_agg(DISTINCT usage.display_name) AS candidate_names,
                    observation.last_added_text, observation.last_removed_text,
                    observation.last_added_count, observation.last_removed_count
               FROM candidate_program_source_observations observation
               JOIN usage ON usage.url = observation.source_url
              WHERE observation.last_changed_at > observation.first_observed_at
                AND observation.last_changed_at > $1
              GROUP BY observation.source_url, observation.last_changed_at,
                       observation.last_added_text, observation.last_removed_text,
                       observation.last_added_count, observation.last_removed_count
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
                    excerpt: change_excerpt(&row)?,
                })
            })
            .collect()
    }

    async fn list_candidates(&self) -> Result<Vec<CandidateRecord>, RepositoryError> {
        let candidate_rows = sqlx::query(
            "SELECT id, display_name, declared_on, declaration_source_url,
                    declaration_source_label, official_site_url, program_url,
                    EXISTS (SELECT 1 FROM candidate_program_proposals proposal
                             WHERE proposal.candidate_id = presidential_candidates.id)
                      AS has_program_excerpts
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
                    has_program_excerpts: row.get("has_program_excerpts"),
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
