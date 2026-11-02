use std::collections::HashSet;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::scrutin_repository::ScrutinRepository;
use crate::application::ports::scrutin_source::ScrutinSource;
use crate::domain::actor::ActorUid;
use crate::domain::legislature::CURRENT_LEGISLATURE;
use crate::domain::scrutin::Scrutin;

#[derive(Debug, thiserror::Error)]
pub enum RefreshScrutinsError {
    #[error("{0}")]
    Source(#[from] crate::application::ports::SourceError),
    #[error("{0}")]
    Repository(#[from] crate::application::ports::RepositoryError),
}

/// Ce qu'un rafraichissement a produit. Les lacunes y figurent explicitement:
/// un chiffre reconstruit ou une position sans groupe doit se voir (README.md §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrutinsSummary {
    pub scrutins: usize,
    pub without_dossier: usize,
    /// Scrutins dont au moins une ligne de groupe a ete reconstruite (RM-03).
    pub reconstructed_scrutins: usize,
    /// Positions nominales qu'aucun groupe n'a pu accueillir. Toute valeur non
    /// nulle rend un decompte reconstruit incomplet.
    pub unresolved_votes: usize,
    /// Renseigne quand le referentiel ne permet pas la reconstruction.
    pub registry_anomaly: Option<String>,
}

/// CU-01 — Ingerer les scrutins.
pub struct RefreshScrutins<'a> {
    source: &'a dyn ScrutinSource,
    repository: &'a dyn ScrutinRepository,
    actor_repository: &'a dyn ActorRepository,
}

impl<'a> RefreshScrutins<'a> {
    pub fn new(
        source: &'a dyn ScrutinSource,
        repository: &'a dyn ScrutinRepository,
        actor_repository: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            source,
            repository,
            actor_repository,
        }
    }

    pub async fn execute(&self) -> Result<ScrutinsSummary, RefreshScrutinsError> {
        let mut scrutins = self.source.fetch_scrutins(CURRENT_LEGISLATURE).await?;

        let mut summary = self.reconstruct_missing_groups(&mut scrutins).await?;

        summary.scrutins = self.repository.save_scrutins(&scrutins).await?;
        summary.without_dossier = scrutins.iter().filter(|s| s.dossier().is_none()).count();

        tracing::info!(
            "Scrutins refreshed: {} stored, {} without a dossier, {} with a reconstructed breakdown",
            summary.scrutins,
            summary.without_dossier,
            summary.reconstructed_scrutins
        );
        if summary.unresolved_votes > 0 {
            tracing::warn!(
                "{} nominal votes could not be attached to a group: reconstructed breakdowns are incomplete",
                summary.unresolved_votes
            );
        }

        Ok(summary)
    }

    /// RM-03: les scrutins ou la source ne nomme pas les groupes sont
    /// reconstruits depuis les positions nominales et l'appartenance a la date
    /// du scrutin (RM-01 de la spec ACTEURS).
    async fn reconstruct_missing_groups(
        &self,
        scrutins: &mut [Scrutin],
    ) -> Result<ScrutinsSummary, RefreshScrutinsError> {
        let mut summary = ScrutinsSummary::default();

        let mut wanted: HashSet<ActorUid> = HashSet::new();
        for scrutin in scrutins.iter() {
            if scrutin.has_sentinel_tallies() {
                wanted.extend(scrutin.actors_under_sentinel());
            }
        }
        if wanted.is_empty() {
            return Ok(summary);
        }

        let uids: Vec<ActorUid> = wanted.into_iter().collect();
        let directory = self.actor_repository.load_directory_for(&uids).await?;
        if directory.is_empty() {
            // Referentiel absent: reconstruire produirait des groupes vides.
            // La lacune remonte, les repartitions restent celles de la source.
            let anomaly = format!(
                "actor registry empty: {} scrutins keep their unnamed group breakdown",
                scrutins.iter().filter(|s| s.has_sentinel_tallies()).count()
            );
            tracing::warn!("{anomaly}");
            summary.registry_anomaly = Some(anomaly);
            return Ok(summary);
        }

        for scrutin in scrutins.iter_mut() {
            if !scrutin.has_sentinel_tallies() {
                continue;
            }

            // RM-04 de la spec ACTEURS: acteur inconnu ou groupe ambigu a cette
            // date, rien n'est devine; le vote reste sans groupe et le rapport
            // le compte comme non resolu.
            let report = scrutin.attach_groups(&directory);
            if report.happened() {
                summary.reconstructed_scrutins += 1;
                summary.unresolved_votes += report.unresolved_votes;
            }
        }

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::sync::Mutex;

    use crate::application::ports::actor_repository::{RegistrySummary, RepositoryError};
    use crate::application::ports::scrutin_repository::{
        ScrutinFilter, ScrutinPage, ScrutinSummary as PageSummary,
    };
    use crate::application::ports::scrutin_source::SourceError;
    use crate::domain::actor::{
        Actor, ActorDirectory, ActorRegistry, ActorRole, GroupMembership, GroupUid,
        MembershipPeriod, MembershipQuality, ParliamentaryGroup,
    };
    use crate::domain::dataset::DatasetShape;
    use crate::domain::scrutin::{
        BallotType, GroupTally, NominalVote, Outcome, ScrutinUid, TallyOrigin, VotePosition,
        VoteSynthesis, VoteTally, MISSING_GROUP_SENTINEL,
    };

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 12, 2).unwrap()
    }

    fn scrutin(uid: &str, tallies: Vec<GroupTally>, votes: Vec<NominalVote>) -> Scrutin {
        Scrutin::new(
            ScrutinUid::new(uid.into()).unwrap(),
            "489".into(),
            17,
            date(),
            None,
            None,
            None,
            BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            Outcome::new("rejet\u{00e9}".into(), "rejet\u{00e9}".into()).unwrap(),
            None,
            "objet".into(),
            VoteSynthesis {
                voters: 2,
                expressed: 2,
                required: 2,
                announcement: "annonce".into(),
                tally: VoteTally::default(),
            },
            tallies,
            votes,
            vec![],
            None,
        )
        .unwrap()
    }

    fn sentinel_line() -> GroupTally {
        GroupTally {
            group_uid: GroupUid::new(MISSING_GROUP_SENTINEL.into()).unwrap(),
            member_count: None,
            majority_position: None,
            tally: VoteTally::default(),
            origin: TallyOrigin::Published,
        }
    }

    fn vote(actor: &str, position: VotePosition) -> NominalVote {
        NominalVote {
            actor_uid: ActorUid::new(actor.into()).unwrap(),
            group_uid: None,
            position,
            cause: None,
            by_delegation: false,
            seat: None,
        }
    }

    struct FakeScrutinSource {
        scrutins: Mutex<Option<Vec<Scrutin>>>,
    }

    #[async_trait]
    impl ScrutinSource for FakeScrutinSource {
        async fn fetch_scrutins(&self, _legislature: u16) -> Result<Vec<Scrutin>, SourceError> {
            self.scrutins
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| SourceError::Download("source unavailable".into()))
        }
    }

    struct InMemoryScrutinRepository {
        stored: Mutex<Vec<Scrutin>>,
    }

    #[async_trait]
    impl ScrutinRepository for InMemoryScrutinRepository {
        async fn save_scrutins(&self, scrutins: &[Scrutin]) -> Result<usize, RepositoryError> {
            let mut stored = self.stored.lock().unwrap();
            stored.clear();
            stored.extend(scrutins.iter().cloned());
            Ok(scrutins.len())
        }

        async fn list(&self, _filter: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
            unreachable!()
        }

        async fn by_uid(&self, _uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
            unreachable!()
        }

        async fn by_dossier(&self, _uid: &str) -> Result<Vec<PageSummary>, RepositoryError> {
            unreachable!()
        }

        async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
            unreachable!()
        }
    }

    struct StubActorRepository {
        empty: bool,
        requested: Mutex<Vec<ActorUid>>,
    }

    #[async_trait]
    impl ActorRepository for StubActorRepository {
        async fn save_registry(
            &self,
            _registry: &ActorRegistry,
        ) -> Result<RegistrySummary, RepositoryError> {
            unreachable!()
        }

        async fn load_directory_for(
            &self,
            actor_uids: &[ActorUid],
        ) -> Result<ActorDirectory, RepositoryError> {
            self.requested.lock().unwrap().extend_from_slice(actor_uids);
            if self.empty {
                return Ok(ActorDirectory::new(vec![], vec![], vec![]));
            }

            let actors = vec![
                Actor::new(
                    ActorUid::new("PA1".into()).unwrap(),
                    None,
                    "Jean".into(),
                    "Dupont".into(),
                    ActorRole::Deputy,
                )
                .unwrap(),
                Actor::new(
                    ActorUid::new("PA2".into()).unwrap(),
                    None,
                    "Marie".into(),
                    "Martin".into(),
                    ActorRole::Deputy,
                )
                .unwrap(),
            ];
            let groups = vec![ParliamentaryGroup::new(
                GroupUid::new("PO_A".into()).unwrap(),
                17,
                "Groupe A".into(),
                "A".into(),
                None,
                None,
                None,
            )
            .unwrap()];
            // PA1 appartient au groupe A a la date du scrutin; PA2 n'a aucune
            // appartenance couvrant cette date.
            let memberships = vec![
                GroupMembership::new(
                    "PM1".into(),
                    ActorUid::new("PA1".into()).unwrap(),
                    GroupUid::new("PO_A".into()).unwrap(),
                    17,
                    MembershipPeriod::new(NaiveDate::from_ymd_opt(2024, 7, 19).unwrap(), None)
                        .unwrap(),
                    MembershipQuality::new("Membre".into()).unwrap(),
                ),
                GroupMembership::new(
                    "PM2".into(),
                    ActorUid::new("PA2".into()).unwrap(),
                    GroupUid::new("PO_A".into()).unwrap(),
                    17,
                    MembershipPeriod::new(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(), None)
                        .unwrap(),
                    MembershipQuality::new("Membre".into()).unwrap(),
                ),
            ];
            Ok(ActorDirectory::new(actors, groups, memberships))
        }
    }

    async fn run(
        scrutins: Vec<Scrutin>,
        registry_empty: bool,
    ) -> (ScrutinsSummary, Vec<Scrutin>, Vec<ActorUid>) {
        let source = FakeScrutinSource {
            scrutins: Mutex::new(Some(scrutins)),
        };
        let repository = InMemoryScrutinRepository {
            stored: Mutex::new(vec![]),
        };
        let actors = StubActorRepository {
            empty: registry_empty,
            requested: Mutex::new(vec![]),
        };

        let summary = RefreshScrutins::new(&source, &repository, &actors)
            .execute()
            .await
            .unwrap();

        let stored = repository.stored.lock().unwrap().clone();
        let requested = actors.requested.lock().unwrap().clone();
        (summary, stored, requested)
    }

    #[tokio::test]
    async fn stores_every_scrutin_including_those_without_a_dossier() {
        let (summary, stored, requested) = run(
            vec![scrutin("V1", vec![], vec![]), scrutin("V2", vec![], vec![])],
            false,
        )
        .await;

        assert_eq!(summary.scrutins, 2);
        assert_eq!(summary.without_dossier, 2);
        assert_eq!(stored.len(), 2);
        // Aucun scrutin sous sentinelle: le referentiel n'est pas sollicite.
        assert!(requested.is_empty());
    }

    #[tokio::test]
    async fn rebuilds_the_breakdown_of_a_scrutin_whose_groups_are_unnamed() {
        let (summary, stored, _) = run(
            vec![scrutin(
                "V489",
                vec![sentinel_line()],
                vec![
                    vote("PA1", VotePosition::For),
                    vote("PA1", VotePosition::Against),
                ],
            )],
            false,
        )
        .await;

        assert_eq!(summary.reconstructed_scrutins, 1);
        assert_eq!(summary.unresolved_votes, 0);

        let tallies = stored[0].group_tallies();
        assert_eq!(tallies.len(), 1);
        assert_eq!(tallies[0].group_uid.as_str(), "PO_A");
        assert_eq!(tallies[0].origin, TallyOrigin::Reconstructed);
        assert_eq!(tallies[0].tally.votes_for, 1);
        assert_eq!(tallies[0].tally.votes_against, 1);
        assert!(stored[0].has_reconstructed_tallies());
    }

    #[tokio::test]
    async fn counts_a_vote_whose_group_is_unknown_at_that_date_instead_of_guessing() {
        let (summary, stored, _) = run(
            vec![scrutin(
                "V489",
                vec![sentinel_line()],
                vec![
                    vote("PA1", VotePosition::For),
                    // PA2 n'a pas d'appartenance a cette date.
                    vote("PA2", VotePosition::For),
                ],
            )],
            false,
        )
        .await;

        assert_eq!(summary.unresolved_votes, 1);
        let tallies = stored[0].group_tallies();
        assert_eq!(tallies.len(), 1);
        assert_eq!(tallies[0].tally.votes_for, 1);
    }

    #[tokio::test]
    async fn keeps_the_source_breakdown_and_signals_when_the_registry_is_empty() {
        let (summary, stored, _) = run(
            vec![scrutin(
                "V489",
                vec![sentinel_line()],
                vec![vote("PA1", VotePosition::For)],
            )],
            true,
        )
        .await;

        assert!(summary.registry_anomaly.is_some());
        assert_eq!(summary.reconstructed_scrutins, 0);
        // Rien n'est invente: la ligne factice reste telle quelle.
        assert!(stored[0].has_sentinel_tallies());
        assert!(!stored[0].has_reconstructed_tallies());
    }

    #[tokio::test]
    async fn leaves_the_stored_scrutins_untouched_when_the_source_is_down() {
        let source = FakeScrutinSource {
            scrutins: Mutex::new(None),
        };
        let repository = InMemoryScrutinRepository {
            stored: Mutex::new(vec![]),
        };
        let actors = StubActorRepository {
            empty: false,
            requested: Mutex::new(vec![]),
        };

        let result = RefreshScrutins::new(&source, &repository, &actors)
            .execute()
            .await;

        assert!(matches!(result, Err(RefreshScrutinsError::Source(_))));
        assert!(repository.stored.lock().unwrap().is_empty());
    }
}
