//! Double en memoire du dépôt des explications de vote.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::vote_explanation_repository::{
    StoredSitting, StoredVersion, VoteExplanationRepository,
};
use crate::application::ports::RepositoryError;
use crate::domain::sitting_record::SittingRecordUid;

#[derive(Default)]
pub struct InMemoryVoteExplanations {
    versions: HashMap<SittingRecordUid, StoredVersion>,
    failing_on: Option<SittingRecordUid>,
    written: Mutex<Vec<StoredSitting>>,
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

    pub fn written(&self) -> Vec<StoredSitting> {
        self.written.lock().unwrap().clone()
    }
}

#[async_trait]
impl VoteExplanationRepository for InMemoryVoteExplanations {
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
}
