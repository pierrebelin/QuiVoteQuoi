//! Read model officiel utilise par la fiche dossier.

use async_trait::async_trait;

use crate::domain::dossier_group_facts::DossierGroupFacts;

pub use super::RepositoryError;

#[async_trait]
pub trait DossierGroupActionsRepository: Send + Sync {
    async fn load_facts(
        &self,
        dossier_uid: &str,
    ) -> Result<Option<DossierGroupFacts>, RepositoryError>;
    async fn list_facts(&self, limit: usize) -> Result<Vec<DossierGroupFacts>, RepositoryError>;
}
