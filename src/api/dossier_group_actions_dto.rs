use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

use crate::application::use_cases::get_dossier_group_actions::DossierGroupActions;
use crate::domain::dossier_group_facts::{DossierGroupFacts, FinalVoteFact, GroupFacts};
use crate::domain::dossier_summary::{
    GroupSummaryState, StoredGroupSummary, SummarySource, SummaryStatus,
};
use crate::domain::scrutin::scrutin_official_url;

#[derive(Serialize)]
pub struct DossierGroupActionsResponse {
    pub dossier_uid: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub groups: Vec<GroupActionsDto>,
    pub notes: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct GroupActionsDto {
    pub uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub state: &'static str,
    pub summary: Option<SummaryDto>,
    pub final_votes: Vec<FinalVoteDto>,
    pub amendment_count: usize,
    pub amendments_url: String,
}

#[derive(Serialize)]
pub struct SummaryDto {
    pub label: &'static str,
    pub status: &'static str,
    pub text: String,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub generated_at: Option<NaiveDateTime>,
    pub sources: Vec<SummarySourceDto>,
}

#[derive(Serialize)]
pub struct SummarySourceDto {
    pub id: String,
    pub kind: String,
    pub uid: String,
    pub label: String,
    pub official_url: Option<String>,
}

#[derive(Serialize)]
pub struct FinalVoteDto {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub subject: String,
    pub text_label: String,
    pub reading: Option<String>,
    pub outcome_code: String,
    pub outcome_label: String,
    pub majority_position: Option<String>,
    pub member_count: Option<u16>,
    pub tally: TallyDto,
    pub official_url: String,
}

#[derive(Serialize)]
pub struct TallyDto {
    pub votes_for: u16,
    pub votes_against: u16,
    pub abstentions: u16,
    pub not_voting: u16,
    pub voluntary_not_voting: u16,
}

impl From<crate::domain::scrutin::VoteTally> for TallyDto {
    fn from(value: crate::domain::scrutin::VoteTally) -> Self {
        Self {
            votes_for: value.votes_for,
            votes_against: value.votes_against,
            abstentions: value.abstentions,
            not_voting: value.not_voting,
            voluntary_not_voting: value.voluntary_not_voting,
        }
    }
}

impl From<DossierGroupActions> for DossierGroupActionsResponse {
    fn from(value: DossierGroupActions) -> Self {
        let DossierGroupActions {
            facts, summaries, ..
        } = value;
        Self::from_parts(facts, summaries)
    }
}

impl DossierGroupActionsResponse {
    fn from_parts(facts: DossierGroupFacts, summaries: Vec<StoredGroupSummary>) -> Self {
        let summary_by_group = summaries
            .into_iter()
            .map(|summary| (summary.group_uid.clone(), summary))
            .collect::<std::collections::HashMap<_, _>>();
        let DossierGroupFacts {
            dossier_uid,
            period_start,
            period_end,
            legislature,
            groups: fact_groups,
            ..
        } = facts;
        let groups = fact_groups
            .into_iter()
            .map(|group| {
                let summary = summary_by_group.get(&group.uid);
                group_dto(&dossier_uid, legislature, group, summary)
            })
            .collect();

        Self {
            dossier_uid,
            period_start,
            period_end,
            groups,
            notes: vec![
                "Cette vue ne présente que les votes finaux ; les autres scrutins restent dans la liste exhaustive.",
                "Les chiffres et répartitions sont rendus à partir des données officielles, jamais par la synthèse automatique.",
                "L'absence de ligne ne vaut ni abstention ni position de groupe.",
            ],
        }
    }
}

fn group_dto(
    dossier_uid: &str,
    legislature: u16,
    group: GroupFacts,
    stored: Option<&StoredGroupSummary>,
) -> GroupActionsDto {
    let has_actions = group.has_actions();
    let summary = stored
        .filter(|summary| summary.status == SummaryStatus::Ready)
        .and_then(|summary| {
            summary.paragraph.clone().map(|text| SummaryDto {
                label: "Synthèse automatique",
                status: "ready",
                text,
                model: summary.model.clone(),
                prompt_version: summary.prompt_version.clone(),
                generated_at: summary.generated_at,
                sources: summary.sources.iter().map(SummarySourceDto::from).collect(),
            })
        });
    let state = GroupSummaryState::new(
        has_actions,
        summary.is_some(),
        stored.map(|summary| &summary.status),
    );

    GroupActionsDto {
        uid: group.uid.clone(),
        abbrev: group.abbrev,
        label: group.label,
        color: group.color,
        start_date: group.start_date,
        end_date: group.end_date,
        state: state.as_str(),
        summary,
        final_votes: group
            .final_votes
            .into_iter()
            .map(|vote| FinalVoteDto::from_fact(vote, legislature))
            .collect(),
        amendment_count: group.amendments.len(),
        amendments_url: format!("/dossiers/{}?amendements_group={}", dossier_uid, group.uid),
    }
}

impl From<&SummarySource> for SummarySourceDto {
    fn from(value: &SummarySource) -> Self {
        Self {
            id: value.source_id.clone(),
            kind: value.kind.clone(),
            uid: value.uid.clone(),
            label: value.label.clone(),
            official_url: value.official_url.clone(),
        }
    }
}

impl FinalVoteDto {
    fn from_fact(value: FinalVoteFact, legislature: u16) -> Self {
        Self {
            scrutin_uid: value.scrutin_uid,
            number: value.number.clone(),
            date: value.date,
            subject: value.subject,
            text_label: value.text_label,
            reading: value.reading,
            outcome_code: value.outcome_code,
            outcome_label: value.outcome_label,
            majority_position: value.majority_position,
            member_count: value.member_count,
            tally: value.tally.into(),
            official_url: scrutin_official_url(legislature, &value.number),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_states_map_to_the_json_contract_strings() {
        assert_eq!(GroupSummaryState::NoData.as_str(), "no_data");
        assert_eq!(GroupSummaryState::Ready.as_str(), "ready");
        assert_eq!(
            GroupSummaryState::SummaryPending.as_str(),
            "summary_pending"
        );
        assert_eq!(
            GroupSummaryState::SummaryUnavailable.as_str(),
            "summary_unavailable"
        );
    }
}
