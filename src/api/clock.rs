//! Date du jour en heure de Paris (RM-02, RM-06).
use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Europe::Paris;

/// RM-02, RM-06: le jour civil se lit en heure de Paris, pas en UTC.
pub fn paris_date(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Paris).date_naive()
}

/// Date du jour en heure de Paris, lue sur l'horloge système.
pub fn paris_today() -> NaiveDate {
    paris_date(Utc::now())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn gives_the_next_day_in_paris_at_half_past_ten_pm_utc_in_summer() {
        let now = Utc.with_ymd_and_hms(2026, 7, 3, 22, 30, 0).unwrap();

        assert_eq!(paris_date(now), NaiveDate::from_ymd_opt(2026, 7, 4).unwrap());
    }

    #[test]
    fn keeps_the_same_day_in_paris_at_half_past_ten_pm_utc_in_winter() {
        let now = Utc.with_ymd_and_hms(2026, 1, 9, 22, 30, 0).unwrap();

        assert_eq!(paris_date(now), NaiveDate::from_ymd_opt(2026, 1, 9).unwrap());
    }
}
