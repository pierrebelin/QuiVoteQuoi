//! DTO du Reel d'un jour (RM-10, RM-11, RM-12, RM-14, RM-16).
use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Paris;
use serde::{Deserialize, Serialize};

use crate::api::scrutin_dto::{RECONSTRUCTED_NOTE, SHOW_OF_HANDS_NOTE};
use crate::api::week_dto::OutcomeCountDto;
use crate::application::ports::scrutin_repository::ScrutinSummary;
use crate::application::use_cases::prepare_reel::{
    PreparedReel, PresentedSubjects, ProgrammesReel, ReelPlan, SkipReason, VotesReel,
};
use crate::domain::reel::{weekday_name, ReelSlot, VoteCompleteness};
use crate::domain::site_news::SiteNewsId;
use crate::domain::theme::FamilyCode;

pub const EMPTY_WEEK_NOTE: &str = "Aucun vote à l'Assemblée nationale cette semaine.";
pub const CRITERION_NOTE: &str =
    "Critère : le scrutin qui compte le plus de votants (pour, contre, abstention).";
pub const MAX_PRESENTED_NEWS: usize = 500;
const PROGRAMMES_PATH: &str = "/candidats-2027";
const THEME_METHOD_PATH: &str = "/themes/methode";
pub const LINK_TOKEN: &str = "[lien]";
pub const TAGS: [&str; 2] = ["#QuiVoteQuoi", "#AssembléeNationale"];

/// Heure de publication du Reel, a Paris (RM-12).
const DUE_HOUR: u32 = 18;

/// Sujets deja presentes, lus dans l'adresse (RM-07, RM-08).
#[derive(Debug, Default, Deserialize)]
pub struct ReelQuery {
    pub dernier_theme: Option<String>,
    pub nouveautes_presentees: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReelQueryError {
    #[error("unknown theme code: {0}")]
    UnknownTheme(String),
    #[error("too many presented site news")]
    TooManyNews,
}

impl TryFrom<ReelQuery> for PresentedSubjects {
    type Error = ReelQueryError;

    fn try_from(query: ReelQuery) -> Result<Self, Self::Error> {
        let last_theme = query
            .dernier_theme
            .filter(|code| !code.trim().is_empty())
            .map(|code| {
                FamilyCode::parse(code.trim()).map_err(|_| ReelQueryError::UnknownTheme(code))
            })
            .transpose()?;
        let tokens: Vec<&str> = query
            .nouveautes_presentees
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .collect();
        if tokens.len() > MAX_PRESENTED_NEWS {
            return Err(ReelQueryError::TooManyNews);
        }
        let news = tokens
            .into_iter()
            .filter_map(|token| SiteNewsId::new(token.to_owned()).ok())
            .collect();
        Ok(Self { last_theme, news })
    }
}

#[derive(Debug, Serialize)]
pub struct ReelProgrammeDto {
    pub candidate_id: String,
    pub name: String,
    pub added: usize,
    pub removed: usize,
    pub themes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ReelSiteNewsDto {
    pub id: String,
    pub title: String,
    pub text: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ReelThemeDto {
    pub code: &'static str,
    pub label: &'static str,
    pub text_count: i64,
    pub scrutin_count: i64,
}

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

pub const COMPLETENESS_RULE: &str = "Un Reel de votes n'est publiable que si les données sont \
     complètes : synchronisation de l'Assemblée nationale postérieure à la fin de la période et \
     aucun numéro de scrutin manquant (RM-09).";
pub const MONDAY_PRIORITY: &str = "programmes, nouveauté, thème";
const DUE_TIME: &str = "18:00";

#[derive(Debug, Serialize)]
pub struct ReelMethodDayDto {
    pub weekday: &'static str,
    pub kind: &'static str,
    pub period_label: String,
}

impl From<ReelSlot> for ReelMethodDayDto {
    fn from(slot: ReelSlot) -> Self {
        let prepared_on = slot.to().checked_add_days(Days::new(1)).unwrap_or(slot.to());
        Self {
            weekday: weekday_name(prepared_on),
            kind: slot.kind().as_str(),
            period_label: slot.period_label(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ReelsMethodResponse {
    pub days: Vec<ReelMethodDayDto>,
    pub theme_rotation: Vec<FamilyCode>,
    pub due_time: String,
    pub most_followed_rule: &'static str,
    pub completeness_rule: &'static str,
    pub monday_priority: &'static str,
}

impl From<[ReelSlot; 7]> for ReelsMethodResponse {
    fn from(slots: [ReelSlot; 7]) -> Self {
        Self {
            days: slots.into_iter().map(ReelMethodDayDto::from).collect(),
            theme_rotation: FamilyCode::ALL.to_vec(),
            due_time: DUE_TIME.to_owned(),
            most_followed_rule: CRITERION_NOTE,
            completeness_rule: COMPLETENESS_RULE,
            monday_priority: MONDAY_PRIORITY,
        }
    }
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
    pub programmes: Vec<ReelProgrammeDto>,
    pub site_news: Option<ReelSiteNewsDto>,
    pub theme: Option<ReelThemeDto>,
    pub caption: Option<String>,
    pub link_path: Option<String>,
    pub tags: Vec<String>,
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

/// Legende d'un lundi sans vote: une phrase neutre, puis le lien.
fn subject_caption(headline: String) -> String {
    format!("{headline}\n{LINK_TOKEN}")
}

fn programmes_of(reel: ProgrammesReel, slot: &ReelSlot) -> (Vec<ReelProgrammeDto>, String) {
    let caption = subject_caption(format!(
        "Programmes 2027, {} : {}.",
        slot.period_label(),
        counted(reel.candidates.len(), "candidature a modifié", "candidatures ont modifié")
            + " son programme"
    ));
    let programmes = reel
        .candidates
        .into_iter()
        .map(|candidate| ReelProgrammeDto {
            candidate_id: candidate.candidate_id.as_str().to_owned(),
            name: candidate.name,
            added: candidate.added,
            removed: candidate.removed,
            themes: candidate.themes.iter().map(FamilyCode::as_str).collect(),
        })
        .collect();
    (programmes, caption)
}

fn reason_of(plan: &ReelPlan) -> Option<&'static str> {
    match plan {
        ReelPlan::Votes(_)
        | ReelPlan::Programmes(_)
        | ReelPlan::SiteNews(_)
        | ReelPlan::Theme(_) => None,
        ReelPlan::Skipped(SkipReason::NoVote) => Some("no_vote"),
        ReelPlan::Incomplete(VoteCompleteness::MissingNumbers { .. }) => Some("missing_numbers"),
        ReelPlan::Incomplete(_) => Some("not_synchronised"),
    }
}

impl From<PreparedReel> for ReelResponse {
    fn from(prepared: PreparedReel) -> Self {
        let slot = prepared.slot;
        let status = match prepared.plan {
            ReelPlan::Votes(_)
            | ReelPlan::Programmes(_)
            | ReelPlan::SiteNews(_)
            | ReelPlan::Theme(_) => "ready",
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
            programmes: vec![],
            site_news: None,
            theme: None,
            caption: None,
            link_path: None,
            tags: TAGS.iter().map(|tag| (*tag).to_owned()).collect(),
        };
        match prepared.plan {
            ReelPlan::Votes(reel) => response.fill_votes(&slot, reel),
            ReelPlan::Programmes(reel) => {
                let (programmes, caption) = programmes_of(reel, &slot);
                response.programmes = programmes;
                response.caption = Some(caption);
                response.link_path = Some(PROGRAMMES_PATH.to_owned());
            }
            ReelPlan::SiteNews(reel) => {
                let news = reel.news;
                response.caption = Some(subject_caption(format!("{}\n{}", news.title, news.text)));
                response.link_path = Some(news.path.clone());
                response.tags.push(format!("reel-nouveaute-{}", news.id.as_str()));
                response.site_news = Some(ReelSiteNewsDto {
                    id: news.id.as_str().to_owned(),
                    title: news.title,
                    text: news.text,
                    path: news.path,
                });
            }
            ReelPlan::Theme(reel) => {
                response.caption = Some(subject_caption(format!(
                    "{} : {} textes, {} scrutins classés.",
                    reel.theme.label(),
                    reel.text_count,
                    reel.scrutin_count
                )));
                response.link_path = Some(THEME_METHOD_PATH.to_owned());
                response.tags.push(format!("reel-theme-{}", reel.theme.as_str()));
                response.theme = Some(ReelThemeDto {
                    code: reel.theme.as_str(),
                    label: reel.theme.label(),
                    text_count: reel.text_count,
                    scrutin_count: reel.scrutin_count,
                });
            }
            ReelPlan::Skipped(_) | ReelPlan::Incomplete(_) => {}
        }
        response
    }
}

impl ReelResponse {
    fn fill_votes(&mut self, slot: &ReelSlot, reel: VotesReel) {
        self.empty_week_note = (reel.totals.votes == 0).then_some(EMPTY_WEEK_NOTE);
        self.reconstructed_note = reel.has_reconstructed_tallies.then_some(RECONSTRUCTED_NOTE);
        self.caption = caption(slot, &reel);
        self.days = reel
            .days
            .iter()
            .map(|day| ReelDayDto {
                date: day.date,
                votes: day.votes,
            })
            .collect();
        self.link_path = Some(link_path(slot, &self.days));
        self.totals = Some(reel.totals.into());
        self.final_votes = reel
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
        self.most_followed = reel.most_followed.map(MostFollowedDto::from);
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
        CandidateChanges, ProgrammesReel, ReelDay, ReelPlan, SiteNewsReel, SkipReason,
        ThemeReel, VotesReel,
    };
    use crate::domain::candidate::CandidateId;
    use crate::domain::site_news::{SiteNews, SiteNewsId};
    use crate::domain::theme::FamilyCode;
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

    fn monday() -> NaiveDate {
        d(11, 23)
    }

    fn site_news() -> SiteNews {
        SiteNews {
            id: SiteNewsId::new("semaine".into()).unwrap(),
            title: "La semaine à l'Assemblée".into(),
            text: "Une nouvelle page résume chaque semaine les votes.".into(),
            delivered_on: d(11, 13),
            path: "/semaine".into(),
        }
    }

    fn theme_reel() -> ReelPlan {
        ReelPlan::Theme(ThemeReel {
            slot: ReelSlot::for_day(monday()),
            theme: FamilyCode::Logement,
            text_count: 12,
            scrutin_count: 34,
        })
    }

    #[test]
    fn maps_a_programmes_reel_with_each_candidate_counts_and_themes() {
        let plan = ReelPlan::Programmes(ProgrammesReel {
            slot: ReelSlot::for_day(monday()),
            candidates: vec![CandidateChanges {
                candidate_id: CandidateId::new("c1".into()).unwrap(),
                name: "Jeanne Martin".into(),
                added: 3,
                removed: 1,
                themes: vec![FamilyCode::Logement, FamilyCode::Sante],
            }],
        });

        let body = json_of(prepared(monday(), plan));

        assert_eq!(body["status"], "ready");
        assert_eq!(body["kind"], "lundi");
        assert_eq!(body["programmes"][0]["name"], "Jeanne Martin");
        assert_eq!(body["programmes"][0]["added"], 3);
        assert_eq!(body["programmes"][0]["removed"], 1);
        assert_eq!(body["programmes"][0]["themes"], json!(["logement", "sante"]));
    }

    #[test]
    fn maps_a_site_news_reel_with_its_text_and_page_link() {
        let plan = ReelPlan::SiteNews(SiteNewsReel {
            slot: ReelSlot::for_day(monday()),
            news: site_news(),
        });

        let body = json_of(prepared(monday(), plan));

        assert_eq!(body["status"], "ready");
        assert_eq!(body["site_news"]["id"], "semaine");
        assert_eq!(body["site_news"]["title"], "La semaine à l'Assemblée");
        assert_eq!(
            body["site_news"]["text"],
            "Une nouvelle page résume chaque semaine les votes."
        );
        assert_eq!(body["site_news"]["path"], "/semaine");
        assert_eq!(body["link_path"], "/semaine");
    }

    #[test]
    fn maps_a_theme_reel_with_its_numbers_and_theme_method_page_link() {
        let body = json_of(prepared(monday(), theme_reel()));

        assert_eq!(body["status"], "ready");
        assert_eq!(body["theme"]["code"], "logement");
        assert_eq!(body["theme"]["label"], "Logement / urbanisme / foncier");
        assert_eq!(body["theme"]["text_count"], 12);
        assert_eq!(body["theme"]["scrutin_count"], 34);
        assert_eq!(body["link_path"], "/themes/methode");
    }

    #[test]
    fn tags_a_theme_or_site_news_reel_with_its_subject() {
        let theme = json_of(prepared(monday(), theme_reel()));
        let news = json_of(prepared(
            monday(),
            ReelPlan::SiteNews(SiteNewsReel {
                slot: ReelSlot::for_day(monday()),
                news: site_news(),
            }),
        ));

        assert!(theme["tags"].as_array().unwrap().contains(&json!("reel-theme-logement")));
        assert!(news["tags"].as_array().unwrap().contains(&json!("reel-nouveaute-semaine")));
        assert!(theme["tags"].as_array().unwrap().contains(&json!("#QuiVoteQuoi")));
    }

    #[test]
    fn reads_the_last_presented_theme_and_presented_site_news_from_the_query() {
        let query = ReelQuery {
            dernier_theme: Some("logement".into()),
            nouveautes_presentees: Some("semaine,budget".into()),
        };

        let presented = PresentedSubjects::try_from(query).unwrap();

        assert_eq!(presented.last_theme, Some(FamilyCode::Logement));
        let ids: Vec<&str> = presented.news.iter().map(SiteNewsId::as_str).collect();
        assert_eq!(ids, vec!["semaine", "budget"]);
        let empty = PresentedSubjects::try_from(ReelQuery::default()).unwrap();
        assert_eq!(empty.last_theme, None);
        assert!(empty.news.is_empty());
    }

    fn method_json() -> Value {
        let slots: [ReelSlot; 7] =
            std::array::from_fn(|i| ReelSlot::for_day(d(11, 16 + i as u32)));
        serde_json::to_value(ReelsMethodResponse::from(slots)).unwrap()
    }

    #[test]
    fn describes_the_reel_kind_and_period_of_each_day_of_the_week() {
        let json = method_json();

        let days = json["days"].as_array().unwrap();
        assert_eq!(days.len(), 7);
        assert_eq!(days[0]["kind"], "lundi");
        assert_eq!(
            days[1]["period_label"],
            ReelSlot::for_day(d(11, 17)).period_label()
        );
        assert_eq!(days[2]["kind"], "veille");
    }

    #[test]
    fn lists_the_theme_rotation_in_referential_order() {
        let json = method_json();

        assert_eq!(
            json["theme_rotation"],
            serde_json::to_value(FamilyCode::ALL).unwrap()
        );
    }

    #[test]
    fn states_the_due_time_the_most_followed_criterion_the_completeness_rule_and_the_monday_priority(
    ) {
        let json = method_json();

        assert_eq!(json["due_time"], "18:00");
        assert_eq!(json["most_followed_rule"], CRITERION_NOTE);
        assert_eq!(json["completeness_rule"], COMPLETENESS_RULE);
        assert_eq!(json["monday_priority"], MONDAY_PRIORITY);
        assert!(json.as_object().unwrap().keys().all(|k| !k.contains("prepar")));
    }
}
