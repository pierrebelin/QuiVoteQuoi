//! DTO de la semaine de votes (RM-03, RM-04, RM-05, RM-06, RM-07, RM-15, RM-16).
use chrono::NaiveDate;
use serde::Serialize;

use crate::api::final_vote_dto::{GroupDto, StanceDto};
use crate::api::scrutin_dto::{DossierStatusDto, ScrutinSummaryDto, SHOW_OF_HANDS_NOTE};
use crate::api::theme_dto::AssignedFamilyDto;
use crate::application::ports::theme_repository::AssignedFamily;
use crate::application::use_cases::browse_final_votes::FinalVoteEntry;
use crate::application::use_cases::get_week_of_votes::{
    CardTally, OutcomeCount, TextCard, WeekDay, WeekOfVotes,
};
use crate::domain::week::DayState;

/// `AAAA-MM-JJ`: la forme sans zero de tete (`2026-2-3`) est refusee.
const ISO_DATE_LEN: usize = 10;

/// Mention des votes a main levee, affichee avec la semaine (RM-16).
pub const COVERAGE_NOTE: &str = SHOW_OF_HANDS_NOTE;

#[derive(Debug, Serialize)]
pub struct OtherOutcomeDto {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct OutcomeCountDto {
    pub votes: usize,
    pub adopted: usize,
    pub rejected: usize,
    pub others: Vec<OtherOutcomeDto>,
}

#[derive(Debug, Serialize)]
pub struct WeekDayDto {
    pub date: NaiveDate,
    pub votes: usize,
    pub state: &'static str,
}

#[derive(Debug, Serialize)]
pub struct WeekFinalVoteDto {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub text_label: String,
    pub outcome_code: String,
    pub outcome_label: String,
    pub families: Vec<AssignedFamilyDto>,
    pub groups: Vec<StanceDto>,
}

/// Semaine precedente: son lundi et son nombre de votes (RM-15).
#[derive(Debug, Serialize)]
pub struct PreviousWeekDto {
    pub monday: NaiveDate,
    pub votes: i64,
}

#[derive(Serialize)]
pub struct WeekResponse {
    pub monday: NaiveDate,
    pub sunday: NaiveDate,
    pub is_current: bool,
    pub current_week_empty: bool,
    pub last_published: Option<NaiveDate>,
    pub totals: OutcomeCountDto,
    pub texts_count: usize,
    pub days: Vec<WeekDayDto>,
    pub final_votes: Vec<WeekFinalVoteDto>,
    pub groups: Vec<GroupDto>,
    pub texts: Vec<WeekTextDto>,
    pub previous: Option<PreviousWeekDto>,
    pub next: Option<NaiveDate>,
    pub coverage_note: &'static str,
}

/// Dernier vote sur l'ensemble d'une carte ou d'un jour (RM-10).
#[derive(Debug, Serialize)]
pub struct LatestFinalDto {
    pub scrutin_uid: String,
    pub date: NaiveDate,
    pub number: String,
    pub outcome_code: String,
    pub outcome_label: String,
}

/// Compteurs d'une carte ou d'un jour, avec le dernier vote sur l'ensemble.
#[derive(Debug, Serialize)]
pub struct CardTallyDto {
    #[serde(flatten)]
    pub outcomes: OutcomeCountDto,
    pub latest_final: Option<LatestFinalDto>,
}

#[derive(Debug, Serialize)]
pub struct CardDayDto {
    pub date: NaiveDate,
    pub tally: CardTallyDto,
}

/// Carte d'un texte de la semaine: etape, sujets, compteurs, jours et votes (RM-08 a RM-11).
#[derive(Serialize)]
pub struct WeekTextDto {
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    pub dossier_status: Option<DossierStatusDto>,
    pub families: Vec<AssignedFamilyDto>,
    pub totals: CardTallyDto,
    pub days: Vec<CardDayDto>,
    pub scrutins: Vec<ScrutinSummaryDto>,
}

impl From<CardTally> for CardTallyDto {
    fn from(tally: CardTally) -> Self {
        Self {
            outcomes: tally.outcomes.into(),
            latest_final: tally.latest_final.map(|last| LatestFinalDto {
                scrutin_uid: last.scrutin_uid,
                date: last.date,
                number: last.number,
                outcome_code: last.outcome_code,
                outcome_label: last.outcome_label,
            }),
        }
    }
}

impl From<TextCard> for WeekTextDto {
    fn from(card: TextCard) -> Self {
        Self {
            dossier_uid: card.dossier_uid,
            dossier_label: card.dossier_label,
            dossier_status: card.dossier.as_ref().map(DossierStatusDto::from),
            families: card.families.into_iter().map(AssignedFamilyDto::from).collect(),
            totals: card.totals.into(),
            days: card
                .days
                .into_iter()
                .map(|day| CardDayDto {
                    date: day.date,
                    tally: day.tally.into(),
                })
                .collect(),
            scrutins: card.items.into_iter().map(ScrutinSummaryDto::from).collect(),
        }
    }
}

/// RM-03: une date lisible est au format ISO `AAAA-MM-JJ`, rien d'autre.
pub fn requested_date(raw: &str) -> Option<NaiveDate> {
    if raw.len() != ISO_DATE_LEN {
        return None;
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
}

impl From<WeekOfVotes> for WeekResponse {
    fn from(view: WeekOfVotes) -> Self {
        let mut families = view.families;
        Self {
            monday: view.week.monday(),
            sunday: view.week.sunday(),
            is_current: view.is_current,
            current_week_empty: view.current_week_empty,
            last_published: view.last_published,
            totals: view.totals.into(),
            texts_count: view.texts_count,
            days: view.days.into_iter().map(WeekDayDto::from).collect(),
            final_votes: view
                .final_votes
                .into_iter()
                .map(|entry| {
                    let uid = entry.vote.scrutin_uid().to_string();
                    WeekFinalVoteDto::from((entry, families.remove(&uid).unwrap_or_default()))
                })
                .collect(),
            groups: view.groups.into_iter().map(GroupDto::from).collect(),
            texts: view.texts.into_iter().map(WeekTextDto::from).collect(),
            previous: view.previous.map(|week| PreviousWeekDto {
                monday: week.monday,
                votes: week.votes_count,
            }),
            next: view.next.map(|week| week.monday),
            coverage_note: COVERAGE_NOTE,
        }
    }
}

impl From<OutcomeCount> for OutcomeCountDto {
    fn from(count: OutcomeCount) -> Self {
        Self {
            votes: count.votes,
            adopted: count.adopted,
            rejected: count.rejected,
            others: count
                .others
                .into_iter()
                .map(|other| OtherOutcomeDto {
                    label: other.label,
                    count: other.count,
                })
                .collect(),
        }
    }
}

impl From<WeekDay> for WeekDayDto {
    fn from(day: WeekDay) -> Self {
        let votes = match day.state {
            DayState::Voted(votes) => votes,
            _ => 0,
        };
        Self {
            date: day.date,
            votes,
            state: day.state.as_str(),
        }
    }
}

/// Un vote sur l'ensemble, avec la position de chaque groupe et les familles de son scrutin.
impl From<(FinalVoteEntry, Vec<AssignedFamily>)> for WeekFinalVoteDto {
    fn from((entry, families): (FinalVoteEntry, Vec<AssignedFamily>)) -> Self {
        let vote = entry.vote;
        Self {
            scrutin_uid: vote.scrutin_uid().to_string(),
            number: vote.number().to_string(),
            date: vote.date(),
            text_label: vote.text_label().to_string(),
            outcome_code: vote.outcome().code().to_string(),
            outcome_label: vote.outcome().label().to_string(),
            families: families.into_iter().map(AssignedFamilyDto::from).collect(),
            groups: vote.stances().iter().map(StanceDto::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::application::ports::final_vote_repository::GroupOption;
    use crate::application::use_cases::browse_final_votes::FinalVoteEntry;
    use crate::application::ports::theme_repository::AssignedFamily;
    use crate::application::use_cases::get_week_of_votes::{
        CardDay, CardTally, FinalOutcome, OtherOutcome, OutcomeCount, TextCard, WeekDay,
    };
    use crate::application::use_cases::get_week_of_votes::WeekNeighbour;
    use crate::application::use_cases::list_scrutins::ScrutinListItem;
    use crate::application::use_cases::read_fakes::{listed_dossier, scrutin_row};
    use crate::domain::dossier::{DossierOutcome, LegislativeStage};
    use crate::domain::scrutin::ShortLabel;
    use crate::domain::theme::FamilyCode;
    use crate::domain::actor::GroupUid;
    use crate::domain::final_vote::{FinalVote, GroupIdentity, GroupStance, NewFinalVote};
    use crate::domain::scrutin::{Outcome, ScrutinUid, VotePosition, VoteTally};
    use crate::domain::theme::TextKey;
    use crate::domain::week::{DayState, Week};

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    // vendredi 13 novembre 2026
    fn today() -> NaiveDate {
        d(13)
    }

    fn view() -> WeekOfVotes {
        WeekOfVotes {
            week: Week::requested(d(4), today()).unwrap(),
            is_current: false,
            current_week_empty: false,
            last_published: Some(d(5)),
            totals: OutcomeCount {
                votes: 0,
                adopted: 0,
                rejected: 0,
                others: vec![],
            },
            texts_count: 0,
            days: vec![],
            final_votes: vec![],
            groups: vec![],
            families: Default::default(),
            texts: vec![],
            previous: None,
            next: None,
        }
    }

    fn json_of(view: WeekOfVotes) -> Value {
        serde_json::to_value(WeekResponse::from(view)).unwrap()
    }

    fn stance(uid: &str, abbrev: &str, position: VotePosition, votes_for: u16) -> GroupStance {
        GroupStance::new(
            GroupIdentity {
                uid: GroupUid::new(uid.into()).unwrap(),
                abbrev: abbrev.into(),
                label: format!("Groupe {abbrev}"),
                color: None,
            },
            Some(120),
            Some(position),
            VoteTally {
                votes_for,
                ..VoteTally::default()
            },
        )
    }

    fn final_vote(uid: &str, stances: Vec<GroupStance>) -> FinalVoteEntry {
        FinalVoteEntry {
            vote: FinalVote::new(NewFinalVote {
                scrutin_uid: ScrutinUid::new(uid.into()).unwrap(),
                number: "12".into(),
                date: d(4),
                subject: "l'ensemble du projet de loi".into(),
                ballot_type_label: "scrutin public solennel".into(),
                outcome: Outcome::new("adopt\u{00e9}".into(), "adopt\u{00e9}".into()).unwrap(),
                text_key: TextKey::from_raw("projet de loi"),
                text_label: "projet de loi de finances".into(),
                dossier_uid: None,
                dossier_label: None,
                official_text: None,
                synthesis: VoteTally::default(),
                stances,
            }),
            families: vec![],
        }
    }

    #[test]
    fn reads_an_iso_date_from_the_path_and_refuses_anything_else() {
        assert_eq!(requested_date("2026-11-13"), Some(d(13)));
        for raw in [
            "2026-02-30",
            "13/11/2026",
            "2026-2-3",
            "2026-11-13T10:00:00",
            "demain",
            "",
        ] {
            assert_eq!(requested_date(raw), None, "{raw:?}");
        }
    }

    #[test]
    fn maps_the_bounds_the_current_week_flag_and_the_empty_current_week_notice() {
        let json = json_of(WeekOfVotes {
            is_current: false,
            current_week_empty: true,
            ..view()
        });

        assert_eq!(json["monday"], "2026-11-02");
        assert_eq!(json["sunday"], "2026-11-08");
        assert_eq!(json["is_current"], false);
        assert_eq!(json["current_week_empty"], true);
        assert_eq!(json["last_published"], "2026-11-05");
    }

    #[test]
    fn maps_week_totals_with_other_outcomes_under_their_published_label() {
        let json = json_of(WeekOfVotes {
            totals: OutcomeCount {
                votes: 5,
                adopted: 2,
                rejected: 1,
                others: vec![OtherOutcome {
                    label: "renvoy\u{00e9} en commission".into(),
                    count: 2,
                }],
            },
            texts_count: 3,
            ..view()
        });

        assert_eq!(
            json["totals"],
            json!({
                "votes": 5,
                "adopted": 2,
                "rejected": 1,
                "others": [{"label": "renvoy\u{00e9} en commission", "count": 2}],
            })
        );
        assert_eq!(json["texts_count"], 3);
    }

    #[test]
    fn maps_each_day_with_its_vote_count_and_state_code() {
        let json = json_of(WeekOfVotes {
            days: vec![
                WeekDay { date: d(2), state: DayState::Voted(3) },
                WeekDay { date: d(3), state: DayState::NoPublicVote },
                WeekDay { date: d(4), state: DayState::NotYetPublished },
                WeekDay { date: d(5), state: DayState::Upcoming },
            ],
            ..view()
        });

        assert_eq!(
            json["days"],
            json!([
                {"date": "2026-11-02", "votes": 3, "state": "voted"},
                {"date": "2026-11-03", "votes": 0, "state": "no_public_vote"},
                {"date": "2026-11-04", "votes": 0, "state": "not_yet_published"},
                {"date": "2026-11-05", "votes": 0, "state": "upcoming"},
            ])
        );
    }

    #[test]
    fn maps_final_votes_with_every_group_and_the_group_list() {
        let json = json_of(WeekOfVotes {
            final_votes: vec![final_vote(
                "VTANR5L17V12",
                vec![
                    stance("PO1", "RN", VotePosition::For, 100),
                    stance("PO2", "SOC", VotePosition::Against, 0),
                ],
            )],
            groups: vec![GroupOption {
                uid: "PO1".into(),
                abbrev: "RN".into(),
                label: "Groupe RN".into(),
                color: None,
                final_vote_count: 7,
            }],
            ..view()
        });

        let vote = &json["final_votes"][0];
        assert_eq!(vote["scrutin_uid"], "VTANR5L17V12");
        assert_eq!(vote["number"], "12");
        assert_eq!(vote["outcome_label"], "adopt\u{00e9}");
        assert_eq!(vote["groups"].as_array().unwrap().len(), 2);
        assert_eq!(vote["groups"][0]["abbrev"], "RN");
        assert_eq!(vote["groups"][0]["majority"], "for");
        assert_eq!(vote["groups"][0]["tally"]["votes_for"], 100);
        assert_eq!(vote["groups"][1]["majority"], "against");
        assert_eq!(json["groups"].as_array().unwrap().len(), 1);
        assert_eq!(json["groups"][0]["abbrev"], "RN");
        assert_eq!(json["groups"][0]["final_vote_count"], 7);
    }

    fn counts(votes: usize, adopted: usize, rejected: usize) -> OutcomeCount {
        OutcomeCount { votes, adopted, rejected, others: vec![] }
    }

    fn listed(uid: &str) -> ScrutinListItem {
        let subject = "l'article 3 du projet de loi de finances";
        ScrutinListItem {
            summary: scrutin_row(uid, subject),
            short_label: ShortLabel::from_subject(subject),
            cited: vec![],
            dossier: None,
        }
    }

    fn card(dossier_uid: Option<&str>, items: Vec<ScrutinListItem>) -> TextCard {
        TextCard {
            dossier_uid: dossier_uid.map(String::from),
            dossier_label: dossier_uid.map(|_| "Projet de loi de finances".to_string()),
            dossier: None,
            families: vec![],
            totals: CardTally { outcomes: counts(items.len(), 0, 0), latest_final: None },
            days: vec![],
            items,
        }
    }

    #[test]
    fn maps_a_text_card_with_its_stage_its_families_and_its_votes() {
        let mut text = card(Some("DLR5L17N54629"), vec![listed("S1"), listed("S2")]);
        text.dossier = Some(listed_dossier(
            "DLR5L17N54629",
            DossierOutcome::Rejected { date: d(4), label: "rejet\u{e9}".into() },
            Some(LegislativeStage::FirstReadingAN),
        ));
        text.families = vec![AssignedFamily {
            family: FamilyCode::PouvoirAchat,
            opened_on: d(3),
            motive: None,
        }];
        text.totals = CardTally { outcomes: counts(2, 1, 1), latest_final: None };

        let json = json_of(WeekOfVotes { texts: vec![text], ..view() });

        let card = &json["texts"][0];
        assert_eq!(card["dossier_uid"], "DLR5L17N54629");
        assert_eq!(card["dossier_status"]["outcome"]["kind"], "rejected");
        assert_eq!(card["families"].as_array().unwrap().len(), 1);
        assert_eq!(card["families"][0]["code"], FamilyCode::PouvoirAchat.as_str());
        assert_eq!(card["totals"]["votes"], 2);
        assert_eq!(card["totals"]["adopted"], 1);
        assert_eq!(card["scrutins"].as_array().unwrap().len(), 2);
        assert_eq!(card["scrutins"][0]["uid"], "S1");
        assert_eq!(card["scrutins"][1]["uid"], "S2");
    }

    #[test]
    fn maps_the_unattached_card_last_without_dossier() {
        let json = json_of(WeekOfVotes {
            texts: vec![
                card(Some("DLR5L17N54629"), vec![listed("S1")]),
                card(None, vec![listed("S9")]),
            ],
            ..view()
        });

        let texts = json["texts"].as_array().unwrap();
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0]["dossier_uid"], "DLR5L17N54629");
        assert!(texts[1]["dossier_uid"].is_null());
        assert_eq!(texts[1]["scrutins"][0]["uid"], "S9");
    }

    #[test]
    fn maps_each_card_day_with_its_counts_and_latest_final_outcome() {
        let mut text = card(Some("DLR5L17N54629"), vec![listed("S1")]);
        text.days = vec![
            CardDay {
                date: d(3),
                tally: CardTally { outcomes: counts(2, 1, 1), latest_final: None },
            },
            CardDay {
                date: d(4),
                tally: CardTally {
                    outcomes: counts(1, 1, 0),
                    latest_final: Some(FinalOutcome {
                        scrutin_uid: "S1".into(),
                        date: d(4),
                        number: "12".into(),
                        outcome_code: "adopte".into(),
                        outcome_label: "adopt\u{e9}".into(),
                    }),
                },
            },
        ];

        let json = json_of(WeekOfVotes { texts: vec![text], ..view() });

        let days = &json["texts"][0]["days"];
        assert_eq!(days.as_array().unwrap().len(), 2);
        assert_eq!(days[0]["date"], "2026-11-03");
        assert_eq!(days[0]["tally"]["votes"], 2);
        assert!(days[0]["tally"]["latest_final"].is_null());
        assert_eq!(days[1]["tally"]["adopted"], 1);
        assert_eq!(days[1]["tally"]["latest_final"]["scrutin_uid"], "S1");
        assert_eq!(days[1]["tally"]["latest_final"]["outcome_label"], "adopt\u{e9}");
    }

    #[test]
    fn maps_the_previous_week_with_its_vote_count_and_the_next_week() {
        let json = json_of(WeekOfVotes {
            previous: Some(WeekNeighbour { monday: d(2), votes_count: 7 }),
            next: Some(WeekNeighbour { monday: d(9), votes_count: 4 }),
            ..view()
        });

        assert_eq!(json["previous"], json!({"monday": "2026-11-02", "votes": 7}));
        assert_eq!(json["next"], "2026-11-09");
    }

    #[test]
    fn maps_no_previous_week_on_the_first_week_and_no_next_week_on_the_current_week() {
        let json = json_of(view());

        assert!(json["previous"].is_null());
        assert!(json["next"].is_null());
    }

    #[test]
    fn carries_the_show_of_hands_note() {
        let json = json_of(view());

        let note = json["coverage_note"].as_str().unwrap();
        assert_eq!(note, COVERAGE_NOTE);
        assert!(note.contains("main lev\u{00e9}e"), "{note:?}");
    }
}
