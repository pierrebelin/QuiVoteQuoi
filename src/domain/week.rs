//! Semaine de consultation, du lundi au dimanche (RM-02, RM-03, RM-06, RM-12).

use chrono::{Datelike, Days, NaiveDate};

use super::legislature::CURRENT_LEGISLATURE_OPENING;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WeekError {
    #[error("week unavailable: {0}")]
    Unavailable(NaiveDate),
}

/// Etat d'un jour de la semaine (RM-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayState {
    Voted(usize),
    NoPublicVote,
    NotYetPublished,
    Upcoming,
}

impl DayState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DayState::Voted(_) => "voted",
            DayState::NoPublicVote => "no_public_vote",
            DayState::NotYetPublished => "not_yet_published",
            DayState::Upcoming => "upcoming",
        }
    }

    /// Etat d'un jour: avec votes, a venir, sans vote public, ou pas encore publie (RM-06).
    pub fn of(
        day: NaiveDate,
        votes: usize,
        last_published: Option<NaiveDate>,
        today: NaiveDate,
    ) -> DayState {
        if votes > 0 {
            DayState::Voted(votes)
        } else if day > today {
            DayState::Upcoming
        } else if last_published.is_some_and(|last| day <= last) {
            DayState::NoPublicVote
        } else {
            DayState::NotYetPublished
        }
    }
}

/// Semaine du lundi au dimanche, entre la premiere de la legislature et la semaine en cours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Week {
    monday: NaiveDate,
}

impl Week {
    fn containing(date: NaiveDate) -> Week {
        let offset = u64::from(date.weekday().num_days_from_monday());
        Week {
            monday: date.checked_sub_days(Days::new(offset)).unwrap_or(date),
        }
    }

    pub fn first() -> Week {
        Week::containing(CURRENT_LEGISLATURE_OPENING)
    }

    pub fn current(today: NaiveDate) -> Week {
        Week::containing(today)
    }

    /// Semaine d'une date demandee, bornee (RM-03).
    pub fn requested(date: NaiveDate, today: NaiveDate) -> Result<Week, WeekError> {
        let week = Week::containing(date);
        if week.monday < Week::first().monday || week.monday > Week::current(today).monday {
            return Err(WeekError::Unavailable(date));
        }
        Ok(week)
    }

    pub fn monday(&self) -> NaiveDate {
        self.monday
    }

    pub fn sunday(&self) -> NaiveDate {
        self.monday
            .checked_add_days(Days::new(6))
            .unwrap_or(self.monday)
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        self.monday <= date && date <= self.sunday()
    }

    pub fn is_current(&self, today: NaiveDate) -> bool {
        self.contains(today)
    }

    pub fn previous(&self) -> Option<Week> {
        if *self == Week::first() {
            return None;
        }
        self.monday
            .checked_sub_days(Days::new(7))
            .map(|monday| Week { monday })
    }

    pub fn next(&self, today: NaiveDate) -> Option<Week> {
        if self.monday >= Week::current(today).monday {
            return None;
        }
        self.monday
            .checked_add_days(Days::new(7))
            .map(|monday| Week { monday })
    }

    /// Lundi a vendredi, samedi et dimanche seulement avec un vote (RM-02).
    pub fn days(&self, votes_on: impl Fn(NaiveDate) -> usize) -> Vec<NaiveDate> {
        (0..7u64)
            .filter_map(|offset| self.monday.checked_add_days(Days::new(offset)))
            .filter(|day| day.weekday().num_days_from_monday() < 5 || votes_on(*day) > 0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // vendredi 13 novembre 2026
    fn today() -> NaiveDate {
        d(2026, 11, 13)
    }

    fn monday_of(date: NaiveDate, today: NaiveDate) -> Result<NaiveDate, WeekError> {
        Week::requested(date, today).map(|w| w.monday())
    }

    #[test]
    fn brings_a_sunday_back_to_the_monday_of_its_week() {
        assert_eq!(monday_of(d(2026, 11, 8), today()), Ok(d(2026, 11, 2)));
    }

    #[test]
    fn brings_a_requested_wednesday_back_to_its_monday() {
        assert_eq!(monday_of(d(2026, 11, 4), today()), Ok(d(2026, 11, 2)));
    }

    #[test]
    fn accepts_a_future_day_of_the_current_week() {
        assert_eq!(monday_of(d(2026, 11, 15), today()), Ok(d(2026, 11, 9)));
    }

    #[test]
    fn accepts_the_week_of_july_18_2024_as_the_first_week() {
        assert_eq!(monday_of(d(2024, 7, 18), today()), Ok(d(2024, 7, 15)));
    }

    #[test]
    fn refuses_a_date_before_the_first_week_of_the_legislature() {
        assert_eq!(
            monday_of(d(2024, 7, 14), today()),
            Err(WeekError::Unavailable(d(2024, 7, 14)))
        );
    }

    #[test]
    fn refuses_a_date_after_the_current_week() {
        assert_eq!(
            monday_of(d(2026, 11, 16), today()),
            Err(WeekError::Unavailable(d(2026, 11, 16)))
        );
    }

    #[test]
    fn has_no_previous_week_on_the_first_week() {
        let first = Week::first();
        assert_eq!(first.monday(), d(2024, 7, 15));
        assert_eq!(first.previous(), None);
        let second = Week::requested(d(2024, 7, 22), today()).unwrap();
        assert_eq!(second.previous(), Some(first));
    }

    #[test]
    fn has_no_next_week_on_the_current_week() {
        let current = Week::current(today());
        assert_eq!(current.next(today()), None);
        let before = Week::requested(d(2026, 11, 4), today()).unwrap();
        assert_eq!(before.next(today()), Some(current));
    }

    #[test]
    fn lists_the_weekdays_and_only_the_weekend_days_with_a_vote() {
        let week = Week::requested(d(2026, 11, 4), today()).unwrap();
        assert_eq!(
            week.days(|_| 0),
            vec![
                d(2026, 11, 2),
                d(2026, 11, 3),
                d(2026, 11, 4),
                d(2026, 11, 5),
                d(2026, 11, 6)
            ]
        );
        let with_saturday = week.days(|day| usize::from(day == d(2026, 11, 7)));
        assert_eq!(with_saturday.len(), 6);
        assert_eq!(with_saturday[5], d(2026, 11, 7));
    }

    #[test]
    fn says_voted_for_a_day_with_votes() {
        assert_eq!(
            DayState::of(d(2026, 11, 4), 3, Some(d(2026, 11, 4)), today()),
            DayState::Voted(3)
        );
    }

    #[test]
    fn says_no_public_vote_for_an_empty_day_up_to_the_last_published_vote() {
        assert_eq!(
            DayState::of(d(2026, 11, 13), 0, Some(d(2026, 11, 16)), d(2026, 11, 16)),
            DayState::NoPublicVote
        );
    }

    #[test]
    fn says_not_yet_published_for_an_empty_day_after_the_last_published_vote_up_to_today() {
        assert_eq!(
            DayState::of(d(2026, 11, 13), 0, Some(d(2026, 11, 12)), today()),
            DayState::NotYetPublished
        );
    }

    #[test]
    fn says_upcoming_for_an_empty_future_day() {
        assert_eq!(
            DayState::of(d(2026, 11, 13), 0, Some(d(2026, 11, 10)), d(2026, 11, 12)),
            DayState::Upcoming
        );
    }
}
