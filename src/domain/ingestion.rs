//! Journal d'ingestion: une ligne par synchronisation tentee d'une source.

use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionSource {
    ActorRegistry,
    Dossiers,
    Scrutins,
    Amendments,
}

impl IngestionSource {
    pub const ALL: [IngestionSource; 4] = [
        IngestionSource::ActorRegistry,
        IngestionSource::Dossiers,
        IngestionSource::Scrutins,
        IngestionSource::Amendments,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            IngestionSource::ActorRegistry => "actor_registry",
            IngestionSource::Dossiers => "dossiers",
            IngestionSource::Scrutins => "scrutins",
            IngestionSource::Amendments => "amendments",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.as_str() == code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    Succeeded { items_read: u64 },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheck {
    source: IngestionSource,
    attempted_at: DateTime<Utc>,
    outcome: CheckOutcome,
}

pub const JOURNAL_RETENTION: TimeDelta = TimeDelta::days(90);
pub const UNSTATED_FAILURE_REASON: &str = "motif non precise";

impl SourceCheck {
    pub fn succeeded(
        source: IngestionSource,
        attempted_at: DateTime<Utc>,
        items_read: u64,
    ) -> Self {
        Self {
            source,
            attempted_at,
            outcome: CheckOutcome::Succeeded { items_read },
        }
    }

    pub fn failed(
        source: IngestionSource,
        attempted_at: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        let reason = if reason.trim().is_empty() {
            UNSTATED_FAILURE_REASON.to_string()
        } else {
            reason
        };
        Self {
            source,
            attempted_at,
            outcome: CheckOutcome::Failed { reason },
        }
    }

    pub fn source(&self) -> IngestionSource {
        self.source
    }

    pub fn attempted_at(&self) -> DateTime<Utc> {
        self.attempted_at
    }

    pub fn outcome(&self) -> &CheckOutcome {
        &self.outcome
    }
}

pub const STALE_AFTER: TimeDelta = TimeDelta::hours(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastAttempt {
    at: DateTime<Utc>,
    succeeded: bool,
}

impl LastAttempt {
    pub fn new(at: DateTime<Utc>, succeeded: bool) -> Self {
        Self { at, succeeded }
    }

    pub fn at(&self) -> DateTime<Utc> {
        self.at
    }

    pub fn succeeded(&self) -> bool {
        self.succeeded
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IngestionError {
    #[error("last success follows last attempt")]
    SuccessAfterLastAttempt,
    #[error("failure count contradicts last attempt outcome")]
    FailureCountContradictsLastAttempt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    UpToDate,
    Late,
    NeverSynchronised,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceHistory {
    source: IngestionSource,
    last_attempt: Option<LastAttempt>,
    last_success_at: Option<DateTime<Utc>>,
    failures_since_last_success: u32,
}

impl SourceHistory {
    pub fn new(
        source: IngestionSource,
        last_attempt: LastAttempt,
        last_success_at: Option<DateTime<Utc>>,
        failures_since_last_success: u32,
    ) -> Result<Self, IngestionError> {
        if last_success_at.is_some_and(|success| success > last_attempt.at()) {
            return Err(IngestionError::SuccessAfterLastAttempt);
        }
        if last_attempt.succeeded() != (failures_since_last_success == 0) {
            return Err(IngestionError::FailureCountContradictsLastAttempt);
        }
        Ok(Self {
            source,
            last_attempt: Some(last_attempt),
            last_success_at,
            failures_since_last_success,
        })
    }

    pub fn never(source: IngestionSource) -> Self {
        Self {
            source,
            last_attempt: None,
            last_success_at: None,
            failures_since_last_success: 0,
        }
    }

    pub fn source(&self) -> IngestionSource {
        self.source
    }

    pub fn last_success_at(&self) -> Option<DateTime<Utc>> {
        self.last_success_at
    }

    pub fn last_attempt(&self) -> Option<&LastAttempt> {
        self.last_attempt.as_ref()
    }

    pub fn failures_since_last_success(&self) -> u32 {
        self.failures_since_last_success
    }

    /// RM-F3: l'état ne dépend que de la dernière réussite et de `now`.
    pub fn freshness(&self, now: DateTime<Utc>) -> Freshness {
        match self.last_success_at {
            None => Freshness::NeverSynchronised,
            Some(success) if now - success < STALE_AFTER => Freshness::UpToDate,
            Some(_) => Freshness::Late,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn attempted_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 4, 8, 30, 0).unwrap()
    }

    fn failure_reason(check: &SourceCheck) -> &str {
        match check.outcome() {
            CheckOutcome::Failed { reason } => reason,
            CheckOutcome::Succeeded { .. } => panic!("expected a failed outcome"),
        }
    }

    #[test]
    fn failed_check_replaces_a_blank_reason_with_the_unstated_one() {
        for blank in ["", "  ", "\t\n"] {
            let check = SourceCheck::failed(IngestionSource::Scrutins, attempted_at(), blank);

            assert_eq!(failure_reason(&check), UNSTATED_FAILURE_REASON);
        }
    }

    #[test]
    fn failed_check_keeps_a_stated_reason_unchanged() {
        let check = SourceCheck::failed(
            IngestionSource::Dossiers,
            attempted_at(),
            "download failed: timeout",
        );

        assert_eq!(failure_reason(&check), "download failed: timeout");
    }

    fn at(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, hour, minute, 0).unwrap()
    }

    fn now() -> DateTime<Utc> {
        at(20, 0)
    }

    fn history_with_success_at(success: DateTime<Utc>) -> SourceHistory {
        SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(success, true),
            Some(success),
            0,
        )
        .unwrap()
    }

    #[test]
    fn should_be_up_to_date_when_last_success_is_younger_than_six_hours() {
        let history = history_with_success_at(at(19, 0));

        assert_eq!(history.freshness(now()), Freshness::UpToDate);
    }

    #[test]
    fn should_be_late_when_last_success_is_exactly_six_hours_old() {
        let history = history_with_success_at(at(14, 0));

        assert_eq!(history.freshness(now()), Freshness::Late);
    }

    #[test]
    fn should_be_late_when_last_success_is_older_than_six_hours() {
        let history = history_with_success_at(at(13, 59));

        assert_eq!(history.freshness(now()), Freshness::Late);
    }

    #[test]
    fn should_stay_up_to_date_when_last_attempt_failed_after_a_recent_success() {
        let history = SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(at(19, 30), false),
            Some(at(19, 0)),
            1,
        )
        .unwrap();

        assert_eq!(history.freshness(now()), Freshness::UpToDate);
    }

    #[test]
    fn should_be_never_synchronised_when_every_attempt_failed() {
        let history = SourceHistory::new(
            IngestionSource::Dossiers,
            LastAttempt::new(at(19, 30), false),
            None,
            3,
        )
        .unwrap();

        assert_eq!(history.freshness(now()), Freshness::NeverSynchronised);
    }

    #[test]
    fn should_be_never_synchronised_when_journal_holds_no_attempt() {
        let history = SourceHistory::never(IngestionSource::Amendments);

        assert_eq!(history.freshness(now()), Freshness::NeverSynchronised);
        assert!(history.last_attempt().is_none());
        assert!(history.last_success_at().is_none());
    }

    #[test]
    fn should_refuse_history_when_last_success_follows_last_attempt() {
        let result = SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(at(19, 0), true),
            Some(at(19, 30)),
            0,
        );

        assert_eq!(result.unwrap_err(), IngestionError::SuccessAfterLastAttempt);
    }

    #[test]
    fn should_refuse_history_when_failure_count_contradicts_last_attempt_outcome() {
        let succeeded_with_failures = SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(at(19, 0), true),
            Some(at(19, 0)),
            2,
        );
        let failed_without_failures = SourceHistory::new(
            IngestionSource::Scrutins,
            LastAttempt::new(at(19, 30), false),
            Some(at(19, 0)),
            0,
        );

        assert_eq!(
            succeeded_with_failures.unwrap_err(),
            IngestionError::FailureCountContradictsLastAttempt
        );
        assert_eq!(
            failed_without_failures.unwrap_err(),
            IngestionError::FailureCountContradictsLastAttempt
        );
    }

    #[test]
    fn should_read_back_every_source_when_its_code_is_parsed() {
        for source in IngestionSource::ALL {
            assert_eq!(IngestionSource::parse(source.as_str()), Some(source));
        }
        assert_eq!(IngestionSource::parse("unknown"), None);
    }
}
