use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::application::ports::candidate_repository::{
    CandidateParliamentaryGroupRecord, CandidateProgramProposalRecord, CandidateRecord,
    ChangedSourceRecord, PoliticalOrganizationRecord,
};
use crate::application::use_cases::browse_candidates::{
    BrowseCandidatesCommand, CandidateComparisonView, MAX_COMPARED_CANDIDATES,
};

/// Pas de verdict sur l'alignement : les deux ensembles de sources restent
/// visibles séparément pour que le lecteur puisse les consulter.
pub const GROUPS_NOTE: &str = "Un parti ou un soutien de campagne n'est pas un groupe parlementaire. Les groupes affichés ici ne le sont que lorsqu'une source les relie explicitement à la candidature ; leurs votes restent des votes de groupe, pas des votes du candidat.";
pub const PROPOSALS_NOTE: &str = "Les propositions sont des extraits attribués à leur programme ou à une autre source primaire. Le site ne les résume pas et ne calcule aucun indicateur d'alignement.";
pub const DECLARATION_NOTE: &str = "Seules les candidatures accompagnées d'une déclaration publique primaire sont référencées. La liste s'enrichit au fil des déclarations vérifiables.";

#[derive(Debug, Deserialize)]
pub struct CandidateListQuery {
    pub theme: Option<String>,
    /// Identifiants stables, séparés par une virgule, dans l'ordre voulu.
    pub candidats: Option<String>,
}

impl From<CandidateListQuery> for BrowseCandidatesCommand {
    fn from(query: CandidateListQuery) -> Self {
        Self {
            family: query.theme,
            candidate_ids: query
                .candidats
                .unwrap_or_default()
                .split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PoliticalOrganizationDto {
    pub label: String,
    pub official_url: Option<String>,
    pub source_url: String,
    pub source_label: String,
}

impl From<&PoliticalOrganizationRecord> for PoliticalOrganizationDto {
    fn from(value: &PoliticalOrganizationRecord) -> Self {
        Self {
            label: value.label.clone(),
            official_url: value.official_url.clone(),
            source_url: value.source_url.clone(),
            source_label: value.source_label.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CandidateDto {
    pub id: String,
    pub display_name: String,
    pub declared_on: NaiveDate,
    pub declaration_source_url: String,
    pub declaration_source_label: String,
    pub official_site_url: Option<String>,
    pub program_url: Option<String>,
    pub political_organizations: Vec<PoliticalOrganizationDto>,
}

impl From<&CandidateRecord> for CandidateDto {
    fn from(value: &CandidateRecord) -> Self {
        Self {
            id: value.id.as_str().to_string(),
            display_name: value.display_name.clone(),
            declared_on: value.declared_on,
            declaration_source_url: value.declaration_source_url.clone(),
            declaration_source_label: value.declaration_source_label.clone(),
            official_site_url: value.official_site_url.clone(),
            program_url: value.program_url.clone(),
            political_organizations: value
                .organizations
                .iter()
                .map(PoliticalOrganizationDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CandidateProgramProposalDto {
    pub candidate_id: String,
    pub theme_code: String,
    pub excerpt: String,
    pub source_url: String,
    pub source_label: String,
    pub source_published_on: Option<NaiveDate>,
}

impl From<&CandidateProgramProposalRecord> for CandidateProgramProposalDto {
    fn from(value: &CandidateProgramProposalRecord) -> Self {
        Self {
            candidate_id: value.candidate_id.as_str().to_string(),
            theme_code: value.family.as_str().to_string(),
            excerpt: value.excerpt.clone(),
            source_url: value.source_url.clone(),
            source_label: value.source_label.clone(),
            source_published_on: value.source_published_on,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CandidateParliamentaryGroupDto {
    pub candidate_id: String,
    pub group_uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub linked_on: NaiveDate,
    pub source_url: String,
    pub source_label: String,
}

impl From<&CandidateParliamentaryGroupRecord> for CandidateParliamentaryGroupDto {
    fn from(value: &CandidateParliamentaryGroupRecord) -> Self {
        Self {
            candidate_id: value.candidate_id.as_str().to_string(),
            group_uid: value.group_uid.clone(),
            abbrev: value.abbrev.clone(),
            label: value.label.clone(),
            color: value.color.clone(),
            linked_on: value.linked_on,
            source_url: value.source_url.clone(),
            source_label: value.source_label.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CandidateComparisonResponse {
    pub candidates: Vec<CandidateDto>,
    pub selected: Vec<CandidateDto>,
    pub proposals: Vec<CandidateProgramProposalDto>,
    pub parliamentary_groups: Vec<CandidateParliamentaryGroupDto>,
    pub selected_theme: Option<String>,
    pub max_compared_candidates: usize,
    pub declaration_note: &'static str,
    pub proposals_note: &'static str,
    pub groups_note: &'static str,
}

impl From<CandidateComparisonView> for CandidateComparisonResponse {
    fn from(value: CandidateComparisonView) -> Self {
        Self {
            candidates: value.candidates.iter().map(CandidateDto::from).collect(),
            selected: value.selected.iter().map(CandidateDto::from).collect(),
            proposals: value
                .proposals
                .iter()
                .map(CandidateProgramProposalDto::from)
                .collect(),
            parliamentary_groups: value
                .parliamentary_groups
                .iter()
                .map(CandidateParliamentaryGroupDto::from)
                .collect(),
            selected_theme: value
                .selected_family
                .map(|family| family.as_str().to_string()),
            max_compared_candidates: MAX_COMPARED_CANDIDATES,
            declaration_note: DECLARATION_NOTE,
            proposals_note: PROPOSALS_NOTE,
            groups_note: GROUPS_NOTE,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChangedSourcesQuery {
    /// Fenêtre de recherche en jours (7 par défaut, 90 au plus).
    pub jours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChangedSourceDto {
    pub source_url: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
    pub candidates: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChangedSourcesResponse {
    pub changed_sources: Vec<ChangedSourceDto>,
}

impl From<Vec<ChangedSourceRecord>> for ChangedSourcesResponse {
    fn from(records: Vec<ChangedSourceRecord>) -> Self {
        Self {
            changed_sources: records
                .into_iter()
                .map(|record| ChangedSourceDto {
                    source_url: record.source_url,
                    changed_at: record.changed_at,
                    candidates: record.candidate_names,
                })
                .collect(),
        }
    }
}
