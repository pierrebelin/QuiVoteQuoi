use crate::application::ports::actor_repository::{ActorRepository, RegistrySummary};
use crate::application::ports::actor_source::ActorSource;
use crate::application::use_cases::journaling::Journaling;
use crate::domain::ingestion::IngestionSource;
use crate::domain::legislature::CURRENT_LEGISLATURE;

#[derive(Debug, thiserror::Error)]
pub enum RefreshRegistryError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::RepositoryError),
}

/// CU-01 — Ingerer le referentiel.
pub struct RefreshActorRegistry<'a> {
    source: &'a dyn ActorSource,
    repository: &'a dyn ActorRepository,
    journaling: Journaling<'a>,
}

impl<'a> RefreshActorRegistry<'a> {
    pub fn new(
        source: &'a dyn ActorSource,
        repository: &'a dyn ActorRepository,
        journaling: Journaling<'a>,
    ) -> Self {
        Self {
            source,
            repository,
            journaling,
        }
    }

    pub async fn execute(&self) -> Result<RegistrySummary, RefreshRegistryError> {
        let result = self.refresh().await;
        self.journaling
            .record(IngestionSource::ActorRegistry, &result, |summary| {
                summary.actors as u64
            })
            .await;
        result
    }

    async fn refresh(&self) -> Result<RegistrySummary, RefreshRegistryError> {
        let registry = self.source.fetch_registry(CURRENT_LEGISLATURE).await?;
        let summary = self.repository.save_registry(&registry).await?;

        tracing::info!(
            "Actor registry refreshed: {} actors, {} groups, {} memberships",
            summary.actors,
            summary.groups,
            summary.memberships
        );

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, NaiveDate, TimeZone, Utc};
    use std::sync::Mutex;

    use crate::application::use_cases::journal_fakes::InMemoryIngestionJournal;
    use crate::domain::ingestion::{CheckOutcome, IngestionSource};

    use crate::application::ports::actor_repository::RepositoryError;
    use crate::application::ports::actor_source::SourceError;
    use crate::domain::actor::{
        Actor, ActorDirectory, ActorRegistry, ActorRole, ActorUid, GroupMembership, GroupUid,
        MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };

    fn sample_registry() -> ActorRegistry {
        let actor = Actor::new(
            ActorUid::new("PA1".into()).unwrap(),
            Some("M.".into()),
            "Jean".into(),
            "Dupont".into(),
            ActorRole::Deputy,
        )
        .unwrap();

        let group = ParliamentaryGroup::new(
            GroupUid::new("PO_A".into()).unwrap(),
            17,
            "Groupe A".into(),
            "A".into(),
            None,
            None,
            None,
        )
        .unwrap();

        let membership = GroupMembership::new(
            "PM1".into(),
            ActorUid::new("PA1".into()).unwrap(),
            GroupUid::new("PO_A".into()).unwrap(),
            17,
            MembershipPeriod::new(NaiveDate::from_ymd_opt(2024, 7, 19).unwrap(), None).unwrap(),
            MembershipQuality::new("Membre".into()).unwrap(),
        );

        ActorRegistry {
            actors: vec![actor],
            groups: vec![group],
            memberships: vec![membership],
        }
    }

    struct FakeActorSource {
        registry: Option<ActorRegistry>,
        requested_legislature: Mutex<Option<u16>>,
    }

    #[async_trait]
    impl ActorSource for FakeActorSource {
        async fn fetch_registry(&self, legislature: u16) -> Result<ActorRegistry, SourceError> {
            *self.requested_legislature.lock().unwrap() = Some(legislature);
            self.registry
                .clone()
                .ok_or_else(|| SourceError::Download("source unavailable".into()))
        }
    }

    struct InMemoryActorRepository {
        registry: Mutex<Option<ActorRegistry>>,
    }

    impl InMemoryActorRepository {
        fn new() -> Self {
            Self {
                registry: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl ActorRepository for InMemoryActorRepository {
        async fn save_registry(
            &self,
            registry: &ActorRegistry,
        ) -> Result<RegistrySummary, RepositoryError> {
            *self.registry.lock().unwrap() = Some(registry.clone());
            Ok(RegistrySummary {
                actors: registry.actors.len(),
                groups: registry.groups.len(),
                memberships: registry.memberships.len(),
            })
        }

        async fn load_directory_for(
            &self,
            _actor_uids: &[ActorUid],
        ) -> Result<ActorDirectory, RepositoryError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn stores_the_registry_of_the_current_legislature() {
        let source = FakeActorSource {
            registry: Some(sample_registry()),
            requested_legislature: Mutex::new(None),
        };
        let repository = InMemoryActorRepository::new();

        let summary = RefreshActorRegistry::new(&source, &repository, Journaling::off())
            .execute()
            .await
            .unwrap();

        assert_eq!(
            summary,
            RegistrySummary {
                actors: 1,
                groups: 1,
                memberships: 1
            }
        );
        assert_eq!(
            *source.requested_legislature.lock().unwrap(),
            Some(CURRENT_LEGISLATURE)
        );

        let stored = repository.registry.lock().unwrap();
        let stored = stored.as_ref().unwrap();
        assert_eq!(stored.groups[0].abbrev(), "A");
        assert_eq!(stored.memberships[0].quality().as_str(), "Membre");
    }

    #[tokio::test]
    async fn leaves_the_previous_registry_untouched_when_the_source_is_down() {
        let source = FakeActorSource {
            registry: None,
            requested_legislature: Mutex::new(None),
        };
        let repository = InMemoryActorRepository::new();

        let result = RefreshActorRegistry::new(&source, &repository, Journaling::off())
            .execute()
            .await;

        assert!(matches!(result, Err(RefreshRegistryError::Source(_))));
        assert!(repository.registry.lock().unwrap().is_none());
    }

    fn attempted_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 8, 30, 0).unwrap()
    }

    #[tokio::test]
    async fn should_record_succeeded_check_with_actors_read_when_registry_refresh_succeeds() {
        let source = FakeActorSource {
            registry: Some(sample_registry()),
            requested_legislature: Mutex::new(None),
        };
        let repository = InMemoryActorRepository::new();
        let journal = InMemoryIngestionJournal::default();

        RefreshActorRegistry::new(
            &source,
            &repository,
            Journaling::to(&journal, attempted_at()),
        )
        .execute()
        .await
        .unwrap();

        let recorded = journal.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].source(), IngestionSource::ActorRegistry);
        assert_eq!(recorded[0].attempted_at(), attempted_at());
        assert_eq!(
            recorded[0].outcome(),
            &CheckOutcome::Succeeded { items_read: 1 }
        );
    }

    #[tokio::test]
    async fn should_record_failed_check_when_actor_source_fails() {
        let source = FakeActorSource {
            registry: None,
            requested_legislature: Mutex::new(None),
        };
        let repository = InMemoryActorRepository::new();
        let journal = InMemoryIngestionJournal::default();

        let result = RefreshActorRegistry::new(
            &source,
            &repository,
            Journaling::to(&journal, attempted_at()),
        )
        .execute()
        .await;

        assert!(matches!(result, Err(RefreshRegistryError::Source(_))));
        let recorded = journal.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].source(), IngestionSource::ActorRegistry);
        assert_eq!(recorded[0].attempted_at(), attempted_at());
        assert_eq!(
            recorded[0].outcome(),
            &CheckOutcome::Failed {
                reason: "download failed: source unavailable".to_string()
            }
        );
    }
}
