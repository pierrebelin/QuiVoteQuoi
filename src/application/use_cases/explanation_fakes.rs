//! Double en memoire du dépôt des explications de vote.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::vote_explanation_repository::{
    ExplanationMention, FinalVoteExplanationsRead, ScrutinExplanationsRead, StoredReport, StoredSitting, StoredVersion,
    VoteExplanationRepository,
};
use crate::application::ports::RepositoryError;
use crate::domain::scrutin::ScrutinUid;
use crate::domain::sitting_record::{
    AttachmentCounters, ExplanationCounters, SittingRecordUid, SittingRef,
};
use crate::domain::vote_explanation::FinalVoteAttachmentCounts;

#[derive(Default)]
pub struct InMemoryVoteExplanations {
    versions: HashMap<SittingRecordUid, StoredVersion>,
    failing_on: Option<SittingRecordUid>,
    written: Mutex<Vec<StoredSitting>>,
    sittings: Vec<StoredSitting>,
    reads: Mutex<usize>,
    final_votes: FinalVoteAttachmentCounts,
    final_vote_reads: Vec<FinalVoteExplanationsRead>,
}

impl InMemoryVoteExplanations {
    /// Dépôt qui porte déjà ces versions de séance.
    pub fn with_versions(versions: Vec<(SittingRecordUid, StoredVersion)>) -> Self {
        Self {
            versions: versions.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Dépôt dont l'écriture de cette séance échoue sans rien écrire.
    pub fn failing_on(uid: SittingRecordUid) -> Self {
        Self {
            failing_on: Some(uid),
            ..Self::default()
        }
    }

    /// Dépôt qui porte déjà ces séances entières, pour la lecture d'un scrutin.
    pub fn with_sittings(sittings: Vec<StoredSitting>) -> Self {
        Self {
            sittings,
            ..Self::default()
        }
    }

    /// Compteurs des votes sur l'ensemble que le dépôt rend.
    pub fn with_final_vote_counts(self, final_votes: FinalVoteAttachmentCounts) -> Self {
        Self {
            final_votes,
            ..self
        }
    }

    /// Dépôt qui rend ces lectures pour la liste des votes sur l'ensemble.
    pub fn with_final_vote_reads(final_vote_reads: Vec<FinalVoteExplanationsRead>) -> Self {
        Self {
            final_vote_reads,
            ..Self::default()
        }
    }

    /// Nombre de lectures faites.
    pub fn reads(&self) -> usize {
        *self.reads.lock().unwrap()
    }

    pub fn written(&self) -> Vec<StoredSitting> {
        self.written.lock().unwrap().clone()
    }
}

#[async_trait]
impl VoteExplanationRepository for InMemoryVoteExplanations {
    async fn coverage(
        &self,
    ) -> Result<
        (
            Vec<(AttachmentCounters, ExplanationCounters)>,
            FinalVoteAttachmentCounts,
        ),
        RepositoryError,
    > {
        Ok((
            self.sittings
                .iter()
                .map(|s| (s.counters, s.block_counters))
                .collect(),
            self.final_votes,
        ))
    }

    async fn mentions_for(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<ExplanationMention>, RepositoryError> {
        *self.reads.lock().unwrap() += 1;
        Ok(self
            .sittings
            .iter()
            .flat_map(|sitting| sitting.explanations.iter())
            .filter(|e| scrutins.contains(e.scrutin_uid()))
            .filter_map(|e| {
                Some(ExplanationMention {
                    scrutin_uid: e.scrutin_uid().clone(),
                    group_uid: e.group().group_uid()?.clone(),
                    rank: e.rank(),
                    speaker_uid: e.speaker_uid().clone(),
                    speaker_name: e.speaker_name().map(str::to_string),
                })
            })
            .collect())
    }

    async fn stored_versions(
        &self,
    ) -> Result<HashMap<SittingRecordUid, StoredVersion>, RepositoryError> {
        Ok(self.versions.clone())
    }

    async fn replace_sitting(&self, sitting: &StoredSitting) -> Result<(), RepositoryError> {
        if self.failing_on.as_ref() == Some(&sitting.record_uid) {
            return Err(RepositoryError::Database("écriture refusée".to_string()));
        }
        self.written.lock().unwrap().push(sitting.clone());
        Ok(())
    }

    async fn explanations_for(
        &self,
        scrutin: &ScrutinUid,
        sitting_ref: &SittingRef,
    ) -> Result<ScrutinExplanationsRead, RepositoryError> {
        *self.reads.lock().unwrap() += 1;
        let Some(sitting) = self.sittings.iter().find(|s| &s.sitting_ref == sitting_ref) else {
            return Ok(ScrutinExplanationsRead::default());
        };
        Ok(ScrutinExplanationsRead {
            report: Some(StoredReport {
                record_uid: sitting.record_uid.clone(),
                version: sitting.version.clone(),
            }),
            attached: sitting
                .attached
                .iter()
                .find(|(uid, _)| uid == scrutin)
                .map(|(_, has_block)| *has_block),
            explanations: sitting
                .explanations
                .iter()
                .filter(|e| e.scrutin_uid() == scrutin)
                .cloned()
                .collect(),
        })
    }

    async fn final_vote_explanations(
        &self,
        scrutins: &[ScrutinUid],
    ) -> Result<Vec<FinalVoteExplanationsRead>, RepositoryError> {
        *self.reads.lock().unwrap() += 1;
        Ok(self
            .final_vote_reads
            .iter()
            .filter(|read| scrutins.contains(&read.scrutin_uid))
            .cloned()
            .collect())
    }
}
