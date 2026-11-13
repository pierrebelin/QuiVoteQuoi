//! Doubles de la source des comptes rendus et des scrutins à référence de séance.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::scrutin_repository::{
    ScrutinFilter, ScrutinPage, ScrutinRepository, ScrutinSummary,
};
use crate::application::ports::sitting_record_source::{SittingRecordSource, SittingRecords};
use crate::application::ports::{RepositoryError, SourceError};
use crate::domain::dataset::DatasetShape;
use crate::domain::scrutin::{Scrutin, ScrutinUid};
use crate::domain::sitting_record::{ScrutinSittingKey, SittingRecord, SittingRecordUid};

/// Archive de séances: chaque entrée est une séance lisible ou le message de son échec.
#[derive(Default)]
pub struct FakeSittingRecords {
    pub items: Vec<Result<SittingRecord, String>>,
    /// Message d'échec à l'ouverture de l'archive.
    pub open_error: Option<String>,
    /// Comptes rendus publiés à part: la version lue ou le message de son échec;
    /// un compte rendu absent d'ici répond par un échec de téléchargement.
    pub singles: Vec<Result<SittingRecord, (SittingRecordUid, String)>>,
    /// Comptes rendus demandés à part, dans l'ordre des demandes.
    pub requested: Mutex<Vec<SittingRecordUid>>,
}

impl FakeSittingRecords {
    pub fn requested(&self) -> Vec<SittingRecordUid> {
        self.requested.lock().unwrap().clone()
    }
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

    async fn fetch_sitting_record(
        &self,
        uid: &SittingRecordUid,
    ) -> Result<SittingRecord, SourceError> {
        self.requested.lock().unwrap().push(uid.clone());
        for single in &self.singles {
            match single {
                Ok(record) if record.uid() == uid => return Ok(record.clone()),
                Err((failing, message)) if failing == uid => {
                    return Err(SourceError::Parse(message.clone()))
                }
                _ => {}
            }
        }
        Err(SourceError::Download(format!("{uid}: HTTP 404")))
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
    async fn between(
        &self,
        _from: chrono::NaiveDate,
        _to: chrono::NaiveDate,
    ) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        unreachable!()
    }
    async fn count_between(
        &self,
        _from: chrono::NaiveDate,
        _to: chrono::NaiveDate,
    ) -> Result<i64, RepositoryError> {
        unreachable!()
    }
    async fn latest_scrutin_date(&self) -> Result<Option<chrono::NaiveDate>, RepositoryError> {
        unreachable!()
    }
}
