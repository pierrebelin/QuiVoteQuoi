use chrono::{DateTime, NaiveDate, Utc};

use crate::application::ports::final_vote_repository::{
    FinalVoteFilter, FinalVoteRecord, FinalVoteRepository,
};
use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::application::ports::scrutin_repository::{
    RepositoryError, ScrutinRepository, ScrutinSummary,
};
use crate::application::use_cases::browse_final_votes::MAX_PAGE_SIZE;
use crate::application::use_cases::get_week_of_votes::{totals_of, OutcomeCount};
use crate::domain::ingestion::IngestionSource;
use crate::domain::reel::{ReelKind, ReelSlot, VoteCompleteness};

const CURRENT_LEGISLATURE: u16 = 17;

/// Votes d'un jour de la periode du Reel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReelDay {
    pub date: NaiveDate,
    pub votes: usize,
}

/// Contenu d'un Reel de votes (RM-03, RM-04, RM-10).
#[derive(Debug, Clone)]
pub struct VotesReel {
    pub totals: OutcomeCount,
    pub final_votes: Vec<FinalVoteRecord>,
    pub most_followed: Option<ScrutinSummary>,
    pub days: Vec<ReelDay>,
    pub has_reconstructed_tallies: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NoVote,
    NoSubject,
}

#[derive(Debug, Clone)]
pub enum ReelPlan {
    Votes(VotesReel),
    Skipped(SkipReason),
    Incomplete(VoteCompleteness),
}

#[derive(Debug, Clone)]
pub struct PreparedReel {
    pub slot: ReelSlot,
    pub last_success_at: Option<DateTime<Utc>>,
    pub plan: ReelPlan,
}

#[derive(Debug, thiserror::Error)]
pub enum PrepareReelError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// CU-01, CU-02 — Preparer le Reel du jour.
pub struct PrepareReel<'a> {
    scrutins: &'a dyn ScrutinRepository,
    final_votes: &'a dyn FinalVoteRepository,
    journal: &'a dyn IngestionJournal,
}

impl<'a> PrepareReel<'a> {
    pub fn new(
        scrutins: &'a dyn ScrutinRepository,
        final_votes: &'a dyn FinalVoteRepository,
        journal: &'a dyn IngestionJournal,
    ) -> Self {
        Self {
            scrutins,
            final_votes,
            journal,
        }
    }

    pub async fn execute(&self, today: NaiveDate) -> Result<PreparedReel, PrepareReelError> {
        let slot = ReelSlot::for_day(today);
        let last_success_at = self
            .journal
            .histories()
            .await?
            .iter()
            .find(|history| history.source() == IngestionSource::Scrutins)
            .and_then(|history| history.last_success_at());
        let prepared = |plan| PreparedReel {
            slot,
            last_success_at,
            plan,
        };
        if slot.kind() == ReelKind::Monday {
            return Ok(prepared(ReelPlan::Skipped(SkipReason::NoSubject)));
        }

        let rows = self.scrutins.between(slot.from(), slot.to()).await?;
        if rows.is_empty() && slot.kind() != ReelKind::Week {
            return Ok(prepared(ReelPlan::Skipped(SkipReason::NoVote)));
        }

        let span = self.scrutins.numbering_span(CURRENT_LEGISLATURE).await?;
        let completeness = VoteCompleteness::of(last_success_at, slot.to(), span);
        if !completeness.is_complete() {
            return Ok(prepared(ReelPlan::Incomplete(completeness)));
        }

        let mut final_votes = self
            .final_votes
            .list_final_votes(&FinalVoteFilter {
                from: Some(slot.from()),
                to: Some(slot.to()),
                limit: MAX_PAGE_SIZE,
                ..FinalVoteFilter::default()
            })
            .await?
            .items;
        final_votes.sort_by_key(|vote| vote_key(vote.date, &vote.number));

        let refs: Vec<&ScrutinSummary> = rows.iter().collect();
        Ok(prepared(ReelPlan::Votes(VotesReel {
            totals: totals_of(&refs),
            final_votes,
            days: days_of(&slot, &rows),
            has_reconstructed_tallies: rows.iter().any(|row| row.has_reconstructed_tallies),
            most_followed: most_followed(rows),
        })))
    }
}

/// Jours de la periode avec leur nombre de votes; hors semaine, seuls les jours avec vote.
fn days_of(slot: &ReelSlot, rows: &[ScrutinSummary]) -> Vec<ReelDay> {
    slot.from()
        .iter_days()
        .take_while(|date| *date <= slot.to())
        .map(|date| ReelDay {
            date,
            votes: rows.iter().filter(|row| row.date == date).count(),
        })
        .filter(|day| day.votes > 0 || slot.kind() == ReelKind::Week)
        .collect()
}

/// Scrutin au plus de votants (pour, contre, abstentions), le plus petit numero a egalite (RM-03).
fn most_followed(rows: Vec<ScrutinSummary>) -> Option<ScrutinSummary> {
    rows.into_iter().max_by_key(|row| {
        let voters = row.tally.votes_for as u32
            + row.tally.votes_against as u32
            + row.tally.abstentions as u32;
        (
            voters,
            std::cmp::Reverse(row.number.parse::<u64>().unwrap_or(u64::MAX)),
        )
    })
}

/// Cle de rang d'un vote: date, puis numero lu comme un nombre (99 avant 100).
fn vote_key(date: NaiveDate, number: &str) -> (NaiveDate, u64) {
    (date, number.parse().unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::final_vote_fakes::{final_vote_record, FakeFinalVotes};
    use crate::application::use_cases::get_week_of_votes::OtherOutcome;
    use crate::application::use_cases::journal_fakes::InMemoryIngestionJournal;
    use crate::application::use_cases::read_fakes::{scrutin_row, FakeScrutins};
    use crate::domain::dataset::DatasetShape;
    use crate::domain::ingestion::{IngestionSource, LastAttempt, SourceHistory};
    use crate::domain::reel::ReelKind;
    use crate::domain::scrutin::{Outcome, VoteTally};
    use chrono::TimeZone;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
    }

    fn row(number: u32, day: u32, code: &str, label: &str, voters: u16) -> ScrutinSummary {
        let mut row = scrutin_row(&format!("S{number}"), "subject");
        row.number = number.to_string();
        row.date = d(day);
        row.outcome_code = code.into();
        row.outcome_label = label.into();
        row.tally = VoteTally {
            votes_for: voters,
            ..VoteTally::default()
        };
        row
    }

    fn adopted(number: u32, day: u32) -> ScrutinSummary {
        row(number, day, Outcome::ADOPTED, "adopt\u{00e9}", 10)
    }

    fn history(success: DateTime<Utc>) -> InMemoryIngestionJournal {
        InMemoryIngestionJournal::with_histories(vec![SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(success, true),
            Some(success),
            0,
        )
        .unwrap()])
    }

    async fn prepare(
        today: NaiveDate,
        rows: Vec<ScrutinSummary>,
        finals: Vec<FinalVoteRecord>,
        success: DateTime<Utc>,
    ) -> PreparedReel {
        let scrutins = FakeScrutins {
            rows,
            total: 0,
            shape: DatasetShape::default(),
        };
        let final_votes = FakeFinalVotes { records: finals };
        let journal = history(success);
        let prepared = PrepareReel::new(&scrutins, &final_votes, &journal)
            .execute(today)
            .await
            .unwrap();
        assert_eq!(prepared.last_success_at, Some(success));
        prepared
    }

    fn votes(prepared: PreparedReel) -> VotesReel {
        match prepared.plan {
            ReelPlan::Votes(reel) => reel,
            other => panic!("expected a votes reel, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn counts_votes_adopted_rejected_and_other_outcomes_of_the_day_before() {
        let rows = vec![
            adopted(1, 19),
            adopted(2, 20),
            adopted(3, 20),
            adopted(4, 20),
            row(5, 20, Outcome::REJECTED, "rejet\u{00e9}", 10),
            row(6, 20, Outcome::REJECTED, "rejet\u{00e9}", 10),
            row(7, 20, "withdrawn", "retir\u{00e9}", 10),
        ];

        let prepared = prepare(d(21), rows, vec![], at(21, 6)).await;

        assert_eq!(prepared.slot.kind(), ReelKind::Yesterday);
        let reel = votes(prepared);
        assert_eq!(
            reel.totals,
            OutcomeCount {
                votes: 6,
                adopted: 3,
                rejected: 2,
                others: vec![OtherOutcome {
                    label: "retir\u{00e9}".into(),
                    count: 1
                }],
            }
        );
        assert!(!reel.has_reconstructed_tallies);
    }

    #[tokio::test]
    async fn lists_every_final_vote_of_the_period_by_date_then_number() {
        let rows = vec![adopted(1, 16), adopted(2, 17), adopted(3, 17)];
        let finals = vec![
            final_vote_record("100", d(17)),
            final_vote_record("99", d(17)),
            final_vote_record("5", d(16)),
            final_vote_record("7", d(22)),
        ];

        let reel = votes(prepare(d(22), rows, finals, at(22, 6)).await);

        let numbers: Vec<&str> = reel.final_votes.iter().map(|v| v.number.as_str()).collect();
        assert_eq!(numbers, vec!["5", "99", "100"]);
    }

    #[tokio::test]
    async fn picks_the_scrutin_with_most_voters_and_the_lowest_number_on_a_tie() {
        let mut rows: Vec<ScrutinSummary> = (1..=5008).map(|n| adopted(n, 19)).collect();
        let tally = |votes_for, votes_against, abstentions, not_voting| VoteTally {
            votes_for,
            votes_against,
            abstentions,
            not_voting,
            voluntary_not_voting: 0,
        };
        let mut tied_high = adopted(5012, 20);
        tied_high.tally = tally(300, 150, 30, 0);
        let mut tied_low = adopted(5009, 20);
        tied_low.tally = tally(400, 80, 0, 0);
        let mut absent_heavy = adopted(5010, 20);
        absent_heavy.tally = tally(300, 100, 79, 200);
        let mut small = adopted(5011, 20);
        small.tally = tally(100, 0, 0, 0);
        rows.extend([tied_high, tied_low, absent_heavy, small]);

        let reel = votes(prepare(d(21), rows, vec![], at(21, 6)).await);

        assert_eq!(reel.most_followed.unwrap().uid, "S5009");
    }

    #[tokio::test]
    async fn covers_monday_to_saturday_with_each_day_count_on_sunday() {
        let rows = vec![adopted(1, 16), adopted(2, 16), adopted(3, 18)];

        let prepared = prepare(d(22), rows, vec![], at(22, 6)).await;

        assert_eq!(prepared.slot.kind(), ReelKind::Week);
        let reel = votes(prepared);
        let days: Vec<(NaiveDate, usize)> = reel.days.iter().map(|x| (x.date, x.votes)).collect();
        assert_eq!(
            days,
            vec![
                (d(16), 2),
                (d(17), 0),
                (d(18), 1),
                (d(19), 0),
                (d(20), 0),
                (d(21), 0)
            ]
        );
    }

    #[tokio::test]
    async fn prepares_a_week_reel_with_zero_counts_when_the_week_had_no_vote() {
        let reel = votes(prepare(d(22), vec![], vec![], at(22, 6)).await);

        assert_eq!(
            reel.totals,
            OutcomeCount {
                votes: 0,
                adopted: 0,
                rejected: 0,
                others: vec![]
            }
        );
        assert!(reel.most_followed.is_none());
        assert!(reel.final_votes.is_empty());
        assert!(reel.days.iter().all(|day| day.votes == 0));
    }

    #[tokio::test]
    async fn flags_a_reconstructed_tally_in_the_period() {
        let mut flagged = adopted(2, 20);
        flagged.has_reconstructed_tallies = true;
        let rows = vec![adopted(1, 20), flagged];

        let reel = votes(prepare(d(21), rows, vec![], at(21, 6)).await);

        assert!(reel.has_reconstructed_tallies);
    }

    #[tokio::test]
    async fn skips_a_daily_reel_when_the_period_has_no_vote() {
        let prepared = prepare(d(25), vec![adopted(1, 19)], vec![], at(25, 6)).await;

        assert!(matches!(
            prepared.plan,
            ReelPlan::Skipped(SkipReason::NoVote)
        ));
    }

    #[tokio::test]
    async fn reports_an_incomplete_period_instead_of_a_reel() {
        let stale = at(20, 22);
        let prepared = prepare(d(21), vec![adopted(1, 20)], vec![], stale).await;
        assert!(matches!(
            prepared.plan,
            ReelPlan::Incomplete(VoteCompleteness::NotSynchronisedSince {
                last_success: Some(last)
            }) if last == stale
        ));

        let gap = prepare(d(21), vec![adopted(3, 20)], vec![], at(21, 6)).await;
        assert!(matches!(
            gap.plan,
            ReelPlan::Incomplete(VoteCompleteness::MissingNumbers { missing: 2 })
        ));
    }

    #[tokio::test]
    async fn skips_monday_until_a_subject_is_chosen() {
        let rows = vec![adopted(1, 16), adopted(2, 20)];

        let prepared = prepare(d(23), rows, vec![], at(23, 6)).await;

        assert_eq!(prepared.slot.kind(), ReelKind::Monday);
        assert!(matches!(
            prepared.plan,
            ReelPlan::Skipped(SkipReason::NoSubject)
        ));
    }
}
