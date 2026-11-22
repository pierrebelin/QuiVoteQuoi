use async_trait::async_trait;

use crate::domain::scrutin::DossierReference;

pub use super::RepositoryError;

/// Scrutin qu'un acte de décision de séance désigne comme celui qui a tranché
/// le texte (`legislative_acts.decision_scrutin_uid`), avec le dossier qui le
/// cite.
#[derive(Debug, Clone)]
pub struct DecisionVote {
    pub scrutin_uid: String,
    pub dossier: DossierReference,
}

/// Lien scrutin ↔ dossier vu depuis le jeu des dossiers législatifs.
///
/// Le fichier d'un scrutin nomme parfois un autre dossier que celui dont la
/// décision le cite : `VTANR5L17V10080`, ensemble du texte vie chère outre-mer,
/// y porte le dossier Territoires zéro chômeur voté le même jour.
#[async_trait]
pub trait DecisionVoteRepository: Send + Sync {
    async fn decision_votes(&self) -> Result<Vec<DecisionVote>, RepositoryError>;
}
