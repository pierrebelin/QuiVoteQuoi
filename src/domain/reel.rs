//! Reel de votes: type, periode et completude des donnees (RM-01, RM-09).

use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Paris;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReelKind {
    Yesterday,
    Week,
    Monday,
}

impl ReelKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReelKind::Yesterday => "veille",
            ReelKind::Week => "semaine",
            ReelKind::Monday => "lundi",
        }
    }
}

fn days_before(day: NaiveDate, days: u64) -> NaiveDate {
    day.checked_sub_days(Days::new(days)).unwrap_or(day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReelSlot {
    kind: ReelKind,
    from: NaiveDate,
    to: NaiveDate,
}

impl ReelSlot {
    /// Type et periode du Reel prepare un jour donne (RM-01).
    pub fn for_day(day: NaiveDate) -> ReelSlot {
        let (kind, from, to) = match day.weekday() {
            Weekday::Mon => (ReelKind::Monday, days_before(day, 7), days_before(day, 1)),
            Weekday::Tue => (
                ReelKind::Yesterday,
                days_before(day, 2),
                days_before(day, 1),
            ),
            Weekday::Sun => (ReelKind::Week, days_before(day, 6), days_before(day, 1)),
            _ => (
                ReelKind::Yesterday,
                days_before(day, 1),
                days_before(day, 1),
            ),
        };
        ReelSlot { kind, from, to }
    }

    pub fn kind(&self) -> ReelKind {
        self.kind
    }

    pub fn from(&self) -> NaiveDate {
        self.from
    }

    pub fn to(&self) -> NaiveDate {
        self.to
    }

    pub fn period_label(&self) -> String {
        period_label(self.from, self.to)
    }
}

pub fn weekday_name(day: NaiveDate) -> &'static str {
    match day.weekday() {
        Weekday::Mon => "lundi",
        Weekday::Tue => "mardi",
        Weekday::Wed => "mercredi",
        Weekday::Thu => "jeudi",
        Weekday::Fri => "vendredi",
        Weekday::Sat => "samedi",
        Weekday::Sun => "dimanche",
    }
}

fn month_name(day: NaiveDate) -> &'static str {
    const MONTHS: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    MONTHS[day.month0() as usize]
}

fn day_number(day: NaiveDate) -> String {
    if day.day() == 1 {
        "1er".to_owned()
    } else {
        day.day().to_string()
    }
}

fn day_with_weekday(day: NaiveDate) -> String {
    format!("{} {}", weekday_name(day), day_number(day))
}

/// Etiquette francaise d'une periode, du premier au dernier jour.
pub fn period_label(from: NaiveDate, to: NaiveDate) -> String {
    let month = |day: NaiveDate| format!("{} {}", day_with_weekday(day), month_name(day));
    if from == to {
        return month(from);
    }
    let same_month = from.month() == to.month() && from.year() == to.year();
    let consecutive = from.checked_add_days(Days::new(1)) == Some(to);
    let first = if same_month {
        day_with_weekday(from)
    } else {
        month(from)
    };
    if consecutive {
        format!("{first} et {}", month(to))
    } else {
        format!("du {first} au {}", month(to))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberingSpan {
    pub count: i64,
    pub highest: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteCompleteness {
    Complete,
    NotSynchronisedSince { last_success: Option<DateTime<Utc>> },
    MissingNumbers { missing: i64 },
}

impl VoteCompleteness {
    pub fn of(
        last_success: Option<DateTime<Utc>>,
        period_end: NaiveDate,
        span: NumberingSpan,
    ) -> Self {
        let synchronised = last_success.is_some_and(|at| at >= end_of_day_in_paris(period_end));
        if !synchronised {
            return VoteCompleteness::NotSynchronisedSince { last_success };
        }
        let missing = span.highest - span.count;
        if missing > 0 {
            return VoteCompleteness::MissingNumbers { missing };
        }
        VoteCompleteness::Complete
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, VoteCompleteness::Complete)
    }
}

/// Minuit a Paris suivant le jour donne (RM-09).
fn end_of_day_in_paris(day: NaiveDate) -> DateTime<Utc> {
    let next = day.checked_add_days(Days::new(1)).unwrap_or(day);
    let midnight = next.and_time(chrono::NaiveTime::MIN);
    Paris
        .from_local_datetime(&midnight)
        .earliest()
        .map_or_else(|| midnight.and_utc(), |at| at.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn at(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, day, h, min, 0).unwrap()
    }

    fn slot(day: NaiveDate) -> (ReelKind, NaiveDate, NaiveDate) {
        let slot = ReelSlot::for_day(day);
        (slot.kind(), slot.from(), slot.to())
    }

    #[test]
    fn covers_the_day_before_from_wednesday_to_saturday() {
        assert_eq!(
            slot(d(2026, 11, 21)),
            (ReelKind::Yesterday, d(2026, 11, 20), d(2026, 11, 20))
        );
        assert_eq!(
            slot(d(2026, 11, 25)),
            (ReelKind::Yesterday, d(2026, 11, 24), d(2026, 11, 24))
        );
        assert_eq!(ReelKind::Yesterday.as_str(), "veille");
    }

    #[test]
    fn covers_sunday_and_monday_on_tuesday() {
        assert_eq!(
            slot(d(2026, 11, 24)),
            (ReelKind::Yesterday, d(2026, 11, 22), d(2026, 11, 23))
        );
    }

    #[test]
    fn covers_monday_to_saturday_on_sunday() {
        assert_eq!(
            slot(d(2026, 11, 22)),
            (ReelKind::Week, d(2026, 11, 16), d(2026, 11, 21))
        );
        assert_eq!(ReelKind::Week.as_str(), "semaine");
    }

    #[test]
    fn covers_the_previous_week_on_monday() {
        assert_eq!(
            slot(d(2026, 11, 23)),
            (ReelKind::Monday, d(2026, 11, 16), d(2026, 11, 22))
        );
        assert_eq!(ReelKind::Monday.as_str(), "lundi");
    }

    #[test]
    fn is_complete_when_synchronised_after_the_period_without_numbering_gap() {
        let span = NumberingSpan {
            count: 414,
            highest: 414,
        };
        let completeness =
            VoteCompleteness::of(Some(at(2026, 11, 21, 6, 0)), d(2026, 11, 20), span);
        assert_eq!(completeness, VoteCompleteness::Complete);
        assert!(completeness.is_complete());
    }

    #[test]
    fn is_incomplete_when_the_last_synchronisation_precedes_midnight_in_paris_after_the_period() {
        let span = NumberingSpan {
            count: 414,
            highest: 414,
        };
        let last = Some(at(2026, 11, 20, 22, 30));
        let completeness = VoteCompleteness::of(last, d(2026, 11, 20), span);
        assert_eq!(
            completeness,
            VoteCompleteness::NotSynchronisedSince { last_success: last }
        );
        assert!(!completeness.is_complete());
        assert_eq!(
            VoteCompleteness::of(None, d(2026, 11, 20), span),
            VoteCompleteness::NotSynchronisedSince { last_success: None }
        );
    }

    #[test]
    fn is_incomplete_when_a_scrutin_number_is_missing() {
        let span = NumberingSpan {
            count: 412,
            highest: 414,
        };
        let completeness =
            VoteCompleteness::of(Some(at(2026, 11, 21, 6, 0)), d(2026, 11, 20), span);
        assert_eq!(completeness, VoteCompleteness::MissingNumbers { missing: 2 });
        assert!(!completeness.is_complete());
    }

    #[test]
    fn labels_a_period_from_its_first_and_last_day() {
        assert_eq!(
            period_label(d(2026, 11, 20), d(2026, 11, 20)),
            "vendredi 20 novembre"
        );
        assert_eq!(
            period_label(d(2026, 11, 22), d(2026, 11, 23)),
            "dimanche 22 et lundi 23 novembre"
        );
    }
}
