use async_trait::async_trait;
use chrono::NaiveDate;

use crate::domain::dossier::{DossierUid, LegislativeDossier};

pub use super::SourceError;

#[async_trait]
pub trait AssemblySource: Send + Sync {
    /// Dossiers dont la derniere activite n'est pas anterieure a `since`.
    ///
    /// `None` ne pose aucune borne basse: tous les dossiers de la source sont
    /// rendus, quelle que soit leur anciennete.
    async fn fetch_dossiers_since(
        &self,
        since: Option<NaiveDate>,
    ) -> Result<Vec<LegislativeDossier>, SourceError>;

    async fn fetch_dossier_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, SourceError>;

    /// Comme `fetch_dossiers_since`, avec les references brutes des
    /// initiateurs de chaque dossier. `None` ne pose aucune borne basse.
    async fn fetch_dossiers_since_with_refs(
        &self,
        since: Option<NaiveDate>,
    ) -> Result<Vec<(LegislativeDossier, Vec<String>)>, SourceError>;

    async fn fetch_dossier_by_uid_with_refs(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError>;
}
