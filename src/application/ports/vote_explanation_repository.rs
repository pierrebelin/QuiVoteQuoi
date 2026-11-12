use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::scrutin::ScrutinUid;
use crate::domain::sitting_record::{
    AttachmentCounters, ExplanationCounters, SittingKeysFingerprint, SittingRecordUid, SittingRef,
};
use crate::domain::vote_explanation::VoteExplanation;

pub use super::RepositoryError;

/// Version d'une séance telle que stockée (RM-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredVersion {
    pub generated_at: DateTime<Utc>,
    pub keys: SittingKeysFingerprint,
}

/// Une séance entière: ses rattachements et ses explications (RM-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSitting {
    pub record_uid: SittingRecordUid,
    pub sitting_ref: SittingRef,
    pub date: NaiveDate,
    pub version: String,
    pub generated_at: DateTime<Utc>,
    pub keys: SittingKeysFingerprint,
    pub counters: AttachmentCounters,
    pub block_counters: ExplanationCounters,
    /// Scrutin rattaché et présence d'un bloc d'explications.
    pub attached: Vec<(ScrutinUid, bool)>,
    pub explanations: Vec<VoteExplanation>,
}

#[async_trait]
pub trait VoteExplanationRepository: Send + Sync {
    async fn stored_versions(
        &self,
    ) -> Result<HashMap<SittingRecordUid, StoredVersion>, RepositoryError>;

    /// Remplace en une transaction tout ce que la séance porte (RM-08).
    async fn replace_sitting(&self, sitting: &StoredSitting) -> Result<(), RepositoryError>;
}
