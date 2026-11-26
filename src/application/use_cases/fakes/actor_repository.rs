//! Fake d'etat du port `ActorRepository`: un referentiel d'acteurs en memoire.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::actor_repository::{
    ActorRepository, RegistrySummary, RepositoryError,
};
use crate::application::use_cases::fakes::fixtures::date;
use crate::domain::actor::{
    Actor, ActorDirectory, ActorRegistry, ActorRole, ActorUid, GroupMembership, GroupUid,
    MembershipPeriod, MembershipQuality, ParliamentaryGroup,
};

/// Referentiel in-memory: un depute passe du groupe A au groupe B le 1er juillet 2025.
pub struct InMemoryActorRepository {
    registry: Mutex<ActorRegistry>,
    pub requested_uids: Mutex<Vec<ActorUid>>,
}

impl InMemoryActorRepository {
    pub fn with_deputy_changing_group() -> Self {
        let actor = Actor::new(
            ActorUid::new("PA111111".into()).unwrap(),
            Some("M.".into()),
            "Jean".into(),
            "Dupont".into(),
            ActorRole::Deputy,
        )
        .unwrap();

        let group = |uid: &str, abbrev: &str, label: &str| {
            ParliamentaryGroup::new(
                GroupUid::new(uid.into()).unwrap(),
                17,
                label.into(),
                abbrev.into(),
                None,
                None,
                None,
            )
            .unwrap()
        };

        let membership = |source: &str, group: &str, start, end| {
            GroupMembership::new(
                source.into(),
                ActorUid::new("PA111111".into()).unwrap(),
                GroupUid::new(group.into()).unwrap(),
                17,
                MembershipPeriod::new(start, end).unwrap(),
                MembershipQuality::new("Membre".into()).unwrap(),
            )
        };

        Self {
            registry: Mutex::new(ActorRegistry {
                actors: vec![actor],
                groups: vec![
                    group("PO_A", "A", "Groupe A"),
                    group("PO_B", "B", "Groupe B"),
                ],
                memberships: vec![
                    membership("PM1", "PO_A", date(2024, 7, 19), Some(date(2025, 6, 30))),
                    membership("PM2", "PO_B", date(2025, 7, 1), None),
                ],
            }),
            requested_uids: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl ActorRepository for InMemoryActorRepository {
    async fn save_registry(
        &self,
        registry: &ActorRegistry,
    ) -> Result<RegistrySummary, RepositoryError> {
        let summary = RegistrySummary {
            actors: registry.actors.len(),
            groups: registry.groups.len(),
            memberships: registry.memberships.len(),
        };
        *self.registry.lock().unwrap() = registry.clone();
        Ok(summary)
    }

    async fn load_directory_for(
        &self,
        actor_uids: &[ActorUid],
    ) -> Result<ActorDirectory, RepositoryError> {
        self.requested_uids
            .lock()
            .unwrap()
            .extend_from_slice(actor_uids);

        let registry = self.registry.lock().unwrap();
        let actors: Vec<Actor> = registry
            .actors
            .iter()
            .filter(|a| actor_uids.contains(a.uid()))
            .cloned()
            .collect();
        let memberships: Vec<GroupMembership> = registry
            .memberships
            .iter()
            .filter(|m| actor_uids.contains(m.actor_uid()))
            .cloned()
            .collect();

        Ok(ActorDirectory::new(
            actors,
            registry.groups.clone(),
            memberships,
        ))
    }
}
