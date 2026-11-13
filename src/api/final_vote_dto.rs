use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::api::scrutin_dto::explanations_state_code;
use crate::api::theme_dto::AssignedFamilyDto;
use crate::application::ports::final_vote_repository::GroupOption;
use crate::application::use_cases::browse_final_votes::{
    BrowseFinalVotesCommand, FinalVoteEntry, FinalVoteView, StanceExplanation, MAX_COMPARED_GROUPS,
};
use crate::domain::amendment::SummaryPreview;
use crate::domain::final_vote::{GroupStance, OfficialTextVersion, VoterShare};
use crate::domain::scrutin::{VotePosition, VoteTally};
use crate::domain::vote_explanation::ExplanationCoverage;

/// Perimetre de la page, affiche en tete (README.md §2): la restriction est
/// annoncee, et le reste des scrutins reste accessible.
pub const SCOPE_NOTE: &str = "Cette page ne montre que le vote sur l'ensemble d'un texte, \
     celui qui tranche le texte entier. Les votes d'amendement, d'article et les motions de \
     censure restent consultables sur la page Scrutins.";

/// Methode du pourcentage, affichee a cote des chiffres (README.md §6, §9).
pub const SHARE_NOTE: &str = "Le pourcentage rapporte les voix d'un groupe à ses seuls votants \
     (pour + contre + abstention). Les non-votants sont comptés à part, en valeur brute.";

/// Portee de l'issue affichee: elle vaut pour cette lecture, pas pour le sort
/// final de la loi.
pub const OUTCOME_NOTE: &str = "« Adopté » ou « rejeté » est l'issue publiée de ce scrutin, \
     pour cette lecture à l'Assemblée. Ce n'est pas l'état final de la loi.";

#[derive(Debug, Deserialize)]
pub struct FinalVoteListQuery {
    /// Code de famille thematique.
    pub theme: Option<String>,
    /// Groupes compares, sigles ou identifiants separes par une virgule.
    pub groupes: Option<String>,
    /// Mode « tous les groupes » de la page d'accueil: une position par groupe
    /// du referentiel. Exclusif avec `groupes`.
    pub tous_groupes: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<FinalVoteListQuery> for BrowseFinalVotesCommand {
    fn from(query: FinalVoteListQuery) -> Self {
        Self {
            family: query.theme,
            groups: query
                .groupes
                .unwrap_or_default()
                .split(',')
                .map(|token| token.trim().to_string())
                .filter(|token| !token.is_empty())
                .collect(),
            all_groups: query.tous_groupes.unwrap_or(false),
            from: None,
            to: None,
            limit: query.limit,
            offset: query.offset,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TallyDto {
    pub votes_for: u16,
    pub votes_against: u16,
    pub abstentions: u16,
    pub not_voting: u16,
    pub voluntary_not_voting: u16,
}

impl From<VoteTally> for TallyDto {
    fn from(tally: VoteTally) -> Self {
        Self {
            votes_for: tally.votes_for,
            votes_against: tally.votes_against,
            abstentions: tally.abstentions,
            not_voting: tally.not_voting,
            voluntary_not_voting: tally.voluntary_not_voting,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ShareDto {
    pub voters: u16,
    pub for_percent: u8,
    pub against_percent: u8,
    pub abstention_percent: u8,
    /// Position rassemblant le plus de votants. `null` a egalite.
    pub leading: Option<String>,
    pub leading_label: Option<String>,
    pub leading_percent: Option<u8>,
    /// Positions a egalite en tete, nommees. Un seul element hors egalite.
    pub tied_labels: Vec<String>,
}

impl From<VoterShare> for ShareDto {
    fn from(share: VoterShare) -> Self {
        let leading = share.leading();
        Self {
            voters: share.voters(),
            for_percent: share.for_percent(),
            against_percent: share.against_percent(),
            abstention_percent: share.abstention_percent(),
            leading: leading.map(|p| p.as_str().to_string()),
            leading_label: leading.map(position_label),
            leading_percent: share.leading_percent(),
            tied_labels: share.tied().iter().copied().map(position_label).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GroupDto {
    pub uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    /// Votes sur l'ensemble ou le groupe apparait. Un groupe constitue en cours
    /// de legislature en porte moins que le total.
    pub final_vote_count: i64,
}

impl From<GroupOption> for GroupDto {
    fn from(group: GroupOption) -> Self {
        Self {
            uid: group.uid,
            abbrev: group.abbrev,
            label: group.label,
            color: group.color,
            final_vote_count: group.final_vote_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StanceDto {
    pub group_uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub member_count: Option<u16>,
    /// Position majoritaire publiee par la source, jamais recalculee (RM-02).
    /// `null` quand la source n'en publie pas.
    pub majority: Option<String>,
    pub majority_label: Option<String>,
    pub tally: TallyDto,
    /// `null` quand aucun membre du groupe ne s'est prononce.
    pub share: Option<ShareDto>,
    /// Explication de vote du groupe (RM-09). Absente quand les explications
    /// ne sont pas lues ou qu'aucun orateur n'est rattaché au groupe: l'état
    /// du vote (`FinalVoteDto::explanations_state`) dit lequel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<StanceExplanationDto>,
}

/// Début d'une explication de vote, repris tel quel du compte rendu (README.md §6).
#[derive(Debug, Serialize)]
pub struct StanceExplanationDto {
    /// Ancre de l'explication sur la fiche du scrutin: `#explication-<rank>`.
    pub rank: u16,
    pub speaker_name: Option<String>,
    /// `null`: position non publiée, jamais déduite.
    pub speaker_position: Option<String>,
    pub preview: Option<String>,
    pub preview_truncated: bool,
}

impl From<&StanceExplanation> for StanceExplanationDto {
    fn from(explanation: &StanceExplanation) -> Self {
        let (preview, preview_truncated) = match &explanation.preview {
            SummaryPreview::Text { text, truncated } => (Some(text.clone()), *truncated),
            SummaryPreview::Unpublished => (None, false),
        };
        Self {
            rank: explanation.rank,
            speaker_name: explanation.speaker_name.clone(),
            speaker_position: explanation.speaker_position.map(|p| p.as_str().to_string()),
            preview,
            preview_truncated,
        }
    }
}

impl From<&GroupStance> for StanceDto {
    fn from(stance: &GroupStance) -> Self {
        Self {
            group_uid: stance.group().uid.as_str().to_string(),
            abbrev: stance.group().abbrev.clone(),
            label: stance.group().label.clone(),
            color: stance.group().color.clone(),
            member_count: stance.member_count(),
            majority: stance.published_majority().map(|p| p.as_str().to_string()),
            majority_label: stance.published_majority().map(position_label),
            tally: (*stance.tally()).into(),
            share: stance.share().cloned().map(ShareDto::from),
            explanation: None,
        }
    }
}

/// Libelle francais d'une position, tel que la source la nomme.
fn position_label(position: VotePosition) -> String {
    match position {
        VotePosition::For => "pour",
        VotePosition::Against => "contre",
        VotePosition::Abstention => "abstention",
        VotePosition::NotVoting => "non-votant",
    }
    .to_string()
}

#[derive(Debug, Serialize)]
pub struct FinalVoteDto {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub ballot_type_label: String,
    /// Titre affiche: le libelle du texte debattu. 150 des 222 votes sur
    /// l'ensemble ne portent aucun dossier, le titre ne peut donc pas en venir.
    pub text_key: String,
    pub text_label: String,
    /// Mention de lecture, quand le libelle du texte ne la porte pas deja.
    pub reading: Option<String>,
    pub outcome_code: String,
    pub outcome_label: String,
    pub adopted: bool,
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    /// `None` quand aucune source officielle n'établit encore le lien entre le
    /// vote et une version précise. Ce n'est pas une absence de texte.
    pub official_text: Option<OfficialTextVersionDto>,
    /// Decompte officiel de l'Assemblee entiere.
    pub synthesis: TallyDto,
    pub families: Vec<AssignedFamilyDto>,
    /// Positions des groupes compares, dans l'ordre demande. Un groupe sans
    /// ligne dans ce scrutin en est absent: rien n'est comble par un zero.
    pub stances: Vec<StanceDto>,
    /// État des explications de vote (RM-09): `explained`, `no_explanation`,
    /// `not_attached` ou `report_not_published`. Absent quand elles ne sont
    /// pas lues (mode « tous les groupes »).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanations_state: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct OfficialTextVersionDto {
    pub document_uid: String,
    pub document_title: String,
    pub version_label: String,
    pub document_published_on: Option<NaiveDate>,
    pub official_url: String,
    pub mapping_source_url: String,
    pub source_producer: String,
    pub source_license: String,
    pub source_metadata_fingerprint: Option<String>,
    pub source_retrieved_at: DateTime<Utc>,
}

impl From<OfficialTextVersion> for OfficialTextVersionDto {
    fn from(value: OfficialTextVersion) -> Self {
        Self {
            document_uid: value.document_uid,
            document_title: value.document_title,
            version_label: value.version_label,
            document_published_on: value.document_published_on,
            official_url: value.official_url,
            mapping_source_url: value.mapping_source_url,
            source_producer: value.source_producer,
            source_license: value.source_license,
            source_metadata_fingerprint: value.source_metadata_fingerprint,
            source_retrieved_at: value.source_retrieved_at,
        }
    }
}

impl From<FinalVoteEntry> for FinalVoteDto {
    fn from(entry: FinalVoteEntry) -> Self {
        let vote = entry.vote;
        Self {
            scrutin_uid: vote.scrutin_uid().to_string(),
            number: vote.number().to_string(),
            date: vote.date(),
            ballot_type_label: vote.ballot_type_label().to_string(),
            text_key: vote.text_key().to_string(),
            text_label: vote.text_label().to_string(),
            reading: vote.reading().map(str::to_string),
            outcome_code: vote.outcome().code().to_string(),
            outcome_label: vote.outcome().label().to_string(),
            adopted: vote.outcome().is_adopted(),
            dossier_uid: vote.dossier_uid().map(ToString::to_string),
            dossier_label: vote.dossier_label().map(str::to_string),
            official_text: vote
                .official_text()
                .cloned()
                .map(OfficialTextVersionDto::from),
            synthesis: (*vote.synthesis()).into(),
            families: entry
                .families
                .into_iter()
                .map(AssignedFamilyDto::from)
                .collect(),
            stances: vote
                .stances()
                .iter()
                .enumerate()
                .map(|(index, stance)| StanceDto {
                    explanation: entry
                        .explanations
                        .as_ref()
                        .and_then(|explanations| explanations.by_stance.get(index))
                        .and_then(Option::as_ref)
                        .map(StanceExplanationDto::from),
                    ..StanceDto::from(stance)
                })
                .collect(),
            explanations_state: entry
                .explanations
                .as_ref()
                .map(|explanations| explanations_state_code(explanations.state)),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FinalVoteListResponse {
    pub items: Vec<FinalVoteDto>,
    pub total: i64,
    /// Votes sur l'ensemble hors filtre thematique.
    pub total_unfiltered: i64,
    /// Votes sur l'ensemble deja rattaches a une famille.
    pub total_with_family: i64,
    pub offset: i64,
    pub groups: Vec<GroupDto>,
    pub selected: Vec<GroupDto>,
    pub max_compared_groups: usize,
    pub scope_note: &'static str,
    pub share_note: &'static str,
    pub outcome_note: &'static str,
}

impl From<(FinalVoteView, i64)> for FinalVoteListResponse {
    fn from((view, offset): (FinalVoteView, i64)) -> Self {
        Self {
            items: view.items.into_iter().map(FinalVoteDto::from).collect(),
            total: view.total,
            total_unfiltered: view.total_unfiltered,
            total_with_family: view.total_with_family,
            offset,
            groups: view.groups.into_iter().map(GroupDto::from).collect(),
            selected: view.selected.into_iter().map(GroupDto::from).collect(),
            max_compared_groups: MAX_COMPARED_GROUPS,
            scope_note: SCOPE_NOTE,
            share_note: SHARE_NOTE,
            outcome_note: OUTCOME_NOTE,
        }
    }
}

/// Compteurs du rattachement des explications de vote (RM-10), publies tels quels.
#[derive(Debug, Serialize)]
pub struct ExplanationCoverageDto {
    pub results_attached: u32,
    pub results_unmatched: u32,
    pub results_ambiguous: u32,
    pub final_votes_unattached: u32,
    pub final_votes_without_block: u32,
    pub blocks_unattached: u32,
    pub headings_unrecognized: u32,
    pub speakers_without_group: u32,
}

impl From<ExplanationCoverage> for ExplanationCoverageDto {
    fn from(coverage: ExplanationCoverage) -> Self {
        Self {
            results_attached: coverage.results_attached,
            results_unmatched: coverage.results_unmatched,
            results_ambiguous: coverage.results_ambiguous,
            final_votes_unattached: coverage.final_votes_unattached,
            final_votes_without_block: coverage.final_votes_without_block,
            blocks_unattached: coverage.blocks_unattached,
            headings_unrecognized: coverage.headings_unrecognized,
            speakers_without_group: coverage.speakers_without_group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> FinalVoteListQuery {
        FinalVoteListQuery {
            theme: None,
            groupes: None,
            tous_groupes: None,
            limit: None,
            offset: None,
        }
    }

    #[test]
    fn the_all_groups_parameter_sets_the_command_flag() {
        let command = BrowseFinalVotesCommand::from(FinalVoteListQuery {
            tous_groupes: Some(true),
            ..query()
        });

        assert!(command.all_groups);
        assert!(command.groups.is_empty());
    }

    #[test]
    fn the_all_groups_flag_is_off_by_default() {
        let command = BrowseFinalVotesCommand::from(FinalVoteListQuery {
            groupes: Some("RN, SOC".to_string()),
            ..query()
        });

        assert!(!command.all_groups);
        assert_eq!(command.groups, vec!["RN", "SOC"]);
    }

    #[test]
    fn maps_every_explanation_coverage_counter() {
        let dto = ExplanationCoverageDto::from(ExplanationCoverage {
            results_attached: 1,
            results_unmatched: 2,
            results_ambiguous: 3,
            final_votes_unattached: 4,
            final_votes_without_block: 5,
            blocks_unattached: 6,
            headings_unrecognized: 7,
            speakers_without_group: 8,
        });

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "results_attached": 1,
                "results_unmatched": 2,
                "results_ambiguous": 3,
                "final_votes_unattached": 4,
                "final_votes_without_block": 5,
                "blocks_unattached": 6,
                "headings_unrecognized": 7,
                "speakers_without_group": 8,
            })
        );
    }

    mod explanations {
        use chrono::NaiveDate;
        use serde_json::json;

        use super::super::*;
        use crate::application::use_cases::browse_final_votes::FinalVoteExplanations;
        use crate::domain::actor::GroupUid;
        use crate::domain::final_vote::{FinalVote, GroupIdentity, NewFinalVote};
        use crate::domain::scrutin::{Outcome, ScrutinUid};
        use crate::domain::theme::TextKey;
        use crate::domain::vote_explanation::ExplanationsState;

        fn stance(uid: &str, abbrev: &str) -> GroupStance {
            GroupStance::new(
                GroupIdentity {
                    uid: GroupUid::new(uid.into()).unwrap(),
                    abbrev: abbrev.into(),
                    label: abbrev.into(),
                    color: None,
                },
                None,
                None,
                VoteTally::default(),
            )
        }

        fn entry(explanations: Option<FinalVoteExplanations>) -> FinalVoteEntry {
            FinalVoteEntry {
                vote: FinalVote::new(NewFinalVote {
                    scrutin_uid: ScrutinUid::new("VTANR5L17V9241".into()).unwrap(),
                    number: "9241".into(),
                    date: NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(),
                    subject: "l'ensemble de la proposition de loi".into(),
                    ballot_type_label: "scrutin public ordinaire".into(),
                    outcome: Outcome::new("adopté".into(), "adopté".into()).unwrap(),
                    text_key: TextKey::from_raw("proposition de loi"),
                    text_label: "proposition de loi".into(),
                    dossier_uid: None,
                    dossier_label: None,
                    official_text: None,
                    synthesis: VoteTally::default(),
                    stances: vec![stance("PO1", "RN"), stance("PO2", "SOC")],
                }),
                families: vec![],
                explanations,
            }
        }

        fn json_of(entry: FinalVoteEntry) -> serde_json::Value {
            serde_json::to_value(FinalVoteDto::from(entry)).unwrap()
        }

        #[test]
        fn serves_the_explanation_of_each_group_under_its_stance() {
            let json = json_of(entry(Some(FinalVoteExplanations {
                state: ExplanationsState::Explained,
                by_stance: vec![
                    Some(StanceExplanation {
                        rank: 3,
                        speaker_name: Some("Claire Marais-Beuil".into()),
                        speaker_position: Some(VotePosition::Abstention),
                        preview: SummaryPreview::Text {
                            text: "Le groupe s'abstiendra…".into(),
                            truncated: true,
                        },
                    }),
                    None,
                ],
            })));

            assert_eq!(json["explanations_state"], "explained");
            assert_eq!(
                json["stances"][0]["explanation"],
                json!({
                    "rank": 3,
                    "speaker_name": "Claire Marais-Beuil",
                    "speaker_position": "abstention",
                    "preview": "Le groupe s'abstiendra…",
                    "preview_truncated": true,
                })
            );
            assert!(json["stances"][1].get("explanation").is_none());
        }

        #[test]
        fn an_unpublished_speaker_position_is_null_never_guessed() {
            let json = json_of(entry(Some(FinalVoteExplanations {
                state: ExplanationsState::Explained,
                by_stance: vec![
                    Some(StanceExplanation {
                        rank: 1,
                        speaker_name: None,
                        speaker_position: None,
                        preview: SummaryPreview::of(Some("Pour.")),
                    }),
                    None,
                ],
            })));

            let explanation = &json["stances"][0]["explanation"];
            assert_eq!(explanation["speaker_position"], serde_json::Value::Null);
            assert_eq!(explanation["speaker_name"], serde_json::Value::Null);
            assert_eq!(explanation["preview_truncated"], false);
        }

        #[test]
        fn each_state_has_its_code() {
            for (state, code) in [
                (ExplanationsState::NoExplanation, "no_explanation"),
                (ExplanationsState::NotAttached, "not_attached"),
                (ExplanationsState::NotPublished, "report_not_published"),
            ] {
                let json = json_of(entry(Some(FinalVoteExplanations {
                    state,
                    by_stance: vec![None, None],
                })));
                assert_eq!(json["explanations_state"], code);
            }
        }

        #[test]
        fn explanations_not_read_leave_the_payload_unchanged() {
            let json = json_of(entry(None));

            assert!(json.get("explanations_state").is_none());
            assert!(json["stances"][0].get("explanation").is_none());
        }
    }
}
