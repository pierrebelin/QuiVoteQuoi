use std::collections::HashMap;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::scrutin_repository::{RepositoryError, ScrutinRepository};
use crate::domain::actor::{ActorDirectory, ActorUid};
use crate::domain::scrutin::{
    GroupTally, NonVotingCause, Scrutin, ScrutinUid, TallyOrigin, VotePosition,
};

/// Position nominale prete a l'affichage.
#[derive(Debug, Clone)]
pub struct VoteView {
    pub actor_uid: String,
    /// Absent quand l'acteur ne figure pas au referentiel: l'identifiant brut
    /// est conserve, aucun nom n'est devine (ACTEURS RM-04).
    pub full_name: Option<String>,
    pub official_url: Option<String>,
    pub position: VotePosition,
    pub cause: Option<NonVotingCause>,
    pub by_delegation: bool,
    pub seat: Option<u16>,
}

/// Une ligne de la ventilation, avec ses votants.
#[derive(Debug, Clone)]
pub struct GroupBreakdown {
    /// `None` pour le bloc des positions qu'aucun groupe ne porte.
    pub group_uid: Option<String>,
    pub abbrev: Option<String>,
    pub label: Option<String>,
    pub color: Option<String>,
    pub member_count: Option<u16>,
    pub majority_position: Option<VotePosition>,
    pub tally: crate::domain::scrutin::VoteTally,
    pub origin: TallyOrigin,
    pub votes: Vec<VoteView>,
}

#[derive(Debug, Clone)]
pub struct CorrectionView {
    pub actor_uid: String,
    pub full_name: Option<String>,
    pub claimed_position: VotePosition,
    pub malfunction: bool,
}

#[derive(Debug, Clone)]
pub struct ScrutinDetail {
    pub scrutin: Scrutin,
    pub groups: Vec<GroupBreakdown>,
    pub corrections: Vec<CorrectionView>,
    /// Acteurs absents du referentiel: affiches par identifiant.
    pub unknown_actors: usize,
}

/// CU-03 — Consulter un scrutin.
pub struct GetScrutinDetail<'a> {
    repository: &'a dyn ScrutinRepository,
    actor_repository: &'a dyn ActorRepository,
}

impl<'a> GetScrutinDetail<'a> {
    pub fn new(
        repository: &'a dyn ScrutinRepository,
        actor_repository: &'a dyn ActorRepository,
    ) -> Self {
        Self {
            repository,
            actor_repository,
        }
    }

    pub async fn execute(
        &self,
        uid: &ScrutinUid,
    ) -> Result<Option<ScrutinDetail>, RepositoryError> {
        let Some(scrutin) = self.repository.by_uid(uid).await? else {
            return Ok(None);
        };

        let mut actor_uids: Vec<ActorUid> = scrutin
            .nominal_votes()
            .iter()
            .map(|v| v.actor_uid.clone())
            .chain(scrutin.corrections().iter().map(|c| c.actor_uid.clone()))
            .collect();
        actor_uids.sort();
        actor_uids.dedup();

        let directory = self
            .actor_repository
            .load_directory_for(&actor_uids)
            .await?;

        let mut unknown_actors = 0usize;
        let mut votes_by_group: HashMap<Option<String>, Vec<VoteView>> = HashMap::new();
        for vote in scrutin.nominal_votes() {
            let view = view_of(&directory, &vote.actor_uid, &mut unknown_actors);
            votes_by_group
                .entry(vote.group_uid.as_ref().map(|g| g.as_str().to_string()))
                .or_default()
                .push(VoteView {
                    actor_uid: view.0,
                    full_name: view.1,
                    official_url: view.2,
                    position: vote.position,
                    cause: vote.cause.clone(),
                    by_delegation: vote.by_delegation,
                    seat: vote.seat,
                });
        }

        let mut groups: Vec<GroupBreakdown> = scrutin
            .group_tallies()
            .iter()
            .map(|tally| {
                let key = Some(tally.group_uid.as_str().to_string());
                let votes = votes_by_group.remove(&key).unwrap_or_default();
                breakdown(&directory, tally, votes)
            })
            .collect();

        // Positions qu'aucune ligne de groupe ne porte: affichees a part plutot
        // que rattachees d'office (README.md §2).
        let mut orphans: Vec<(Option<String>, Vec<VoteView>)> =
            votes_by_group.into_iter().collect();
        orphans.sort_by(|a, b| a.0.cmp(&b.0));
        for (group_uid, votes) in orphans {
            let mut tally = crate::domain::scrutin::VoteTally::default();
            for vote in &votes {
                match vote.position {
                    VotePosition::For => tally.votes_for += 1,
                    VotePosition::Against => tally.votes_against += 1,
                    VotePosition::Abstention => tally.abstentions += 1,
                    VotePosition::NotVoting => tally.not_voting += 1,
                }
            }
            groups.push(GroupBreakdown {
                group_uid,
                abbrev: None,
                label: None,
                color: None,
                member_count: None,
                majority_position: None,
                tally,
                origin: TallyOrigin::Reconstructed,
                votes,
            });
        }

        let corrections = scrutin
            .corrections()
            .iter()
            .map(|c| {
                let view = view_of(&directory, &c.actor_uid, &mut unknown_actors);
                CorrectionView {
                    actor_uid: view.0,
                    full_name: view.1,
                    claimed_position: c.claimed_position,
                    malfunction: c.malfunction,
                }
            })
            .collect();

        Ok(Some(ScrutinDetail {
            scrutin,
            groups,
            corrections,
            unknown_actors,
        }))
    }
}

fn view_of(
    directory: &ActorDirectory,
    uid: &ActorUid,
    unknown: &mut usize,
) -> (String, Option<String>, Option<String>) {
    match directory.actor(uid) {
        Some(actor) => (
            uid.as_str().to_string(),
            Some(actor.full_name()),
            actor.official_url(),
        ),
        None => {
            *unknown += 1;
            (uid.as_str().to_string(), None, None)
        }
    }
}

fn breakdown(
    directory: &ActorDirectory,
    tally: &GroupTally,
    votes: Vec<VoteView>,
) -> GroupBreakdown {
    // RM-06 de la spec ACTEURS: le libelle officiel, jamais un parti. Groupe
    // absent du referentiel: identifiant brut, aucun libelle invente.
    let group = directory.group(&tally.group_uid);
    GroupBreakdown {
        group_uid: Some(tally.group_uid.as_str().to_string()),
        abbrev: group.map(|g| g.abbrev().to_string()),
        label: group.map(|g| g.label().to_string()),
        color: group.and_then(|g| g.color().map(str::to_string)),
        member_count: tally.member_count,
        majority_position: tally.majority_position,
        tally: tally.tally,
        origin: tally.origin,
        votes,
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
        ScrutinFilter, ScrutinPage, ScrutinSummary,
    };
    use crate::domain::actor::{Actor, ActorRegistry, ActorRole, GroupUid, ParliamentaryGroup};
    use crate::domain::dataset::DatasetShape;
    use crate::domain::scrutin::{
        BallotType, NewScrutin, NominalVote, Outcome, VoteCorrection, VoteSynthesis, VoteTally,
    };

    fn scrutin() -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("V1".into()).unwrap(),
            number: "1".into(),
            legislature: 17,
            date: NaiveDate::from_ymd_opt(2025, 3, 27).unwrap(),
            session_ref: None,
            sitting_ref: None,
            place: None,
            ballot_type: BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            outcome: Outcome::new("adopt\u{00e9}".into(), "adopt\u{00e9}".into()).unwrap(),
            requester: None,
            subject: "objet".into(),
            synthesis: VoteSynthesis {
                voters: 3,
                expressed: 3,
                required: 2,
                announcement: "annonce".into(),
                tally: VoteTally {
                    votes_for: 2,
                    votes_against: 1,
                    ..VoteTally::default()
                },
            },
            group_tallies: vec![GroupTally {
                group_uid: GroupUid::new("PO_A".into()).unwrap(),
                member_count: Some(10),
                majority_position: Some(VotePosition::For),
                tally: VoteTally {
                    votes_for: 2,
                    ..VoteTally::default()
                },
                origin: TallyOrigin::Published,
            }],
            nominal_votes: vec![
                NominalVote {
                    actor_uid: ActorUid::new("PA1".into()).unwrap(),
                    group_uid: Some(GroupUid::new("PO_A".into()).unwrap()),
                    position: VotePosition::For,
                    cause: None,
                    by_delegation: false,
                    seat: Some(12),
                },
                // Acteur absent du referentiel.
                NominalVote {
                    actor_uid: ActorUid::new("PA_UNKNOWN".into()).unwrap(),
                    group_uid: Some(GroupUid::new("PO_A".into()).unwrap()),
                    position: VotePosition::For,
                    cause: None,
                    by_delegation: false,
                    seat: None,
                },
                // Position qu'aucune ligne de groupe ne porte.
                NominalVote {
                    actor_uid: ActorUid::new("PA2".into()).unwrap(),
                    group_uid: None,
                    position: VotePosition::Against,
                    cause: None,
                    by_delegation: false,
                    seat: None,
                },
            ],
            corrections: vec![VoteCorrection {
                actor_uid: ActorUid::new("PA1".into()).unwrap(),
                claimed_position: VotePosition::Against,
                malfunction: false,
            }],
            dossier: None,
        })
        .unwrap()
    }

    struct StubScrutinRepository;

    #[async_trait]
    impl ScrutinRepository for StubScrutinRepository {
        async fn save_scrutins(&self, _s: &[Scrutin]) -> Result<usize, RepositoryError> {
            unreachable!()
        }
        async fn list(&self, _f: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
            unreachable!()
        }
        async fn by_uid(&self, uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
            Ok((uid.as_str() == "V1").then(scrutin))
        }
        async fn by_dossier(&self, _u: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
            unreachable!()
        }

        async fn dataset_shape(&self) -> Result<DatasetShape, RepositoryError> {
            unreachable!()
        }
    }

    struct StubActorRepository {
        requested: Mutex<Vec<ActorUid>>,
    }

    #[async_trait]
    impl ActorRepository for StubActorRepository {
        async fn save_registry(
            &self,
            _r: &ActorRegistry,
        ) -> Result<RegistrySummary, RepositoryError> {
            unreachable!()
        }

        async fn load_directory_for(
            &self,
            actor_uids: &[ActorUid],
        ) -> Result<ActorDirectory, RepositoryError> {
            self.requested.lock().unwrap().extend_from_slice(actor_uids);
            Ok(ActorDirectory::new(
                vec![
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
                ],
                vec![ParliamentaryGroup::new(
                    GroupUid::new("PO_A".into()).unwrap(),
                    17,
                    "Groupe A".into(),
                    "A".into(),
                    Some("#123456".into()),
                    None,
                    None,
                )
                .unwrap()],
                vec![],
            ))
        }
    }

    async fn detail() -> ScrutinDetail {
        let repository = StubScrutinRepository;
        let actors = StubActorRepository {
            requested: Mutex::new(vec![]),
        };
        GetScrutinDetail::new(&repository, &actors)
            .execute(&ScrutinUid::new("V1".into()).unwrap())
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn returns_nothing_for_an_unknown_scrutin() {
        let repository = StubScrutinRepository;
        let actors = StubActorRepository {
            requested: Mutex::new(vec![]),
        };
        let result = GetScrutinDetail::new(&repository, &actors)
            .execute(&ScrutinUid::new("V999".into()).unwrap())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn names_the_group_with_its_official_label() {
        let detail = detail().await;
        let group = &detail.groups[0];

        assert_eq!(group.group_uid.as_deref(), Some("PO_A"));
        assert_eq!(group.abbrev.as_deref(), Some("A"));
        assert_eq!(group.label.as_deref(), Some("Groupe A"));
        assert_eq!(group.color.as_deref(), Some("#123456"));
        assert_eq!(group.tally.votes_for, 2);
        assert_eq!(group.origin, TallyOrigin::Published);
    }

    #[tokio::test]
    async fn keeps_an_unknown_actor_as_a_raw_identifier() {
        let detail = detail().await;
        let group = &detail.groups[0];

        let known = group.votes.iter().find(|v| v.actor_uid == "PA1").unwrap();
        assert_eq!(known.full_name.as_deref(), Some("Jean Dupont"));
        assert_eq!(
            known.official_url.as_deref(),
            Some("https://www.assemblee-nationale.fr/dyn/deputes/PA1")
        );
        assert_eq!(known.seat, Some(12));

        let unknown = group
            .votes
            .iter()
            .find(|v| v.actor_uid == "PA_UNKNOWN")
            .unwrap();
        assert!(unknown.full_name.is_none());
        assert!(unknown.official_url.is_none());
        assert_eq!(detail.unknown_actors, 1);
    }

    #[tokio::test]
    async fn shows_a_vote_without_a_group_in_its_own_block() {
        let detail = detail().await;

        assert_eq!(detail.groups.len(), 2);
        let orphan = &detail.groups[1];
        assert!(orphan.group_uid.is_none());
        assert!(orphan.label.is_none());
        assert_eq!(orphan.tally.votes_against, 1);
        assert_eq!(orphan.votes[0].actor_uid, "PA2");
    }

    #[tokio::test]
    async fn lists_corrections_apart_from_the_counts() {
        let detail = detail().await;

        assert_eq!(detail.corrections.len(), 1);
        assert_eq!(
            detail.corrections[0].full_name.as_deref(),
            Some("Jean Dupont")
        );
        assert_eq!(
            detail.corrections[0].claimed_position,
            VotePosition::Against
        );
        // RM-05: la synthese et la ventilation restent celles de la source.
        assert_eq!(detail.scrutin.synthesis().tally.votes_for, 2);
        assert_eq!(detail.groups[0].tally.votes_for, 2);
    }
}
