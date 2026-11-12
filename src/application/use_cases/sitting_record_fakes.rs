//! Doubles de la source des comptes rendus et des scrutins à référence de séance.

use async_trait::async_trait;

use crate::application::ports::scrutin_repository::{
    ScrutinFilter, ScrutinPage, ScrutinRepository, ScrutinSummary,
};
use crate::application::ports::sitting_record_source::{SittingRecordSource, SittingRecords};
use crate::application::ports::{RepositoryError, SourceError};
use crate::domain::dataset::DatasetShape;
use crate::domain::scrutin::{Scrutin, ScrutinUid};
use crate::domain::sitting_record::{ScrutinSittingKey, SittingRecord};

/// Archive de séances: chaque entrée est une séance lisible ou le message de son échec.
#[derive(Default)]
pub struct FakeSittingRecords {
    pub items: Vec<Result<SittingRecord, String>>,
    /// Message d'échec à l'ouverture de l'archive.
    pub open_error: Option<String>,
}

#[async_trait]
impl SittingRecordSource for FakeSittingRecords {
    async fn fetch_sitting_records(
        &self,
        _legislature: u16,
    ) -> Result<SittingRecords, SourceError> {
        if let Some(message) = &self.open_error {
            return Err(SourceError::Download(message.clone()));
        }
        let items: Vec<Result<SittingRecord, SourceError>> = self
            .items
            .iter()
            .map(|item| match item {
                Ok(record) => Ok(record.clone()),
                Err(message) => Err(SourceError::Parse(message.clone())),
            })
            .collect();
        Ok(Box::new(items.into_iter()))
    }
}

/// Scrutins stockés: leurs clés de séance et leurs objets.
#[derive(Default)]
pub struct FakeSittingScrutins {
    pub keys: Vec<ScrutinSittingKey>,
    pub scrutins: Vec<Scrutin>,
}

#[async_trait]
impl ScrutinRepository for FakeSittingScrutins {
    async fn save_scrutins(&self, _s: &[Scrutin]) -> Result<usize, RepositoryError> {
        unreachable!()
    }
    async fn list(&self, _f: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
        unreachable!()
    }
    async fn by_uid(&self, uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
        Ok(self.scrutins.iter().find(|s| s.uid() == uid).cloned())
    }
    async fn by_dossier(&self, _d: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        unreachable!()
    }
    async fn sitting_keys(&self) -> Result<Vec<ScrutinSittingKey>, RepositoryError> {
        Ok(self.keys.clone())
    }
    async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
        unreachable!()
    }
}
