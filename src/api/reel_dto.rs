//! DTO du Reel d'un jour (RM-10, RM-11, RM-12, RM-14, RM-16).
use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Paris;
use serde::Serialize;

use crate::api::scrutin_dto::{RECONSTRUCTED_NOTE, SHOW_OF_HANDS_NOTE};
use crate::api::week_dto::OutcomeCountDto;
use crate::application::ports::scrutin_repository::ScrutinSummary;
use crate::application::use_cases::prepare_reel::{
    PreparedReel, ReelPlan, SkipReason, VotesReel,
};
use crate::domain::reel::{ReelSlot, VoteCompleteness};

pub const EMPTY_WEEK_NOTE: &str = "Aucun vote à l'Assemblée nationale cette semaine.";
pub const CRITERION_NOTE: &str =
    "Critère : le scrutin qui compte le plus de votants (pour, contre, abstention).";
pub const LINK_TOKEN: &str = "[lien]";
pub const TAGS: [&str; 2] = ["#QuiVoteQuoi", "#AssembléeNationale"];

/// Heure de publication du Reel, a Paris (RM-12).
const DUE_HOUR: u32 = 18;

#[derive(Debug, Serialize)]
pub struct ReelPeriodDto {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct ReelDayDto {
    pub date: NaiveDate,
    pub votes: usize,
}

#[derive(Debug, Serialize)]
pub struct ReelFinalVoteDto {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub text_label: String,
    pub outcome_code: String,
    pub outcome_label: String,
}

#[derive(Debug, Serialize)]
pub struct ReelTallyDto {
    pub votes_for: u16,
    pub votes_against: u16,
    pub abstentions: u16,
}

#[derive(Debug, Serialize)]
pub struct MostFollowedDto {
    pub uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub subject: String,
    pub outcome_label: String,
    pub tally: ReelTallyDto,
}

#[derive(Debug, Serialize)]
pub struct ReelResponse {
    pub status: &'static str,
    pub kind: &'static str,
    pub last_success_at: Option<DateTime<Utc>>,
    pub reason: Option<&'static str>,
    pub empty_week_note: Option<&'static str>,
    pub reconstructed_note: Option<&'static str>,
    pub period: ReelPeriodDto,
    pub due_at: String,
    pub totals: Option<OutcomeCountDto>,
    pub days: Vec<ReelDayDto>,
    pub final_votes: Vec<ReelFinalVoteDto>,
    pub most_followed: Option<MostFollowedDto>,
    pub caption: Option<String>,
    pub link_path: Option<String>,
    pub tags: Vec<&'static str>,
}

impl From<ScrutinSummary> for MostFollowedDto {
    fn from(row: ScrutinSummary) -> Self {
        Self {
            uid: row.uid,
            number: row.number,
            date: row.date,
            subject: row.subject,
            outcome_label: row.outcome_label,
            tally: ReelTallyDto {
                votes_for: row.tally.votes_for,
                votes_against: row.tally.votes_against,
                abstentions: row.tally.abstentions,
            },
        }
    }
}

/// Lundi de la semaine contenant le jour donne.
fn monday_of(day: NaiveDate) -> NaiveDate {
    let back = Days::new(u64::from(day.weekday().num_days_from_monday()));
    day.checked_sub_days(back).unwrap_or(day)
}

/// 18 h a Paris le jour de preparation, lendemain du dernier jour couvert (RM-12).
fn due_at(slot: &ReelSlot) -> String {
    let today = slot.to().checked_add_days(Days::new(1)).unwrap_or(slot.to());
    let due = today.and_time(NaiveTime::from_hms_opt(DUE_HOUR, 0, 0).unwrap_or(NaiveTime::MIN));
    Paris
        .from_local_datetime(&due)
        .earliest()
        .map_or_else(|| due.and_utc().to_rfc3339(), |at| at.to_rfc3339())
}

/// Un seul jour avec votes: la semaine filtree sur ce jour; sinon la semaine du dernier jour (RM-14).
fn link_path(slot: &ReelSlot, days: &[ReelDayDto]) -> String {
    let mut voted = days.iter().filter(|day| day.votes > 0);
    match (voted.next(), voted.next()) {
        (Some(only), None) => format!("/semaine/{}?jour={}", monday_of(only.date), only.date),
        _ => format!("/semaine/{}", monday_of(slot.to())),
    }
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count > 1 { plural } else { singular })
}

/// Legende du Reel; sans vote, rien a legender (RM-16).
fn caption(slot: &ReelSlot, reel: &VotesReel) -> Option<String> {
    let totals = &reel.totals;
    if totals.votes == 0 {
        return None;
    }
    let mut counts = vec![
        counted(totals.votes, "vote", "votes"),
        counted(totals.adopted, "adopté", "adoptés"),
        counted(totals.rejected, "rejeté", "rejetés"),
    ];
    counts.extend(
        totals
            .others
            .iter()
            .map(|other| format!("{} {}", other.count, other.label)),
    );
    let mut lines = vec![format!(
        "{} à l'Assemblée nationale : {}.",
        slot.period_label(),
        counts.join(", ")
    )];
    if let Some(followed) = &reel.most_followed {
        lines.push(format!(
            "Scrutin le plus suivi : {} ({}).",
            followed.subject, followed.outcome_label
        ));
        lines.push(CRITERION_NOTE.to_owned());
    }
    lines.push(SHOW_OF_HANDS_NOTE.to_owned());
    lines.push(LINK_TOKEN.to_owned());
    Some(lines.join("\n"))
}

fn reason_of(plan: &ReelPlan) -> Option<&'static str> {
    match plan {
        ReelPlan::Votes(_) => None,
        ReelPlan::Skipped(SkipReason::NoVote) => Some("no_vote"),
        ReelPlan::Skipped(SkipReason::NoSubject) => Some("no_subject"),
        ReelPlan::Incomplete(VoteCompleteness::MissingNumbers { .. }) => Some("missing_numbers"),
        ReelPlan::Incomplete(_) => Some("not_synchronised"),
    }
}

impl From<PreparedReel> for ReelResponse {
    fn from(prepared: PreparedReel) -> Self {
        let slot = prepared.slot;
        let status = match prepared.plan {
            ReelPlan::Votes(_) => "ready",
            ReelPlan::Skipped(_) => "skipped",
            ReelPlan::Incomplete(_) => "incomplete",
        };
        let mut response = Self {
            status,
            kind: slot.kind().as_str(),
            last_success_at: prepared.last_success_at,
            reason: reason_of(&prepared.plan),
            empty_week_note: None,
            reconstructed_note: None,
            period: ReelPeriodDto {
                from: slot.from(),
                to: slot.to(),
                label: slot.period_label(),
            },
            due_at: due_at(&slot),
            totals: None,
            days: vec![],
            final_votes: vec![],
            most_followed: None,
            caption: None,
            link_path: None,
            tags: TAGS.to_vec(),
        };
        if let ReelPlan::Votes(reel) = prepared.plan {
            response.empty_week_note = (reel.totals.votes == 0).then_some(EMPTY_WEEK_NOTE);
            response.reconstructed_note =
                reel.has_reconstructed_tallies.then_some(RECONSTRUCTED_NOTE);
            response.caption = caption(&slot, &reel);
            response.days = reel
                .days
                .iter()
                .map(|day| ReelDayDto {
                    date: day.date,
                    votes: day.votes,
                })
                .collect();
            response.link_path = Some(link_path(&slot, &response.days));
            response.totals = Some(reel.totals.into());
            response.final_votes = reel
                .final_votes
                .into_iter()
                .map(|vote| ReelFinalVoteDto {
                    scrutin_uid: vote.scrutin_uid,
                    number: vote.number,
                    date: vote.date,
                    text_label: vote.text_label,
                    outcome_code: vote.outcome_code,
                    outcome_label: vote.outcome_label,
                })
                .collect();
            response.most_followed = reel.most_followed.map(MostFollowedDto::from);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::{json, Value};

    use super::*;
    use crate::api::scrutin_dto::{RECONSTRUCTED_NOTE, SHOW_OF_HANDS_NOTE};
    use crate::application::use_cases::final_vote_fakes::final_vote_record;
    use crate::application::use_cases::get_week_of_votes::{OtherOutcome, OutcomeCount};
    use crate::application::use_cases::prepare_reel::{
        ReelDay, ReelPlan, SkipReason, VotesReel,
    };
    use crate::application::use_cases::read_fakes::scrutin_row;
    use crate::domain::reel::{ReelSlot, VoteCompleteness};
    use crate::domain::scrutin::VoteTally;

    fn d(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn synchronised_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 21, 6, 0, 0).unwrap()
    }

    fn prepared(today: NaiveDate, plan: ReelPlan) -> PreparedReel {
        PreparedReel {
            slot: ReelSlot::for_day(today),
            last_success_at: Some(synchronised_at()),
            plan,
        }
    }

    fn json_of(prepared: PreparedReel) -> Value {
        serde_json::to_value(ReelResponse::from(prepared)).unwrap()
    }

    fn totals() -> OutcomeCount {
        OutcomeCount {
            votes: 6,
            adopted: 3,
            rejected: 2,
            others: vec![OtherOutcome {
                label: "retiré".into(),
                count: 1,
            }],
        }
    }

    fn day(month: u32, date: u32, votes: usize) -> ReelDay {
        ReelDay {
            date: d(month, date),
            votes,
        }
    }

    fn reel(days: Vec<ReelDay>) -> VotesReel {
        let mut followed = scrutin_row("VTANR5L17V5012", "l'article 3");
        followed.number = "5012".into();
        followed.date = d(11, 20);
        followed.tally = VoteTally {
            votes_for: 300,
            votes_against: 150,
            abstentions: 30,
            ..VoteTally::default()
        };
        VotesReel {
            totals: totals(),
            final_votes: vec![final_vote_record("100", d(11, 20))],
            most_followed: Some(followed),
            days,
            has_reconstructed_tallies: false,
        }
    }

    fn ready(today: NaiveDate, days: Vec<ReelDay>) -> Value {
        json_of(prepared(today, ReelPlan::Votes(reel(days))))
    }

    #[test]
    fn maps_a_ready_daily_reel_with_its_period_totals_final_votes_and_most_followed_scrutin() {
        let body = ready(d(11, 21), vec![day(11, 20, 6)]);

        assert_eq!(body["status"], "ready");
        assert_eq!(body["kind"], "veille");
        assert_eq!(body["reason"], Value::Null);
        assert_eq!(body["empty_week_note"], Value::Null);
        assert_eq!(body["period"]["from"], "2026-11-20");
        assert_eq!(body["period"]["to"], "2026-11-20");
        assert_eq!(body["period"]["label"], "vendredi 20 novembre");
        assert_eq!(
            body["totals"],
            json!({"votes": 6, "adopted": 3, "rejected": 2,
                "others": [{"label": "retiré", "count": 1}]})
        );
        assert_eq!(body["days"], json!([{"date": "2026-11-20", "votes": 6}]));
        assert_eq!(body["final_votes"][0]["scrutin_uid"], "VTANR5L17V100");
        assert_eq!(body["final_votes"][0]["number"], "100");
        assert_eq!(body["final_votes"][0]["date"], "2026-11-20");
        assert_eq!(body["most_followed"]["uid"], "VTANR5L17V5012");
        assert_eq!(body["most_followed"]["number"], "5012");
        assert_eq!(body["most_followed"]["tally"]["votes_for"], 300);
        assert_eq!(body["most_followed"]["tally"]["votes_against"], 150);
        assert_eq!(body["most_followed"]["tally"]["abstentions"], 30);
        assert_eq!(body["tags"], json!(["#QuiVoteQuoi", "#AssembléeNationale"]));
    }

    #[test]
    fn maps_skipped_and_incomplete_plans_with_their_reason() {
        let cases = [
            (d(11, 21), ReelPlan::Skipped(SkipReason::NoVote), "skipped", "no_vote"),
            (
                d(11, 23),
                ReelPlan::Skipped(SkipReason::NoSubject),
                "skipped",
                "no_subject",
            ),
            (
                d(11, 21),
                ReelPlan::Incomplete(VoteCompleteness::NotSynchronisedSince { last_success: None }),
                "incomplete",
                "not_synchronised",
            ),
            (
                d(11, 21),
                ReelPlan::Incomplete(VoteCompleteness::MissingNumbers { missing: 2 }),
                "incomplete",
                "missing_numbers",
            ),
        ];

        for (today, plan, status, reason) in cases {
            let body = json_of(prepared(today, plan));

            assert_eq!(body["status"], status);
            assert_eq!(body["reason"], reason);
            assert_eq!(body["totals"], Value::Null);
            assert_eq!(body["caption"], Value::Null);
            assert_eq!(body["link_path"], Value::Null);
        }
        let monday = json_of(prepared(d(11, 23), ReelPlan::Skipped(SkipReason::NoSubject)));
        assert_eq!(monday["kind"], "lundi");
    }

    #[test]
    fn sets_the_due_time_at_six_pm_in_paris_in_summer_and_in_winter() {
        let plan = || ReelPlan::Skipped(SkipReason::NoVote);

        let summer = json_of(prepared(d(7, 4), plan()));
        let winter = json_of(prepared(d(11, 21), plan()));

        assert_eq!(summer["due_at"], "2026-07-04T18:00:00+02:00");
        assert_eq!(winter["due_at"], "2026-11-21T18:00:00+01:00");
    }

    #[test]
    fn links_a_single_voting_day_to_the_week_page_filtered_on_that_day() {
        let body = ready(d(11, 21), vec![day(11, 20, 6)]);

        assert_eq!(body["link_path"], "/semaine/2026-11-16?jour=2026-11-20");
    }

    #[test]
    fn links_a_period_of_several_voting_days_to_the_week_page() {
        let days = vec![
            day(11, 16, 0),
            day(11, 17, 3),
            day(11, 18, 0),
            day(11, 19, 3),
        ];

        let body = ready(d(11, 22), days);

        assert_eq!(body["link_path"], "/semaine/2026-11-16");
    }

    #[test]
    fn links_a_tuesday_covering_sunday_and_monday_to_the_week_of_monday() {
        let body = ready(d(11, 24), vec![day(11, 22, 1), day(11, 23, 5)]);

        assert_eq!(body["kind"], "veille");
        assert_eq!(body["link_path"], "/semaine/2026-11-23");
    }

    #[test]
    fn carries_the_no_vote_this_week_note() {
        let mut empty = reel((16..=21).map(|n| day(11, n, 0)).collect());
        empty.totals = OutcomeCount {
            votes: 0,
            adopted: 0,
            rejected: 0,
            others: vec![],
        };
        empty.most_followed = None;
        empty.final_votes = vec![];

        let body = json_of(prepared(d(11, 22), ReelPlan::Votes(empty)));

        assert_eq!(body["status"], "ready");
        assert_eq!(body["kind"], "semaine");
        assert_eq!(body["empty_week_note"], EMPTY_WEEK_NOTE);
        assert_ne!(EMPTY_WEEK_NOTE, "");
    }

    #[test]
    fn carries_the_last_successful_synchronisation_in_every_response() {
        let plans = [
            ReelPlan::Votes(reel(vec![day(11, 20, 6)])),
            ReelPlan::Skipped(SkipReason::NoVote),
            ReelPlan::Incomplete(VoteCompleteness::MissingNumbers { missing: 1 }),
        ];

        for plan in plans {
            let body = json_of(prepared(d(11, 21), plan));

            assert_eq!(body["last_success_at"], "2026-11-21T06:00:00Z");
        }
    }

    #[test]
    fn writes_the_caption_with_period_numbers_criterion_show_of_hands_note_and_link_token() {
        let body = ready(d(11, 21), vec![day(11, 20, 6)]);

        assert!(body["caption"].is_string());
        let caption = body["caption"].as_str().unwrap_or_default();

        assert!(caption.contains("vendredi 20 novembre"));
        assert!(caption.contains("6 votes"));
        assert!(caption.contains("3 adoptés"));
        assert!(caption.contains("2 rejetés"));
        assert!(caption.contains("l'article 3"));
        assert_ne!(CRITERION_NOTE, "");
        assert!(caption.contains(CRITERION_NOTE));
        assert!(caption.contains(SHOW_OF_HANDS_NOTE));
        assert_ne!(LINK_TOKEN, "");
        assert!(caption.contains(LINK_TOKEN));
        for banned in ["massivement", "largement", "seulement"] {
            assert!(!caption.contains(banned), "{banned}");
        }
    }

    #[test]
    fn carries_the_reconstructed_tally_note_when_flagged() {
        let mut flagged = reel(vec![day(11, 20, 6)]);
        flagged.has_reconstructed_tallies = true;

        let with = json_of(prepared(d(11, 21), ReelPlan::Votes(flagged)));
        let without = ready(d(11, 21), vec![day(11, 20, 6)]);

        assert_eq!(with["reconstructed_note"], RECONSTRUCTED_NOTE);
        assert_eq!(without["reconstructed_note"], Value::Null);
    }
}
