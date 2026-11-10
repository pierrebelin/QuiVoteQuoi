//! Doubles de lecture partages par les tests des cas d usage de lecture.
//!
//! Ports d'etat: fakes en memoire. Les methodes qu'aucun test de lecture
//! n'atteint sont `unreachable!()`.

use async_trait::async_trait;

use crate::application::ports::actor_repository::{ActorRepository, RegistrySummary};
use crate::application::ports::amendment_repository::{
    AmendmentGroupOption, AmendmentPage, AmendmentPageRequest, AmendmentRepository,
    AmendmentSummary, DossierAmendmentCoverage, SignatoryRow,
};
use crate::application::ports::dossier_repository::{
    DossierCriteria, DossierPage, DossierRepository,
};
use crate::application::ports::RepositoryError;
use crate::domain::actor::{
    Actor, ActorDirectory, ActorRegistry, ActorRole, ActorUid, GroupUid, ParliamentaryGroup,
};
use crate::application::ports::scrutin_repository::{
    ScrutinFilter, ScrutinPage, ScrutinRepository, ScrutinSummary,
};
use crate::domain::amendment::Amendment;
use crate::domain::dataset::DatasetShape;
use crate::domain::dossier::{
    CurationStatus, DossierOutcome, DossierUid, LegislativeDossier, LegislativeStage,
    NewLegislativeDossier, Score, StoredDossierState,
};
use crate::domain::scrutin::{Scrutin, ScrutinUid, VoteTally};

/// Ligne de liste d'un scrutin `uid` portant l'objet `subject`.
pub fn scrutin_row(uid: &str, subject: &str) -> ScrutinSummary {
    ScrutinSummary {
        uid: uid.into(),
        number: "1".into(),
        legislature: 17,
        date: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        subject: subject.into(),
        ballot_type_label: "Scrutin public ordinaire".into(),
        outcome_code: "adopted".into(),
        outcome_label: "adopt\u{00e9}".into(),
        tally: VoteTally::default(),
        dossier_uid: None,
        dossier_label: None,
        has_reconstructed_tallies: false,
    }
}

/// Scrutins semes: la page rendue suit l'ordre des lignes, le total est celui donne.
pub struct FakeScrutins {
    pub rows: Vec<ScrutinSummary>,
    pub total: i64,
    pub shape: DatasetShape,
}

#[async_trait]
impl ScrutinRepository for FakeScrutins {
    async fn save_scrutins(&self, _s: &[Scrutin]) -> Result<usize, RepositoryError> {
        unreachable!()
    }
    async fn list(&self, _f: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
        Ok(ScrutinPage {
            items: self.rows.clone(),
            total: self.total,
        })
    }
    async fn by_uid(&self, _u: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
        unreachable!()
    }
    async fn by_dossier(&self, _d: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
        unreachable!()
    }
    async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
        Ok(self.shape.clone())
    }
}

/// Ligne d'amendement d'un auteur `PA1` du groupe `PO_A`, sort adopte, exposé publié.
pub fn amendment_row() -> AmendmentSummary {
    AmendmentSummary {
        uid: "AM1".into(),
        number: "12".into(),
        target_title: "Article 3".into(),
        target_kind: None,
        author_kind: "deputy".into(),
        author_actor_uid: Some("PA1".into()),
        author_label: None,
        author_group_uid: Some("PO_A".into()),
        author_group_origin: "deposit".into(),
        author_group_ambiguous: false,
        fate_code: "adopted".into(),
        fate_label: "Adopt\u{00e9}".into(),
        state_label: None,
        deposited_on: None,
        summary: Some("Court expos\u{00e9}.".into()),
        cosignatory_count: 0,
    }
}

pub struct FakeAmendments {
    pub rows: Vec<AmendmentSummary>,
}

#[async_trait]
impl AmendmentRepository for FakeAmendments {
    async fn save_amendments(&self, _a: &[Amendment]) -> Result<usize, RepositoryError> {
        unreachable!()
    }
    async fn by_dossier(
        &self,
        _d: &str,
        p: &AmendmentPageRequest,
    ) -> Result<AmendmentPage, RepositoryError> {
        let items: Vec<AmendmentSummary> = self
            .rows
            .iter()
            .filter(|row| p.number.as_ref().is_none_or(|n| &row.number == n))
            .cloned()
            .collect();
        Ok(AmendmentPage {
            total: items.len() as i64,
            items,
        })
    }
    async fn groups_by_dossier(
        &self,
        _d: &str,
    ) -> Result<Vec<AmendmentGroupOption>, RepositoryError> {
        Ok(vec![])
    }
    async fn dossier_coverage(
        &self,
        _d: &str,
    ) -> Result<DossierAmendmentCoverage, RepositoryError> {
        Ok(DossierAmendmentCoverage::default())
    }
    async fn signatories_of(&self, _a: &str) -> Result<Vec<SignatoryRow>, RepositoryError> {
        unreachable!()
    }
    async fn last_archive_id(&self, _l: &str) -> Result<Option<String>, RepositoryError> {
        unreachable!()
    }
    async fn remember_archive(&self, _l: &str, _i: &str) -> Result<(), RepositoryError> {
        unreachable!()
    }
}

/// Referentiel: acteur `PA1`, groupe `PO_A` a la couleur donnee.
pub struct FakeActors {
    pub group_color: Option<String>,
}

#[async_trait]
impl ActorRepository for FakeActors {
    async fn save_registry(&self, _r: &ActorRegistry) -> Result<RegistrySummary, RepositoryError> {
        unreachable!()
    }
    async fn load_directory_for(
        &self,
        _uids: &[ActorUid],
    ) -> Result<ActorDirectory, RepositoryError> {
        Ok(ActorDirectory::new(
            vec![Actor::new(
                ActorUid::new("PA1".into()).unwrap(),
                None,
                "Jean".into(),
                "Dupont".into(),
                ActorRole::Deputy,
            )
            .unwrap()],
            vec![ParliamentaryGroup::new(
                GroupUid::new("PO_A".into()).unwrap(),
                17,
                "Groupe A".into(),
                "A".into(),
                self.group_color.clone(),
                None,
                None,
            )
            .unwrap()],
            vec![],
        ))
    }
}

/// Dossier `uid` sans actes ni documents, au sort et a l'etape donnes : ce que
/// la liste des scrutins en lit.
pub fn listed_dossier(
    uid: &str,
    outcome: DossierOutcome,
    stage: Option<LegislativeStage>,
) -> LegislativeDossier {
    LegislativeDossier::new(NewLegislativeDossier {
        uid: DossierUid::new(uid.into()).unwrap(),
        title: "Projet de loi de finances pour 2027".into(),
        procedure: "Projet de loi de finances de l'ann\u{00e9}e".into(),
        legislature: 17,
        url: None,
        summary: None,
        deposit_date: None,
        last_activity_date: chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(),
        last_activity_label: "R\u{00e9}union de commission".into(),
        acts: vec![],
        documents: vec![],
        score: Score::new(0, 0, 0, 0).unwrap(),
        current_stage: stage,
        initiators: vec![],
        committee: None,
        curation_status: CurationStatus::New,
        outcome,
    })
}

/// Dossiers semes, lus par uid. `lookups` compte les lectures : la liste des
/// scrutins doit lire ses dossiers en une fois.
#[derive(Default)]
pub struct FakeDossiers {
    pub dossiers: Vec<LegislativeDossier>,
    pub lookups: std::sync::Mutex<Vec<Vec<String>>>,
}

#[async_trait]
impl DossierRepository for FakeDossiers {
    async fn save_all(&self, _d: &[LegislativeDossier]) -> Result<usize, RepositoryError> {
        unreachable!()
    }
    async fn load_states(
        &self,
    ) -> Result<std::collections::HashMap<String, StoredDossierState>, RepositoryError> {
        unreachable!()
    }
    async fn find_recent(
        &self,
        _since: chrono::NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        unreachable!()
    }
    async fn find_page(
        &self,
        _c: &DossierCriteria,
        _order: crate::application::ports::dossier_repository::DossierOrder,
        _l: i64,
        _o: i64,
    ) -> Result<DossierPage, RepositoryError> {
        unreachable!()
    }
    async fn find_by_uid(
        &self,
        _u: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, RepositoryError> {
        unreachable!()
    }
    async fn find_by_uids(
        &self,
        uids: &[DossierUid],
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        let asked: Vec<String> = uids.iter().map(|u| u.as_str().to_string()).collect();
        let found = self
            .dossiers
            .iter()
            .filter(|d| asked.iter().any(|uid| uid == d.uid().as_str()))
            .cloned()
            .collect();
        self.lookups.lock().unwrap().push(asked);
        Ok(found)
    }
    async fn find_suggestions(
        &self,
        _count: usize,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        unreachable!()
    }
    async fn update_curation_status(
        &self,
        _u: &DossierUid,
        _s: CurationStatus,
    ) -> Result<bool, RepositoryError> {
        unreachable!()
    }
}
