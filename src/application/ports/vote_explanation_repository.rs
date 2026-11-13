use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::actor::{ActorUid, GroupUid};
use crate::domain::scrutin::{ScrutinUid, VotePosition};
use crate::domain::sitting_record::{
    AttachmentCounters, ExplanationCounters, SittingKeysFingerprint, SittingRecordUid, SittingRef,
};
use crate::domain::vote_explanation::{FinalVoteAttachmentCounts, VoteExplanation};

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

/// Séance stockée qui porte le compte rendu d'un scrutin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReport {
    pub record_uid: SittingRecordUid,
    pub version: String,
}

/// Ce que la base dit des explications d'un scrutin (RM-09).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScrutinExplanationsRead {
    /// `None`: aucun compte rendu stocké pour la séance du scrutin.
    pub report: Option<StoredReport>,
    /// `None`: le compte rendu ne rattache pas le scrutin; `Some(bloc présent)`.
    pub attached: Option<bool>,
    /// Dans l'ordre du compte rendu.
    pub explanations: Vec<VoteExplanation>,
}

/// Mention d'un orateur rattaché à un scrutin, sous le groupe que la base lui donne (RM-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationMention {
    pub scrutin_uid: ScrutinUid,
    pub group_uid: GroupUid,
    pub rank: u16,
    pub speaker_uid: ActorUid,
    pub speaker_name: Option<String>,
}

/// Explication d'un groupe telle que la liste des votes sur l'ensemble la lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupExplanationRead {
    pub rank: u16,
    /// `None`: groupe de l'orateur non établi (RM-06).
    pub group_uid: Option<GroupUid>,
    pub speaker_name: Option<String>,
    /// Vote nominatif de l'orateur sur ce scrutin; `None`: non publié, jamais déduit.
    pub speaker_position: Option<VotePosition>,
    pub text: String,
}

/// Ce que la base dit des explications d'un vote sur l'ensemble, pour la liste (RM-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalVoteExplanationsRead {
    pub scrutin_uid: ScrutinUid,
    /// `false`: le scrutin ne porte aucune référence de séance.
    pub sitting_known: bool,
    /// `false`: aucun compte rendu stocké pour la séance du scrutin.
    pub report_stored: bool,
    /// `None`: le compte rendu ne rattache pas le scrutin; `Some(bloc présent)`.
    pub attached: Option<bool>,
    /// Dans l'ordre du compte rendu.
    pub explanations: Vec<GroupExplanationRead>,
}

#[async_trait]
pub trait VoteExplanationRepository: Send + Sync {
    /// Mentions des orateurs au groupe établi, pour ces scrutins: une lecture (RM-09).
    async fn mentions_for(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<ExplanationMention>, RepositoryError>;

    /// Compteurs du rattachement de chaque séance stockée et ceux des votes sur l'ensemble (RM-10, RM-13).
    async fn coverage(
        &self,
    ) -> Result<
        (
            Vec<(AttachmentCounters, ExplanationCounters)>,
            FinalVoteAttachmentCounts,
        ),
        RepositoryError,
    >;

    async fn stored_versions(
        &self,
    ) -> Result<HashMap<SittingRecordUid, StoredVersion>, RepositoryError>;

    /// Remplace en une transaction tout ce que la séance porte (RM-08).
    async fn replace_sitting(&self, sitting: &StoredSitting) -> Result<(), RepositoryError>;

    /// Explications d'un scrutin et état de son compte rendu: une lecture (RM-09, RM-11).
    async fn explanations_for(
        &self,
        scrutin: &ScrutinUid,
        sitting_ref: &SittingRef,
    ) -> Result<ScrutinExplanationsRead, RepositoryError>;

    /// Explications de ces votes sur l'ensemble, avec le vote de chaque orateur:
    /// une lecture pour toute une page de la liste (RM-09).
    async fn final_vote_explanations(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<FinalVoteExplanationsRead>, RepositoryError>;
}
