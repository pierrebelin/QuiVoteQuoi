use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

use crate::application::ports::vote_explanation_repository::ExplanationMention;
use crate::application::use_cases::get_dossier_group_actions::DossierGroupActions;
use crate::domain::dossier_group_facts::{
    AmendmentFact, DossierGroupFacts, FinalVoteFact, GroupFacts,
};
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
    /// Amendements du groupe par sort, libelle de la source (RM-09), du plus
    /// frequent au plus rare. Les chiffres restent ceux du groupe sur ce dossier.
    pub amendment_fates: Vec<FateCountDto>,
    pub amendments_url: String,
}

#[derive(Serialize)]
pub struct FateCountDto {
    /// Vide quand la source ne publie aucun sort.
    pub label: String,
    pub count: usize,
}

fn fate_counts(amendments: &[AmendmentFact]) -> Vec<FateCountDto> {
    let mut counts: Vec<FateCountDto> = Vec::new();
    for amendment in amendments {
        match counts.iter_mut().find(|c| c.label == amendment.fate_label) {
            Some(count) => count.count += 1,
            None => counts.push(FateCountDto {
                label: amendment.fate_label.clone(),
                count: 1,
            }),
        }
    }
    counts.sort_by(|a, b| b.count.cmp(&a.count));
    counts
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
    /// Orateurs rattachés à ce scrutin sous ce groupe, dans l'ordre du compte rendu (RM-09).
    pub explanations: Vec<ExplanationMentionDto>,
}

#[derive(Serialize)]
pub struct ExplanationMentionDto {
    pub speaker_uid: String,
    pub speaker_name: Option<String>,
    pub scrutin_uid: String,
    pub rank: u16,
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
            facts,
            summaries,
            explanations,
            ..
        } = value;
        Self::from_parts(facts, summaries, explanations)
    }
}

impl From<&ExplanationMention> for ExplanationMentionDto {
    fn from(value: &ExplanationMention) -> Self {
        Self {
            speaker_uid: value.speaker_uid.to_string(),
            speaker_name: value.speaker_name.clone(),
            scrutin_uid: value.scrutin_uid.to_string(),
            rank: value.rank,
        }
    }
}

impl DossierGroupActionsResponse {
    fn from_parts(
        facts: DossierGroupFacts,
        summaries: Vec<StoredGroupSummary>,
        explanations: Vec<ExplanationMention>,
    ) -> Self {
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
                let mentions: Vec<&ExplanationMention> = explanations
                    .iter()
                    .filter(|mention| mention.group_uid == group.uid)
                    .collect();
                group_dto(dossier_uid.as_str(), legislature, group, summary, &mentions)
            })
            .collect();

        Self {
            dossier_uid: dossier_uid.to_string(),
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
    mentions: &[&ExplanationMention],
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
        uid: group.uid.to_string(),
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
            .map(|vote| {
                let explanations = mentions
                    .iter()
                    .filter(|mention| mention.scrutin_uid == vote.scrutin_uid)
                    .map(|mention| ExplanationMentionDto::from(*mention))
                    .collect();
                FinalVoteDto::from_fact(vote, legislature, explanations)
            })
            .collect(),
        amendment_count: group.amendments.len(),
        amendment_fates: fate_counts(&group.amendments),
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
    fn from_fact(
        value: FinalVoteFact,
        legislature: u16,
        explanations: Vec<ExplanationMentionDto>,
    ) -> Self {
        Self {
            scrutin_uid: value.scrutin_uid.to_string(),
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
            explanations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::actor::{ActorUid, GroupUid};
    use crate::domain::dossier::DossierUid;
    use crate::domain::dossier_summary::FactsFingerprint;
    use crate::domain::scrutin::{ScrutinUid, VoteTally};

    fn group(uid: &str, scrutin: &str) -> GroupFacts {
        GroupFacts {
            uid: GroupUid::new(uid.to_string()).unwrap(),
            abbrev: uid.into(),
            label: uid.into(),
            color: None,
            start_date: None,
            end_date: None,
            final_votes: vec![FinalVoteFact {
                scrutin_uid: ScrutinUid::new(scrutin.to_string()).unwrap(),
                number: "1".into(),
                date: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
                legislature: 17,
                subject: "l'ensemble du texte".into(),
                text_label: "texte".into(),
                reading: None,
                outcome_code: "adopted".into(),
                outcome_label: "Adopté".into(),
                majority_position: None,
                member_count: None,
                tally: VoteTally::default(),
            }],
            amendments: vec![],
        }
    }

    fn mention(scrutin: &str, group: &str, rank: u16, speaker: &str) -> ExplanationMention {
        ExplanationMention {
            scrutin_uid: ScrutinUid::new(scrutin.to_string()).unwrap(),
            group_uid: GroupUid::new(group.to_string()).unwrap(),
            rank,
            speaker_uid: ActorUid::new(speaker.to_string()).unwrap(),
            speaker_name: Some(format!("Orateur {speaker}")),
        }
    }

    #[test]
    fn places_each_mention_under_the_final_vote_of_its_group_and_scrutin() {
        let actions = DossierGroupActions {
            facts: DossierGroupFacts {
                dossier_uid: DossierUid::new("D1".to_string()).unwrap(),
                title: "Texte".into(),
                official_url: None,
                legislature: 17,
                period_start: None,
                period_end: None,
                groups: vec![group("G1", "S1"), group("G2", "S2")],
            },
            facts_fingerprint: FactsFingerprint::new("f".to_string()),
            summaries: vec![],
            explanations: vec![
                mention("S1", "G1", 1, "PA1"),
                mention("S2", "G2", 2, "PA2"),
                mention("S2", "G2", 3, "PA3"),
            ],
        };

        let response = DossierGroupActionsResponse::from(actions);

        let placed: Vec<Vec<(&str, u16)>> = response
            .groups
            .iter()
            .map(|g| {
                g.final_votes[0]
                    .explanations
                    .iter()
                    .map(|m| (m.speaker_uid.as_str(), m.rank))
                    .collect()
            })
            .collect();
        assert_eq!(placed, vec![vec![("PA1", 1)], vec![("PA2", 2), ("PA3", 3)]]);
        let first = &response.groups[0].final_votes[0].explanations[0];
        assert_eq!(first.scrutin_uid, "S1");
        assert_eq!(first.speaker_name.as_deref(), Some("Orateur PA1"));
    }

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
