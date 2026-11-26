use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::assembly_source::AssemblySource;
use crate::application::ports::dossier_repository::DossierRepository;
use crate::domain::actor::ActorUid;
use crate::domain::dossier::DossierUid;

#[derive(Debug, thiserror::Error)]
pub enum SaveDossierError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::assembly_source::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::dossier_repository::RepositoryError),
    #[error("dossier not found: {0}")]
    NotFound(String),
}

pub struct SaveDossier<'a> {
    source: &'a dyn AssemblySource,
    repository: &'a dyn DossierRepository,
    actor_repository: &'a dyn ActorRepository,
}

impl<'a> SaveDossier<'a> {
    pub fn new(
        source: &'a dyn AssemblySource,
        repository: &'a dyn DossierRepository,
        actor_repository: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            source,
            repository,
            actor_repository,
        }
    }

    pub async fn execute(&self, uid: &DossierUid) -> Result<(), SaveDossierError> {
        let (mut dossier, refs) = self
            .source
            .fetch_dossier_by_uid_with_refs(uid)
            .await?
            .ok_or_else(|| SaveDossierError::NotFound(uid.as_str().to_string()))?;

        if !refs.is_empty() && dossier.initiators().is_empty() {
            let uids: Vec<ActorUid> = refs
                .into_iter()
                .filter_map(|r| ActorUid::new(r).ok())
                .collect();
            let directory = self.actor_repository.load_directory_for(&uids).await?;
            dossier.attach_initiators(&uids, &directory);
        }

        self.repository.save_all(&[dossier]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::ports::assembly_source::SourceError;
    use crate::application::use_cases::fakes::dossier_repository::InMemoryDossierRepository;
    use crate::application::use_cases::refresh_dossiers::tests::InMemoryActorRepository;
    use crate::domain::dossier::{
        DossierOutcome, LegislativeDossier, NewLegislativeDossier, Score,
    };

    struct FakeSource {
        dossiers: Mutex<HashMap<String, (LegislativeDossier, Vec<String>)>>,
    }

    #[async_trait]
    impl AssemblySource for FakeSource {
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
            unreachable!()
        }

        async fn fetch_dossier_by_uid(
            &self,
            _uid: &DossierUid,
        ) -> Result<Option<LegislativeDossier>, SourceError> {
            unreachable!()
        }

        async fn fetch_dossier_by_uid_with_refs(
            &self,
            uid: &DossierUid,
        ) -> Result<Option<(LegislativeDossier, Vec<String>)>, SourceError> {
            let store = self.dossiers.lock().unwrap();
            Ok(store.get(uid.as_str()).cloned())
        }
    }

    #[tokio::test]
    async fn saves_dossier_with_initiators_dated_on_the_deposit_date() {
        let mut source_map = HashMap::new();
        source_map.insert(
            "D1".into(),
            (
                LegislativeDossier::new(NewLegislativeDossier {
                    uid: DossierUid::new("D1".into()).unwrap(),
                    title: "Loi test".into(),
                    procedure: "PL".into(),
                    legislature: 17,
                    url: None,
                    summary: None,
                    deposit_date: NaiveDate::from_ymd_opt(2025, 5, 13),
                    last_activity_date: NaiveDate::from_ymd_opt(2026, 6, 25).unwrap(),
                    last_activity_label: "Dépôt".into(),
                    acts: vec![],
                    documents: vec![],
                    score: Score::new(2, 4, 2, 23).unwrap(),
                    current_stage: None,
                    initiators: vec![],
                    committee: None,
                    outcome: DossierOutcome::NoRecordedConclusion,
                }),
                vec!["PA111111".into()],
            ),
        );

        let source = FakeSource {
            dossiers: Mutex::new(source_map),
        };
        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let uc = SaveDossier::new(&source, &repo, &actors);
        let uid = DossierUid::new("D1".into()).unwrap();
        uc.execute(&uid).await.unwrap();

        assert_eq!(repo.count(), 1);
        let dossier = repo.get("D1").unwrap();
        assert_eq!(dossier.initiators().len(), 1);
        let initiator = &dossier.initiators()[0];
        assert_eq!(initiator.full_name(), "Jean Dupont");
        assert_eq!(initiator.group().unwrap().abbrev, "A");
        assert_eq!(
            initiator.reference_date(),
            NaiveDate::from_ymd_opt(2025, 5, 13)
        );
    }

    #[tokio::test]
    async fn returns_not_found_when_missing_from_source() {
        let source = FakeSource {
            dossiers: Mutex::new(HashMap::new()),
        };
        let repo = InMemoryDossierRepository::new();
        let actors = InMemoryActorRepository::with_deputy_changing_group();

        let uc = SaveDossier::new(&source, &repo, &actors);
        let uid = DossierUid::new("UNKNOWN".into()).unwrap();
        let result = uc.execute(&uid).await;

        assert!(matches!(result, Err(SaveDossierError::NotFound(_))));
    }
}
