//! DTO de fraicheur des sources (RM-F5, RM-F6, RM-F7).
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use crate::application::use_cases::describe_freshness::{FreshnessOverview, SourceFreshness};
use crate::domain::ingestion::{Freshness, IngestionSource};

#[derive(Debug, Serialize)]
pub struct SourceFreshnessDto {
    pub source: &'static str,
    pub label: &'static str,
    pub status: &'static str,
    pub status_label: &'static str,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_attempt_succeeded: Option<bool>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Serialize)]
pub struct FreshnessResponse {
    pub sources: Vec<SourceFreshnessDto>,
    pub stale_sources: Vec<&'static str>,
    pub last_scrutin_date: Option<NaiveDate>,
}

fn label_of(source: IngestionSource) -> &'static str {
    match source {
        IngestionSource::ActorRegistry => "Députés et groupes",
        IngestionSource::Dossiers => "Dossiers législatifs",
        IngestionSource::Scrutins => "Scrutins",
        IngestionSource::Amendments => "Amendements",
    }
}

fn status_of(freshness: Freshness) -> (&'static str, &'static str) {
    match freshness {
        Freshness::UpToDate => ("up_to_date", "À jour"),
        Freshness::Late => ("late", "En retard"),
        Freshness::NeverSynchronised => ("never_synchronised", "Jamais synchronisée"),
    }
}

/// RM-F5: le motif d'échec n'entre pas dans la réponse publique.
impl From<SourceFreshness> for SourceFreshnessDto {
    fn from(value: SourceFreshness) -> Self {
        let (status, status_label) = status_of(value.freshness);
        let history = value.history;
        Self {
            source: history.source().as_str(),
            label: label_of(history.source()),
            status,
            status_label,
            last_success_at: history.last_success_at(),
            last_attempt_at: history.last_attempt().map(|a| a.at()),
            last_attempt_succeeded: history.last_attempt().map(|a| a.succeeded()),
            consecutive_failures: history.failures_since_last_success(),
        }
    }
}

/// RM-F6: `stale_sources` liste les codes des sources en retard ou jamais synchronisées.
impl From<FreshnessOverview> for FreshnessResponse {
    fn from(overview: FreshnessOverview) -> Self {
        let stale_sources = overview
            .sources
            .iter()
            .filter(|s| s.freshness != Freshness::UpToDate)
            .map(|s| s.history.source().as_str())
            .collect();
        Self {
            sources: overview
                .sources
                .into_iter()
                .map(SourceFreshnessDto::from)
                .collect(),
            stale_sources,
            last_scrutin_date: overview.last_scrutin_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::application::use_cases::describe_freshness::SourceFreshness;
    use crate::domain::ingestion::{Freshness, IngestionSource, LastAttempt, SourceHistory};

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
    }

    fn entry(history: SourceHistory, freshness: Freshness) -> SourceFreshness {
        SourceFreshness { history, freshness }
    }

    fn succeeded(source: IngestionSource) -> SourceHistory {
        SourceHistory::new(source, LastAttempt::new(at(2, 19), true), Some(at(2, 19)), 0).unwrap()
    }

    fn overview(sources: Vec<SourceFreshness>) -> FreshnessOverview {
        FreshnessOverview {
            sources,
            last_scrutin_date: NaiveDate::from_ymd_opt(2026, 10, 28),
        }
    }

    #[test]
    fn should_list_late_and_never_synchronised_sources_in_stale_sources_when_some_are_not_up_to_date()
    {
        let response = FreshnessResponse::from(overview(vec![
            entry(succeeded(IngestionSource::ActorRegistry), Freshness::UpToDate),
            entry(SourceHistory::never(IngestionSource::Dossiers), Freshness::NeverSynchronised),
            entry(succeeded(IngestionSource::Scrutins), Freshness::Late),
            entry(succeeded(IngestionSource::Amendments), Freshness::UpToDate),
        ]));

        assert_eq!(response.stale_sources, vec!["dossiers", "scrutins"]);
    }

    #[test]
    fn should_list_no_stale_source_when_every_source_is_up_to_date() {
        let response = FreshnessResponse::from(overview(
            IngestionSource::ALL
                .into_iter()
                .map(|s| entry(succeeded(s), Freshness::UpToDate))
                .collect(),
        ));

        assert!(response.stale_sources.is_empty());
        assert_eq!(response.sources.len(), 4);
    }

    #[test]
    fn should_name_source_and_state_in_french_when_overview_is_rendered() {
        let response = FreshnessResponse::from(overview(vec![
            entry(succeeded(IngestionSource::ActorRegistry), Freshness::UpToDate),
            entry(succeeded(IngestionSource::Dossiers), Freshness::Late),
            entry(SourceHistory::never(IngestionSource::Scrutins), Freshness::NeverSynchronised),
            entry(succeeded(IngestionSource::Amendments), Freshness::UpToDate),
        ]));

        let rendered: Vec<(&str, &str, &str, &str)> = response
            .sources
            .iter()
            .map(|s| (s.source, s.label, s.status, s.status_label))
            .collect();
        assert_eq!(
            rendered,
            vec![
                ("actor_registry", "Députés et groupes", "up_to_date", "À jour"),
                ("dossiers", "Dossiers législatifs", "late", "En retard"),
                ("scrutins", "Scrutins", "never_synchronised", "Jamais synchronisée"),
                ("amendments", "Amendements", "up_to_date", "À jour"),
            ]
        );
    }

    #[test]
    fn should_give_last_attempt_date_outcome_and_failure_count_when_last_attempt_failed() {
        let history = SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(at(2, 18), false),
            Some(at(1, 12)),
            3,
        )
        .unwrap();

        let response = FreshnessResponse::from(overview(vec![entry(history, Freshness::Late)]));

        let dto = &response.sources[0];
        assert_eq!(dto.last_attempt_at, Some(at(2, 18)));
        assert_eq!(dto.last_attempt_succeeded, Some(false));
        assert_eq!(dto.consecutive_failures, 3);
        assert_eq!(dto.last_success_at, Some(at(1, 12)));
        assert_eq!(response.last_scrutin_date, NaiveDate::from_ymd_opt(2026, 10, 28));
    }

    #[test]
    fn should_give_no_attempt_date_nor_outcome_when_source_was_never_attempted() {
        let response = FreshnessResponse::from(overview(vec![entry(
            SourceHistory::never(IngestionSource::Amendments),
            Freshness::NeverSynchronised,
        )]));

        let dto = &response.sources[0];
        assert_eq!(dto.last_attempt_at, None);
        assert_eq!(dto.last_attempt_succeeded, None);
        assert_eq!(dto.last_success_at, None);
        assert_eq!(dto.consecutive_failures, 0);
    }
}
