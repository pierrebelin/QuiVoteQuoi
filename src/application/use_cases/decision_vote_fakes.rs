//! Double en memoire des scrutins de decision cites par les dossiers.

use async_trait::async_trait;

use crate::application::ports::decision_vote_repository::{DecisionVote, DecisionVoteRepository};
use crate::application::ports::RepositoryError;
use crate::domain::dossier::DossierUid;
use crate::domain::scrutin::DossierReference;

#[derive(Default)]
pub struct InMemoryDecisionVotes {
    votes: Vec<DecisionVote>,
}

impl InMemoryDecisionVotes {
    /// Aucun dossier ne cite de scrutin de decision.
    pub fn none() -> Self {
        Self::default()
    }

    /// Le dossier `dossier_uid`, intitule `title`, cite `scrutin_uid` dans sa
    /// decision.
    pub fn citing(mut self, scrutin_uid: &str, dossier_uid: &str, title: &str) -> Self {
        self.votes.push(DecisionVote {
            scrutin_uid: scrutin_uid.to_string(),
            dossier: DossierReference {
                uid: DossierUid::new(dossier_uid.to_string()).unwrap(),
                label: title.to_string(),
            },
        });
        self
    }
}

#[async_trait]
impl DecisionVoteRepository for InMemoryDecisionVotes {
    async fn decision_votes(&self) -> Result<Vec<DecisionVote>, RepositoryError> {
        Ok(self.votes.clone())
    }
}
