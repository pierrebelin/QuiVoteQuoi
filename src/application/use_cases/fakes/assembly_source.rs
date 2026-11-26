//! Fake du port `AssemblySource`: une source de dossiers fixe, ou en panne.
//!
//! Chaque dossier est servi avec les references d'acteurs de ses initiateurs.
//! Les methodes qu'aucun test n'appelle restent en `unreachable!()`.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::assembly_source::{AssemblySource, SourceError};
use crate::domain::dossier::{DossierUid, LegislativeDossier};

#[derive(Default)]
pub struct FakeAssemblySource {
    dossiers: Vec<(LegislativeDossier, Vec<String>)>,
    /// Message d'une source indisponible : chaque appel echoue avec lui.
    failure: Option<String>,
}

impl FakeAssemblySource {
    pub fn with_refs(dossiers: Vec<(LegislativeDossier, Vec<String>)>) -> Self {
        Self {
            dossiers,
            failure: None,
        }
    }

    pub fn without_refs(dossiers: Vec<LegislativeDossier>) -> Self {
        Self::with_refs(dossiers.into_iter().map(|d| (d, vec![])).collect())
    }

    pub fn down(message: &str) -> Self {
        Self {
            dossiers: vec![],
            failure: Some(message.to_string()),
        }
    }

    fn check_available(&self) -> Result<(), SourceError> {
        match &self.failure {
            Some(message) => Err(SourceError::Download(message.clone())),
            None => Ok(()),
        }
    }

    fn entry(&self, uid: &DossierUid) -> Option<(LegislativeDossier, Vec<String>)> {
        self.dossiers
            .iter()
            .find(|(d, _)| d.uid().as_str() == uid.as_str())
            .cloned()
    }
}

#[async_trait]
impl AssemblySource for FakeAssemblySource {
    async fn fetch_dossiers_since(
        &self,
        _since: Option<NaiveDate>,
    ) -> Result<Vec<LegislativeDossier>, SourceError> {
        unreachable!()
    }

    async fn fetch_dossiers_since_with_refs(
        &self,
        _since: Option<NaiveDate>,
    ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError> {
        self.check_available()?;
        Ok(self.dossiers.clone())
    }

    async fn fetch_dossier_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, SourceError> {
        self.check_available()?;
        Ok(self.entry(uid).map(|(dossier, _)| dossier))
    }

    async fn fetch_dossier_by_uid_with_refs(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
        self.check_available()?;
        Ok(self.entry(uid))
    }
}
