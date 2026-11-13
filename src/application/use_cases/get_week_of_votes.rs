use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_citation_repository::AmendmentCitationRepository;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::application::ports::final_vote_repository::{FinalVoteRepository, GroupOption};
use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinRepository, ScrutinSummary,
};
use crate::application::ports::theme_repository::{AssignedFamily, ThemeRepository};
use crate::application::use_cases::browse_final_votes::{
    BrowseFinalVotes, BrowseFinalVotesCommand, BrowseFinalVotesError, FinalVoteEntry,
    MAX_PAGE_SIZE,
};
use crate::application::use_cases::list_scrutins::{ListScrutins, ScrutinListItem};
use crate::domain::dossier::LegislativeDossier;
use crate::domain::final_vote::is_final_vote_subject;
use crate::domain::scrutin::Outcome;
use crate::domain::week::{DayState, Week, WeekError};

/// Nombre de scrutins d'un sort publie sous un libelle propre (RM-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherOutcome {
    pub label: String,
    pub count: usize,
}

/// Nombres d'un ensemble de votes (RM-04, RM-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeCount {
    pub votes: usize,
    pub adopted: usize,
    pub rejected: usize,
    pub others: Vec<OtherOutcome>,
}

/// Dernier vote sur l'ensemble d'une carte ou d'un jour de carte (RM-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalOutcome {
    pub scrutin_uid: String,
    pub date: NaiveDate,
    pub number: String,
    pub outcome_code: String,
    pub outcome_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardTally {
    pub outcomes: OutcomeCount,
    pub latest_final: Option<FinalOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardDay {
    pub date: NaiveDate,
    pub tally: CardTally,
}

/// Carte d'un texte: les votes de la semaine rattaches a un dossier, ou ceux
/// qui n'en ont pas (RM-08, RM-09, RM-10).
#[derive(Debug, Clone)]
pub struct TextCard {
    pub dossier_uid: Option<String>,
    pub dossier_label: Option<String>,
    pub dossier: Option<LegislativeDossier>,
    pub families: Vec<AssignedFamily>,
    pub items: Vec<ScrutinListItem>,
    pub totals: CardTally,
    pub days: Vec<CardDay>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekDay {
    pub date: NaiveDate,
    pub state: DayState,
}

/// Semaine voisine, avec son nombre de votes (RM-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekNeighbour {
    pub monday: NaiveDate,
    pub votes_count: i64,
}

#[derive(Debug, Clone)]
pub struct WeekOfVotes {
    pub week: Week,
    pub is_current: bool,
    pub current_week_empty: bool,
    pub last_published: Option<NaiveDate>,
    pub totals: OutcomeCount,
    pub texts_count: usize,
    pub days: Vec<WeekDay>,
    pub final_votes: Vec<FinalVoteEntry>,
    pub groups: Vec<GroupOption>,
    pub families: HashMap<String, Vec<AssignedFamily>>,
    pub texts: Vec<TextCard>,
    pub previous: Option<WeekNeighbour>,
    pub next: Option<WeekNeighbour>,
}

#[derive(Debug, thiserror::Error)]
pub enum GetWeekOfVotesError {
    #[error(transparent)]
    Unavailable(#[from] WeekError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    FinalVotes(#[from] BrowseFinalVotesError),
}

/// CU-01, CU-04 — Consulter la semaine demandee ou par defaut.
pub struct GetWeekOfVotes<'a> {
    repository: &'a dyn ScrutinRepository,
    citations: &'a dyn AmendmentCitationRepository,
    actors: &'a dyn ActorRepository,
    dossiers: &'a dyn DossierRepository,
    final_votes: &'a dyn FinalVoteRepository,
    themes: &'a dyn ThemeRepository,
}

impl<'a> GetWeekOfVotes<'a> {
    pub fn new(
        repository: &'a dyn ScrutinRepository,
        citations: &'a dyn AmendmentCitationRepository,
        actors: &'a dyn ActorRepository,
        dossiers: &'a dyn DossierRepository,
        final_votes: &'a dyn FinalVoteRepository,
        themes: &'a dyn ThemeRepository,
    ) -> Self {
        Self {
            repository,
            citations,
            actors,
            dossiers,
            final_votes,
            themes,
        }
    }

    pub async fn execute(
        &self,
        requested: Option<NaiveDate>,
        today: NaiveDate,
    ) -> Result<WeekOfVotes, GetWeekOfVotesError> {
        let last_published = self.repository.latest_scrutin_date().await?;

        let current = Week::current(today);
        let current_week_empty = last_published.is_none_or(|last| last < current.monday());
        let week = match (requested, last_published) {
            (Some(date), _) => Week::requested(date, today)?,
            (None, Some(last)) if current_week_empty => Week::requested(last, today)?,
            _ => current,
        };

        let mut items = ListScrutins::new(
            self.repository,
            self.citations,
            self.actors,
            self.dossiers,
        )
        .execute_between(week.monday(), week.sunday())
        .await?;
        items.sort_by_key(|item| vote_order(&item.summary));
        let rows: Vec<&ScrutinSummary> = items.iter().map(|item| &item.summary).collect();

        let uids: Vec<String> = rows.iter().map(|row| row.uid.clone()).collect();
        let families = if uids.is_empty() {
            HashMap::new()
        } else {
            self.themes.families_of_scrutins(&uids).await?
        };

        let mut final_view = BrowseFinalVotes::new(self.final_votes)
            .execute(BrowseFinalVotesCommand {
                all_groups: true,
                from: Some(week.monday()),
                to: Some(week.sunday()),
                limit: Some(MAX_PAGE_SIZE),
                ..BrowseFinalVotesCommand::default()
            })
            .await?;
        final_view
            .items
            .sort_by_key(|item| vote_key(item.vote.date(), item.vote.number()));

        let votes_on = |day: NaiveDate| rows.iter().filter(|row| row.date == day).count();
        let days = week
            .days(votes_on)
            .into_iter()
            .map(|date| {
                let votes = votes_on(date);
                WeekDay {
                    date,
                    state: DayState::of(date, votes, last_published, today),
                }
            })
            .collect();

        let previous = self.neighbour(week.previous()).await?;
        let next = self.neighbour(week.next(today)).await?;

        Ok(WeekOfVotes {
            week,
            is_current: week.is_current(today),
            current_week_empty,
            last_published,
            totals: totals_of(&rows),
            texts_count: distinct_texts(&rows),
            days,
            final_votes: final_view.items,
            groups: final_view.selected,
            texts: cards_of(items, &families),
            families,
            previous,
            next,
        })
    }

    async fn neighbour(
        &self,
        week: Option<Week>,
    ) -> Result<Option<WeekNeighbour>, RepositoryError> {
        let Some(week) = week else {
            return Ok(None);
        };
        let votes_count = self
            .repository
            .count_between(week.monday(), week.sunday())
            .await?;
        Ok(Some(WeekNeighbour {
            monday: week.monday(),
            votes_count,
        }))
    }
}

/// Sorts publies de la semaine: adopte, rejete, et chaque autre sous son libelle (RM-04).
fn totals_of(rows: &[&ScrutinSummary]) -> OutcomeCount {
    let mut totals = OutcomeCount {
        votes: rows.len(),
        adopted: 0,
        rejected: 0,
        others: Vec::new(),
    };
    for row in rows.iter().copied() {
        if row.outcome_code == Outcome::ADOPTED {
            totals.adopted += 1;
        } else if row.outcome_code == Outcome::REJECTED {
            totals.rejected += 1;
        } else if let Some(other) = totals
            .others
            .iter_mut()
            .find(|other| other.label == row.outcome_label)
        {
            other.count += 1;
        } else {
            totals.others.push(OtherOutcome {
                label: row.outcome_label.clone(),
                count: 1,
            });
        }
    }
    totals
}

/// Textes distincts de la semaine, votes sans dossier exclus (RM-05).
fn distinct_texts(rows: &[&ScrutinSummary]) -> usize {
    rows.iter()
        .filter_map(|row| row.dossier_uid.as_deref())
        .collect::<HashSet<_>>()
        .len()
}

/// Cle de rang d'un vote: date, puis numero lu comme un nombre (99 avant 100).
fn vote_key(date: NaiveDate, number: &str) -> (NaiveDate, u64, String) {
    (date, number.parse().unwrap_or(u64::MAX), number.to_string())
}

fn vote_order(summary: &ScrutinSummary) -> (NaiveDate, u64, String) {
    vote_key(summary.date, &summary.number)
}

/// Une carte par dossier, dans l'ordre de leur premier vote; les votes sans
/// dossier forment la derniere carte (RM-08, RM-09). `items` est deja rangé
/// par date puis numero (RM-10).
fn cards_of(
    items: Vec<ScrutinListItem>,
    families: &HashMap<String, Vec<AssignedFamily>>,
) -> Vec<TextCard> {
    let mut grouped: Vec<(Option<String>, Vec<ScrutinListItem>)> = Vec::new();
    for item in items {
        let key = item.summary.dossier_uid.clone();
        match grouped.iter_mut().find(|(uid, _)| *uid == key) {
            Some((_, members)) => members.push(item),
            None => grouped.push((key, vec![item])),
        }
    }
    grouped.sort_by_key(|(uid, _)| uid.is_none());
    grouped
        .into_iter()
        .map(|(dossier_uid, items)| card_of(dossier_uid, items, families))
        .collect()
}

fn card_of(
    dossier_uid: Option<String>,
    items: Vec<ScrutinListItem>,
    families: &HashMap<String, Vec<AssignedFamily>>,
) -> TextCard {
    let mut card_families: Vec<AssignedFamily> = Vec::new();
    for family in items
        .iter()
        .filter_map(|item| families.get(&item.summary.uid))
        .flatten()
    {
        if !card_families.iter().any(|kept| kept.family == family.family) {
            card_families.push(family.clone());
        }
    }

    let mut dates: Vec<NaiveDate> = items.iter().map(|item| item.summary.date).collect();
    dates.dedup();
    let days = dates
        .into_iter()
        .map(|date| CardDay {
            date,
            tally: tally_of(
                &items
                    .iter()
                    .map(|item| &item.summary)
                    .filter(|summary| summary.date == date)
                    .collect::<Vec<_>>(),
            ),
        })
        .collect();

    let summaries: Vec<&ScrutinSummary> = items.iter().map(|item| &item.summary).collect();
    TextCard {
        dossier_label: summaries.iter().find_map(|s| s.dossier_label.clone()),
        dossier: items.iter().find_map(|item| item.dossier.clone()),
        families: card_families,
        totals: tally_of(&summaries),
        days,
        dossier_uid,
        items,
    }
}

/// Sorts comptes et dernier vote sur l'ensemble de ces votes (RM-10).
fn tally_of(rows: &[&ScrutinSummary]) -> CardTally {
    let latest_final = rows
        .iter()
        .copied()
        .filter(|row| is_final_vote_subject(&row.subject))
        .max_by_key(|row| vote_order(row))
        .map(|row| FinalOutcome {
            scrutin_uid: row.uid.clone(),
            date: row.date,
            number: row.number.clone(),
            outcome_code: row.outcome_code.clone(),
            outcome_label: row.outcome_label.clone(),
        });
    CardTally {
        outcomes: totals_of(rows),
        latest_final,
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::application::ports::final_vote_repository::{
        FinalVoteFilter, FinalVotePage, FinalVoteRecord, FinalVoteTotals, GroupTallyRecord,
    };
    use crate::application::ports::scrutin_repository::ScrutinSummary;
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::application::use_cases::read_fakes::{
        scrutin_row, FakeActors, FakeDossiers, FakeScrutins,
    };
    use crate::application::use_cases::theme_fakes::InMemoryThemeRepository;
    use crate::domain::dataset::DatasetShape;
    use crate::domain::scrutin::{VotePosition, VoteTally};
    use crate::domain::theme::FamilyCode;

    // vendredi 13 novembre 2026
    fn today() -> NaiveDate {
        d(13)
    }

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn vote(uid: &str, day: u32, code: &str, label: &str, dossier: Option<&str>) -> ScrutinSummary {
        let mut row = scrutin_row(uid, "l'ensemble du projet de loi");
        row.date = d(day);
        row.outcome_code = code.into();
        row.outcome_label = label.into();
        row.dossier_uid = dossier.map(String::from);
        row
    }

    fn adopted(uid: &str, day: u32) -> ScrutinSummary {
        vote(uid, day, "adopt\u{00e9}", "adopt\u{00e9}", None)
    }

    /// Lignes de la plus recente a la plus ancienne, comme la liste publiee.
    fn repository(mut rows: Vec<ScrutinSummary>) -> FakeScrutins {
        rows.sort_by(|a, b| b.date.cmp(&a.date));
        FakeScrutins {
            rows,
            total: 0,
            shape: DatasetShape::default(),
        }
    }

    async fn week_of(
        rows: Vec<ScrutinSummary>,
        requested: Option<NaiveDate>,
    ) -> Result<WeekOfVotes, GetWeekOfVotesError> {
        week_with(rows, vec![], vec![], HashMap::new(), requested).await
    }

    async fn week_with(
        rows: Vec<ScrutinSummary>,
        records: Vec<FinalVoteRecord>,
        groups: Vec<GroupOption>,
        families: HashMap<String, Vec<AssignedFamily>>,
        requested: Option<NaiveDate>,
    ) -> Result<WeekOfVotes, GetWeekOfVotesError> {
        let repository = repository(rows);
        let final_votes = FakeFinalVotes { records, groups };
        let themes = InMemoryThemeRepository::default();
        *themes.scrutin_families.lock().unwrap() = families;
        GetWeekOfVotes::new(
            &repository,
            &InMemoryCitationRepository::default(),
            &FakeActors { group_color: None },
            &FakeDossiers::default(),
            &final_votes,
            &themes,
        )
        .execute(requested, today())
        .await
    }

    // Local: lit les enregistrements dans l'ordre donne, sans tri, pour que
    // l'ordre date puis numero soit celui du cas d'usage et non du depot.
    struct FakeFinalVotes {
        records: Vec<FinalVoteRecord>,
        groups: Vec<GroupOption>,
    }

    #[async_trait]
    impl FinalVoteRepository for FakeFinalVotes {
        async fn list_final_votes(
            &self,
            filter: &FinalVoteFilter,
        ) -> Result<FinalVotePage, RepositoryError> {
            let items: Vec<FinalVoteRecord> = self
                .records
                .iter()
                .filter(|record| filter.from.is_none_or(|from| record.date >= from))
                .filter(|record| filter.to.is_none_or(|to| record.date <= to))
                .take(filter.limit as usize)
                .cloned()
                .collect();
            Ok(FinalVotePage {
                total: items.len() as i64,
                items,
            })
        }

        async fn groups(&self) -> Result<Vec<GroupOption>, RepositoryError> {
            Ok(self.groups.clone())
        }

        async fn totals(&self) -> Result<FinalVoteTotals, RepositoryError> {
            Ok(FinalVoteTotals {
                total: self.records.len() as i64,
                with_family: 0,
            })
        }
    }

    /// Vote numerote, d'un dossier ou sans, sur un article (pas un vote sur l'ensemble).
    fn numbered(uid: &str, day: u32, number: &str, dossier: Option<&str>) -> ScrutinSummary {
        let mut row = adopted(uid, day);
        row.number = number.into();
        row.dossier_uid = dossier.map(String::from);
        row.dossier_label = dossier.map(|d| format!("Texte {d}"));
        row.subject = "l'article 3 du projet de loi".into();
        row
    }

    /// Vote sur l'ensemble, au sort donne.
    fn final_row(uid: &str, day: u32, number: &str, code: &str, dossier: &str) -> ScrutinSummary {
        let mut row = numbered(uid, day, number, Some(dossier));
        row.subject = "l'ensemble du projet de loi".into();
        row.outcome_code = code.into();
        row.outcome_label = code.into();
        row
    }

    fn uids(items: &[ScrutinListItem]) -> Vec<&str> {
        items.iter().map(|i| i.summary.uid.as_str()).collect()
    }

    fn family(code: FamilyCode) -> AssignedFamily {
        AssignedFamily {
            family: code,
            opened_on: d(1),
            motive: None,
        }
    }

    fn group(uid: &str, abbrev: &str, count: i64) -> GroupOption {
        GroupOption {
            uid: uid.into(),
            abbrev: abbrev.into(),
            label: format!("Groupe {abbrev}"),
            color: None,
            final_vote_count: count,
        }
    }

    fn tally_of(uid: &str, abbrev: &str, position: &str) -> GroupTallyRecord {
        GroupTallyRecord {
            group_uid: uid.into(),
            abbrev: abbrev.into(),
            label: format!("Groupe {abbrev}"),
            color: None,
            member_count: Some(50),
            majority_position: Some(position.into()),
            tally: VoteTally::default(),
        }
    }

    fn record(uid: &str, day: u32, number: &str, tallies: Vec<GroupTallyRecord>) -> FinalVoteRecord {
        FinalVoteRecord {
            scrutin_uid: uid.into(),
            number: number.into(),
            date: d(day),
            subject: "l'ensemble du projet de loi".into(),
            ballot_type_label: "scrutin public solennel".into(),
            outcome_code: "adopt\u{00e9}".into(),
            outcome_label: "adopt\u{00e9}".into(),
            text_key: "projet de loi".into(),
            text_label: "projet de loi".into(),
            dossier_uid: None,
            dossier_label: None,
            official_text: None,
            synthesis: VoteTally::default(),
            families: vec![],
            tallies,
        }
    }

    #[tokio::test]
    async fn opens_the_current_week_when_it_has_a_published_vote() {
        let view = week_of(vec![adopted("S1", 10), adopted("S0", 4)], None)
            .await
            .unwrap();

        assert_eq!(view.week.monday(), d(9));
        assert!(view.is_current);
        assert!(!view.current_week_empty);
        assert_eq!(view.last_published, Some(d(10)));
    }

    #[tokio::test]
    async fn opens_the_week_of_the_last_published_vote_with_the_empty_current_week_notice() {
        let view = week_of(vec![adopted("S1", 5), adopted("S0", 3)], None)
            .await
            .unwrap();

        assert_eq!(view.week.monday(), d(2));
        assert!(!view.is_current);
        assert!(view.current_week_empty);
        assert_eq!(view.last_published, Some(d(5)));
    }

    #[tokio::test]
    async fn opens_the_current_week_with_the_empty_notice_when_no_vote_is_published() {
        let view = week_of(vec![], None).await.unwrap();

        assert_eq!(view.week.monday(), d(9));
        assert!(view.is_current);
        assert!(view.current_week_empty);
        assert_eq!(view.last_published, None);
    }

    #[tokio::test]
    async fn opens_the_week_of_a_requested_date() {
        let view = week_of(
            vec![adopted("S1", 10), adopted("S2", 4), adopted("S3", 3)],
            Some(d(4)),
        )
        .await
        .unwrap();

        assert_eq!(view.week.monday(), d(2));
        assert!(!view.is_current);
        assert!(!view.current_week_empty);
        assert_eq!(view.totals.votes, 2);
    }

    #[tokio::test]
    async fn refuses_a_requested_date_after_the_current_week() {
        let error = week_of(vec![adopted("S1", 10)], Some(d(16)))
            .await
            .unwrap_err();

        assert!(matches!(error, GetWeekOfVotesError::Unavailable(_)));
    }

    #[tokio::test]
    async fn counts_adopted_rejected_and_other_outcomes_under_their_published_label() {
        let view = week_of(
            vec![
                adopted("S1", 10),
                adopted("S2", 10),
                vote("S3", 11, "rejet\u{00e9}", "rejet\u{00e9}", None),
                vote("S4", 11, "irrecevable", "irrecevable", None),
            ],
            None,
        )
        .await
        .unwrap();

        assert_eq!(view.totals.votes, 4);
        assert_eq!(view.totals.adopted, 2);
        assert_eq!(view.totals.rejected, 1);
        assert_eq!(
            view.totals.others,
            vec![OtherOutcome {
                label: "irrecevable".into(),
                count: 1
            }]
        );
    }

    #[tokio::test]
    async fn counts_distinct_texts_without_the_unattached_votes() {
        let code = "adopt\u{00e9}";
        let view = week_of(
            vec![
                vote("S1", 10, code, code, Some("DOS_A")),
                vote("S2", 10, code, code, Some("DOS_A")),
                vote("S3", 11, code, code, Some("DOS_B")),
                vote("S4", 11, code, code, None),
            ],
            None,
        )
        .await
        .unwrap();

        assert_eq!(view.texts_count, 2);
    }

    #[tokio::test]
    async fn gives_each_day_its_vote_count_and_state() {
        let view = week_of(
            vec![
                adopted("S1", 2),
                adopted("S2", 2),
                adopted("S3", 4),
                adopted("S4", 5),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        let days: Vec<(NaiveDate, DayState)> =
            view.days.iter().map(|day| (day.date, day.state)).collect();
        assert_eq!(
            days,
            vec![
                (d(2), DayState::Voted(2)),
                (d(3), DayState::NoPublicVote),
                (d(4), DayState::Voted(1)),
                (d(5), DayState::Voted(1)),
                (d(6), DayState::NotYetPublished),
            ]
        );
    }

    #[tokio::test]
    async fn gives_the_previous_week_with_its_vote_count_and_no_next_week_on_the_current_week() {
        let view = week_of(
            vec![adopted("S1", 10), adopted("S2", 3), adopted("S3", 3)],
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            view.previous,
            Some(WeekNeighbour {
                monday: d(2),
                votes_count: 2
            })
        );
        assert_eq!(view.next, None);
    }

    #[tokio::test]
    async fn keeps_every_vote_of_a_week_of_362_votes() {
        let dossiers = [Some("DOS_A"), Some("DOS_B"), Some("DOS_C"), None];
        let rows: Vec<ScrutinSummary> = (0..362)
            .map(|i| {
                numbered(
                    &format!("S{i}"),
                    2 + (i % 5) as u32,
                    &(i + 1).to_string(),
                    dossiers[i % 4],
                )
            })
            .collect();

        let view = week_of(rows, Some(d(4))).await.unwrap();

        let kept: usize = view.texts.iter().map(|card| card.items.len()).sum();
        assert_eq!(kept, 362);
        assert_eq!(view.totals.votes, 362);
        assert_eq!(view.texts.len(), 4);
    }

    #[tokio::test]
    async fn groups_the_votes_by_attached_dossier_in_order_of_their_first_vote() {
        let view = week_of(
            vec![
                numbered("S1", 3, "5", Some("DOS_A")),
                numbered("S2", 2, "9", Some("DOS_A")),
                numbered("S3", 2, "3", Some("DOS_B")),
                numbered("S4", 4, "1", Some("DOS_B")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        let cards: Vec<(Option<&str>, Option<&str>)> = view
            .texts
            .iter()
            .map(|card| (card.dossier_uid.as_deref(), card.dossier_label.as_deref()))
            .collect();
        assert_eq!(
            cards,
            vec![
                (Some("DOS_B"), Some("Texte DOS_B")),
                (Some("DOS_A"), Some("Texte DOS_A"))
            ]
        );
    }

    #[tokio::test]
    async fn lists_the_votes_of_a_card_by_date_then_number() {
        let view = week_of(
            vec![
                numbered("S3", 3, "1", Some("DOS_A")),
                numbered("S100", 2, "100", Some("DOS_A")),
                numbered("S99", 2, "99", Some("DOS_A")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        assert_eq!(view.texts.len(), 1);
        assert_eq!(uids(&view.texts[0].items), vec!["S99", "S100", "S3"]);
    }

    #[tokio::test]
    async fn puts_the_unattached_votes_in_a_last_card() {
        let view = week_of(
            vec![
                numbered("S1", 2, "1", None),
                numbered("S2", 3, "2", Some("DOS_A")),
                numbered("S3", 4, "3", Some("DOS_B")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        let cards: Vec<Option<&str>> = view
            .texts
            .iter()
            .map(|card| card.dossier_uid.as_deref())
            .collect();
        assert_eq!(cards, vec![Some("DOS_A"), Some("DOS_B"), None]);
        assert_eq!(uids(&view.texts[2].items), vec!["S1"]);
    }

    #[tokio::test]
    async fn gives_each_card_its_outcome_counts_per_day() {
        let view = week_of(
            vec![
                numbered("S1", 2, "1", Some("DOS_A")),
                numbered("S2", 2, "2", Some("DOS_A")),
                vote("S3", 2, "rejet\u{00e9}", "rejet\u{00e9}", Some("DOS_A")),
                vote("S4", 3, "irrecevable", "irrecevable", Some("DOS_A")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        let card = &view.texts[0];
        assert_eq!(card.totals.outcomes.votes, 4);
        assert_eq!(card.totals.outcomes.adopted, 2);
        assert_eq!(card.totals.outcomes.rejected, 1);
        assert_eq!(
            card.totals.outcomes.others,
            vec![OtherOutcome {
                label: "irrecevable".into(),
                count: 1
            }]
        );
        let days: Vec<(NaiveDate, usize, usize, usize, usize)> = card
            .days
            .iter()
            .map(|day| {
                let o = &day.tally.outcomes;
                (day.date, o.votes, o.adopted, o.rejected, o.others.len())
            })
            .collect();
        assert_eq!(days, vec![(d(2), 3, 2, 1, 0), (d(3), 1, 0, 0, 1)]);
    }

    #[tokio::test]
    async fn gives_a_card_the_outcome_of_its_latest_final_vote_of_the_week() {
        let view = week_of(
            vec![
                final_row("S1", 2, "5", "adopt\u{00e9}", "DOS_A"),
                final_row("S2", 4, "3", "rejet\u{00e9}", "DOS_A"),
                numbered("S3", 5, "1", Some("DOS_A")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        assert_eq!(
            view.texts[0].totals.latest_final,
            Some(FinalOutcome {
                scrutin_uid: "S2".into(),
                date: d(4),
                number: "3".into(),
                outcome_code: "rejet\u{00e9}".into(),
                outcome_label: "rejet\u{00e9}".into(),
            })
        );
    }

    #[tokio::test]
    async fn gives_a_card_day_the_outcome_of_its_latest_final_vote_of_that_day() {
        let view = week_of(
            vec![
                final_row("S1", 2, "1", "adopt\u{00e9}", "DOS_A"),
                final_row("S2", 2, "2", "rejet\u{00e9}", "DOS_A"),
                numbered("S3", 3, "1", Some("DOS_A")),
            ],
            Some(d(4)),
        )
        .await
        .unwrap();

        let days = &view.texts[0].days;
        let first = days[0].tally.latest_final.as_ref().unwrap();
        assert_eq!(first.scrutin_uid, "S2");
        assert_eq!(first.outcome_code, "rejet\u{00e9}");
        assert_eq!(days[1].tally.latest_final, None);
    }

    #[tokio::test]
    async fn lists_the_final_votes_of_the_week_by_date_then_number_with_every_group() {
        let tallies = |first: &str, second: &str| {
            vec![tally_of("PO2", "B", second), tally_of("PO1", "A", first)]
        };
        let view = week_with(
            vec![],
            vec![
                record("S4", 4, "3", tallies("for", "against")),
                record("S100", 2, "100", tallies("against", "for")),
                record("S99", 2, "99", tallies("for", "for")),
                record("OUT", 10, "1", tallies("for", "for")),
            ],
            vec![group("PO2", "B", 3), group("PO1", "A", 5)],
            HashMap::new(),
            Some(d(4)),
        )
        .await
        .unwrap();

        let order: Vec<&str> = view
            .final_votes
            .iter()
            .map(|entry| entry.vote.scrutin_uid().as_str())
            .collect();
        assert_eq!(order, vec!["S99", "S100", "S4"]);
        let abbrevs: Vec<&str> = view.groups.iter().map(|g| g.abbrev.as_str()).collect();
        assert_eq!(abbrevs, vec!["A", "B"]);
        let positions: Vec<Vec<(String, Option<VotePosition>)>> = view
            .final_votes
            .iter()
            .map(|entry| {
                entry
                    .vote
                    .stances()
                    .iter()
                    .map(|s| (s.group().abbrev.clone(), s.published_majority()))
                    .collect()
            })
            .collect();
        let pos = |a: &str, b: &str| {
            vec![
                ("A".to_string(), VotePosition::parse(a)),
                ("B".to_string(), VotePosition::parse(b)),
            ]
        };
        assert_eq!(
            positions,
            vec![
                pos("for", "for"),
                pos("against", "for"),
                pos("for", "against")
            ]
        );
    }

    #[tokio::test]
    async fn gives_each_vote_its_families_and_a_card_the_families_of_its_votes_without_duplicates() {
        let mut stored = HashMap::new();
        stored.insert("S1".to_string(), vec![family(FamilyCode::Logement)]);
        stored.insert(
            "S2".to_string(),
            vec![family(FamilyCode::Logement), family(FamilyCode::Sante)],
        );
        let mut with_family = record("S1", 2, "1", vec![]);
        with_family.families = vec![family(FamilyCode::Logement)];

        let view = week_with(
            vec![
                numbered("S1", 2, "1", Some("DOS_A")),
                numbered("S2", 3, "2", Some("DOS_A")),
                numbered("S3", 3, "3", Some("DOS_A")),
            ],
            vec![with_family],
            vec![],
            stored,
            Some(d(4)),
        )
        .await
        .unwrap();

        let codes = |families: &[AssignedFamily]| {
            let mut codes: Vec<FamilyCode> = families.iter().map(|f| f.family).collect();
            codes.sort();
            codes
        };
        assert_eq!(codes(&view.families["S1"]), vec![FamilyCode::Logement]);
        assert_eq!(
            codes(&view.families["S2"]),
            vec![FamilyCode::Logement, FamilyCode::Sante]
        );
        assert!(!view.families.contains_key("S3"));
        assert_eq!(
            codes(&view.texts[0].families),
            vec![FamilyCode::Logement, FamilyCode::Sante]
        );
        assert_eq!(
            codes(&view.final_votes[0].families),
            vec![FamilyCode::Logement]
        );
    }
}
