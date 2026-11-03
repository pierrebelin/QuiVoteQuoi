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
}
