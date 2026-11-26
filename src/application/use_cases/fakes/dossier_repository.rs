//! Fake d'etat du port `DossierRepository`: on verifie l'etat persiste.
//!
//! Les dossiers gardent leur ordre d'insertion (un tri stable laisse les
//! egalites dans cet ordre). Les methodes qu'aucun test n'appelle restent en
//! `unreachable!()`.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::dossier_repository::{
    DossierCriteria, DossierListing, DossierOrder, DossierPage, DossierRepository,
    DossierScrutinTally, LawStateCounts, RepositoryError,
};
use crate::domain::dossier::{DossierUid, LegislativeDossier, StoredDossierState};

#[derive(Default)]
pub struct InMemoryDossierRepository {
    pub dossiers: Mutex<Vec<LegislativeDossier>>,
    /// Uid de chaque dossier reellement passe a `save_all`, dans l'ordre.
    /// C'est ce que le rafraichissement incremental doit reduire.
    pub written: Mutex<Vec<String>>,
    /// Uid des dossiers stockes avant le calcul de l'etat « ou en est la
    /// loi » : leur etat n'a jamais ete ecrit. Une ecriture les en retire.
    pub without_law_progress: Mutex<HashSet<String>>,
    /// Scrutins de chaque dossier, par uid ; absent : aucun scrutin.
    tallies: HashMap<String, DossierScrutinTally>,
}

impl InMemoryDossierRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// Etat initial de la base, pose sans passer par `save_all` : le journal
    /// des ecritures reste vide.
    pub fn with_dossiers(dossiers: Vec<LegislativeDossier>) -> Self {
        Self {
            dossiers: Mutex::new(dossiers),
            ..Self::default()
        }
    }

    pub fn with_tallies(mut self, tallies: HashMap<String, DossierScrutinTally>) -> Self {
        self.tallies = tallies;
        self
    }

    pub fn count(&self) -> usize {
        self.dossiers.lock().unwrap().len()
    }

    pub fn get(&self, uid: &str) -> Option<LegislativeDossier> {
        self.dossiers
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.uid().as_str() == uid)
            .cloned()
    }

    fn tally_of(&self, dossier: &LegislativeDossier) -> DossierScrutinTally {
        self.tallies
            .get(dossier.uid().as_str())
            .copied()
            .unwrap_or_default()
    }
}

#[async_trait]
impl DossierRepository for InMemoryDossierRepository {
    async fn save_all(&self, dossiers: &[LegislativeDossier]) -> Result<usize, RepositoryError> {
        let mut store = self.dossiers.lock().unwrap();
        for d in dossiers {
            self.written
                .lock()
                .unwrap()
                .push(d.uid().as_str().to_string());
            self.without_law_progress
                .lock()
                .unwrap()
                .remove(d.uid().as_str());
            match store
                .iter_mut()
                .find(|stored| stored.uid().as_str() == d.uid().as_str()) {
                Some(stored) => *stored = d.clone(),
                None => store.push(d.clone()),
            }
        }
        Ok(dossiers.len())
    }

    async fn load_states(&self) -> Result<HashMap<String, StoredDossierState>, RepositoryError> {
        let without_law_progress = self.without_law_progress.lock().unwrap();
        Ok(self
            .dossiers
            .lock()
            .unwrap()
            .iter()
            .map(|d| {
                (
                    d.uid().as_str().to_string(),
                    StoredDossierState {
                        last_activity_date: d.last_activity_date(),
                        act_count: d.acts().len(),
                        outcome_is_final: d.outcome().is_final(),
                        law_progress_computed: !without_law_progress.contains(d.uid().as_str()),
                        decision_fingerprint: d.decision_fingerprint(),
                    },
                )
            })
            .collect())
    }

    async fn find_recent(
        &self,
        since: NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        let store = self.dossiers.lock().unwrap();
        let mut result: Vec<_> = store
            .iter()
            .filter(|d| d.last_activity_date() >= since)
            .cloned()
            .collect();
        result.sort_by(|a, b| b.last_activity_date().cmp(&a.last_activity_date()));
        Ok(result)
    }

    async fn find_page(
        &self,
        criteria: &DossierCriteria,
        order: DossierOrder,
        limit: i64,
        offset: i64,
    ) -> Result<DossierPage, RepositoryError> {
        let store = self.dossiers.lock().unwrap();
        let perimeter = criteria.without_law_states();
        let mut state_counts = LawStateCounts::default();
        for dossier in store
            .iter()
            .filter(|d| matches(d, &perimeter))
            .filter(|d| !criteria.with_scrutins || self.tally_of(d).count > 0)
        {
            if let Some(state) = dossier.law_state() {
                state_counts.add(state.kind(), 1);
            }
        }
        let mut kept: Vec<_> = store
            .iter()
            .filter(|d| matches(d, criteria))
            .filter(|d| !criteria.with_scrutins || self.tally_of(d).count > 0)
            .cloned()
            .collect();
        match order {
            DossierOrder::LastActivity => {
                kept.sort_by_key(|d| std::cmp::Reverse(d.last_activity_date()))
            }
            // `None` passe après toute date : `Option` range `None` en
            // premier, l'ordre décroissant le renvoie en fin de liste.
            DossierOrder::LastScrutin => kept.sort_by(|a, b| {
                self.tally_of(b)
                    .last_date
                    .cmp(&self.tally_of(a).last_date)
                    .then_with(|| b.uid().as_str().cmp(a.uid().as_str()))
            }),
            DossierOrder::StateDate => kept.sort_by(|a, b| {
                let date = |d: &LegislativeDossier| d.law_state().map(|s| s.date());
                date(b)
                    .cmp(&date(a))
                    .then_with(|| b.uid().as_str().cmp(a.uid().as_str()))
            }),
        }
        Ok(DossierPage {
            state_counts,
            total: kept.len() as i64,
            items: kept
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|dossier| DossierListing {
                    scrutins: self.tally_of(&dossier),
                    dossier,
                    families: vec![],
                })
                .collect(),
        })
    }

    async fn find_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, RepositoryError> {
        Ok(self.get(uid.as_str()))
    }

    async fn find_by_uids(
        &self,
        _uids: &[DossierUid],
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        unreachable!()
    }

    async fn find_with_decisions_between(
        &self,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        unreachable!()
    }
}

/// Le fake reproduit les critères que le dépôt applique en SQL : sans cela
/// les tests de filtre ne vérifieraient que l'appel, pas son effet.
fn matches(dossier: &LegislativeDossier, criteria: &DossierCriteria) -> bool {
    let title_matches = criteria.search.as_ref().is_none_or(|needle| {
        dossier
            .title()
            .to_lowercase()
            .contains(&needle.to_lowercase())
    });
    let outcome_matches = criteria.outcome_kinds.is_empty()
        || criteria
            .outcome_kinds
            .iter()
            .any(|kind| dossier.outcome().kind() == kind);
    let initiative_matches = criteria.initiative.is_none_or(|initiative| {
        dossier
            .procedure()
            .starts_with(initiative.procedure_prefix())
    });

    let state_matches = criteria.law_states.is_empty()
        || dossier
            .law_state()
            .is_some_and(|state| criteria.law_states.contains(&state.kind()));

    title_matches && outcome_matches && initiative_matches && state_matches
}
