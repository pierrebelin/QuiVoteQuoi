use std::collections::HashMap;

use crate::application::ports::actor_repository::ActorRepository;
use crate::application::ports::amendment_citation_repository::AmendmentCitationRepository;
use crate::application::use_cases::browse_dossier_amendments::{
    author_uids, cited_view_of, CitedAmendmentView,
};
use crate::domain::amendment_citation::UnlinkedCause;
use crate::application::ports::scrutin_repository::{RepositoryError, ScrutinRepository};
use crate::domain::actor::{ActorDirectory, ActorUid};
use crate::application::ports::vote_explanation_repository::VoteExplanationRepository;
use crate::domain::amendment::SummaryPreview;
use crate::domain::final_vote::is_final_vote_subject;
use crate::domain::sitting_record::SittingRef;
use crate::domain::vote_explanation::ExplanationsState;
use crate::domain::scrutin::{
    GroupTally, NonVotingCause, Scrutin, ScrutinUid, SubjectMentions, TallyOrigin, VotePosition,
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

/// Un numero cite par l'objet: l'amendement trouve, ou la cause de l'absence de lien.
#[derive(Debug, Clone)]
pub struct CitedNumberView {
    pub rank: u16,
    pub cited_number: Option<String>,
    pub amendment: Option<CitedAmendmentView>,
    pub cause: Option<UnlinkedCause>,
}

/// Une explication de vote avec le vote réel de l'orateur et de son groupe (RM-07).
#[derive(Debug, Clone)]
pub struct ExplanationView {
    pub rank: u16,
    pub speaker_uid: String,
    pub speaker_name: Option<String>,
    /// `None`: groupe non établi.
    pub group_uid: Option<String>,
    /// Provenance du groupe: `scrutin`, `membership` ou `undetermined` (RM-06).
    pub group_basis: &'static str,
    /// `None`: position non publiée, jamais déduite.
    pub speaker_position: Option<VotePosition>,
    pub group_tally: Option<crate::domain::scrutin::VoteTally>,
    pub preview: SummaryPreview,
    pub text: String,
    pub url: String,
}

/// Groupe du scrutin auquel aucun orateur n'est rattaché (RM-09).
#[derive(Debug, Clone)]
pub struct GroupWithoutSpeakerView {
    pub group_uid: String,
    pub abbrev: Option<String>,
    pub label: Option<String>,
}

/// Section d'explications d'un vote sur l'ensemble (RM-09).
#[derive(Debug, Clone)]
pub struct ExplanationsView {
    pub state: ExplanationsState,
    pub items: Vec<ExplanationView>,
    pub groups_without_speaker: Vec<GroupWithoutSpeakerView>,
    pub source_url: Option<String>,
    pub report_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScrutinDetail {
    pub scrutin: Scrutin,
    pub groups: Vec<GroupBreakdown>,
    pub corrections: Vec<CorrectionView>,
    /// Acteurs absents du referentiel: affiches par identifiant.
    pub unknown_actors: usize,
    pub cited: Vec<CitedNumberView>,
    pub unnumbered_identical_amendments: bool,
    /// `None`: le scrutin n'est pas un vote sur l'ensemble.
    pub explanations: Option<ExplanationsView>,
}

/// CU-03 — Consulter un scrutin.
pub struct GetScrutinDetail<'a> {
    repository: &'a dyn ScrutinRepository,
    citations: &'a dyn AmendmentCitationRepository,
    actor_repository: &'a dyn ActorRepository,
    explanations: &'a dyn VoteExplanationRepository,
}

impl<'a> GetScrutinDetail<'a> {
    pub fn new(
        repository: &'a dyn ScrutinRepository,
        citations: &'a dyn AmendmentCitationRepository,
        actor_repository: &'a dyn ActorRepository,
        explanations: &'a dyn VoteExplanationRepository,
    ) -> Self {
        Self {
            repository,
            citations,
            actor_repository,
            explanations,
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

        // Citations stockees, dans l'ordre de l'objet; leurs auteurs se
        // nomment par le meme chargement du referentiel.
        let mut rows = self
            .citations
            .citations_of(std::slice::from_ref(scrutin.uid()))
            .await?;
        rows.sort_by_key(|row| row.rank);
        actor_uids.extend(author_uids(
            rows.iter()
                .filter_map(|row| row.amendment.as_ref())
                .map(|cited| &cited.summary),
        ));
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

        let cited = rows
            .into_iter()
            .map(|row| {
                let full_summary = row
                    .amendment
                    .as_ref()
                    .and_then(|cited| cited.summary.summary.clone());
                let (rank, cited_number, cause) = (row.rank, row.cited_number.clone(), row.cause);
                let amendment = cited_view_of(row, &directory).map(|mut view| {
                    // Le detail sert l'expose entier, jamais l'apercu (RM-06).
                    view.summary = full_summary;
                    view
                });
                CitedNumberView {
                    rank,
                    cited_number,
                    amendment,
                    cause,
                }
            })
            .collect();
        let unnumbered_identical_amendments =
            SubjectMentions::unnumbered_identical_amendments(scrutin.subject());

        let explanations = self.explanations_view(&scrutin, &directory).await?;

        Ok(Some(ScrutinDetail {
            scrutin,
            groups,
            corrections,
            unknown_actors,
            cited,
            unnumbered_identical_amendments,
            explanations,
        }))
    }

    /// Section d'explications d'un vote sur l'ensemble; une seule lecture, et
    /// aucune quand la séance du scrutin n'est pas connue (RM-09).
    async fn explanations_view(
        &self,
        scrutin: &Scrutin,
        directory: &ActorDirectory,
    ) -> Result<Option<ExplanationsView>, RepositoryError> {
        if !is_final_vote_subject(scrutin.subject()) {
            return Ok(None);
        }
        let sitting_ref = scrutin
            .sitting_ref()
            .and_then(|raw| SittingRef::new(raw.to_string()).ok());
        let Some(sitting_ref) = sitting_ref else {
            return Ok(Some(ExplanationsView {
                state: ExplanationsState::NotAttached,
                items: Vec::new(),
                groups_without_speaker: Vec::new(),
                source_url: None,
                report_version: None,
            }));
        };

        let read = self
            .explanations
            .explanations_for(scrutin.uid(), &sitting_ref)
            .await?;
        let state = ExplanationsState::decide(
            read.report.is_some(),
            read.attached,
            !read.explanations.is_empty(),
        );
        let mut explanations = read.explanations;
        explanations.sort_by_key(|explanation| explanation.rank());

        let items: Vec<ExplanationView> = explanations
            .iter()
            .map(|explanation| {
                let group_uid = explanation.group().group_uid();
                let report_uid = explanation.sitting_record_uid();
                ExplanationView {
                    rank: explanation.rank(),
                    speaker_uid: explanation.speaker_uid().as_str().to_string(),
                    speaker_name: explanation.speaker_name().map(str::to_string),
                    group_uid: group_uid.map(|group| group.as_str().to_string()),
                    group_basis: explanation.group().as_str(),
                    speaker_position: scrutin
                        .nominal_votes()
                        .iter()
                        .find(|vote| &vote.actor_uid == explanation.speaker_uid())
                        .map(|vote| vote.position),
                    group_tally: group_uid.and_then(|group| {
                        scrutin
                            .group_tallies()
                            .iter()
                            .find(|tally| &tally.group_uid == group)
                            .map(|tally| tally.tally)
                    }),
                    preview: SummaryPreview::of(Some(explanation.text())),
                    text: explanation.text().to_string(),
                    url: report_uid.official_url(Some(explanation.anchor())),
                }
            })
            .collect();

        let groups_without_speaker = if state == ExplanationsState::Explained {
            scrutin
                .group_tallies()
                .iter()
                .filter(|tally| {
                    !items
                        .iter()
                        .any(|item| item.group_uid.as_deref() == Some(tally.group_uid.as_str()))
                })
                .map(|tally| {
                    let group = directory.group(&tally.group_uid);
                    GroupWithoutSpeakerView {
                        group_uid: tally.group_uid.as_str().to_string(),
                        abbrev: group.map(|g| g.abbrev().to_string()),
                        label: group.map(|g| g.label().to_string()),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(Some(ExplanationsView {
            state,
            items,
            groups_without_speaker,
            source_url: read
                .report
                .as_ref()
                .filter(|_| state == ExplanationsState::NotAttached)
                .map(|report| report.record_uid.official_url(None)),
            report_version: read.report.map(|report| report.version),
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
    use crate::application::ports::amendment_citation_repository::{CitationRow, CitedAmendmentRow};
    use crate::application::ports::vote_explanation_repository::StoredSitting;
    use crate::application::use_cases::citation_fakes::InMemoryCitationRepository;
    use crate::application::use_cases::explanation_fakes::InMemoryVoteExplanations;
    use crate::domain::sitting_record::{
        SittingKeysFingerprint, SittingRecordUid, SittingRef, SpeechAnchor,
    };
    use crate::domain::vote_explanation::{NewVoteExplanation, SpeakerGroup, VoteExplanation};
    use crate::application::use_cases::read_fakes::amendment_row;
    use crate::domain::amendment_citation::LinkMethod;
    use crate::domain::dossier::DossierUid;
    use crate::application::ports::scrutin_repository::{
        ScrutinFilter, ScrutinPage, ScrutinSummary,
    };
    use crate::domain::actor::{Actor, ActorRegistry, ActorRole, GroupUid, ParliamentaryGroup};
    use crate::domain::dataset::DatasetShape;
    use crate::domain::scrutin::{
        BallotType, NewScrutin, NominalVote, Outcome, VoteCorrection, VoteSynthesis, VoteTally,
    };

    fn scrutin_about(subject: &str, sitting_ref: Option<&str>) -> Scrutin {
        Scrutin::new(NewScrutin {
            uid: ScrutinUid::new("V1".into()).unwrap(),
            number: "1".into(),
            legislature: 17,
            date: NaiveDate::from_ymd_opt(2025, 3, 27).unwrap(),
            session_ref: None,
            sitting_ref: sitting_ref.map(str::to_string),
            place: None,
            ballot_type: BallotType::new("SPO".into(), "ordinaire".into(), None).unwrap(),
            outcome: Outcome::new("adopt\u{00e9}".into(), "adopt\u{00e9}".into()).unwrap(),
            requester: None,
            subject: subject.into(),
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

    struct StubScrutinRepository {
        subject: &'static str,
        sitting_ref: Option<&'static str>,
    }

    #[async_trait]
    impl ScrutinRepository for StubScrutinRepository {
        async fn save_scrutins(&self, _s: &[Scrutin]) -> Result<usize, RepositoryError> {
            unreachable!()
        }
        async fn list(&self, _f: &ScrutinFilter) -> Result<ScrutinPage, RepositoryError> {
            unreachable!()
        }
        async fn by_uid(&self, uid: &ScrutinUid) -> Result<Option<Scrutin>, RepositoryError> {
            Ok((uid.as_str() == "V1").then(|| scrutin_about(self.subject, self.sitting_ref)))
        }
        async fn by_dossier(&self, _u: &str) -> Result<Vec<ScrutinSummary>, RepositoryError> {
            unreachable!()
        }

        async fn sitting_keys(
            &self,
        ) -> Result<Vec<crate::domain::sitting_record::ScrutinSittingKey>, RepositoryError> {
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
        detail_of("objet", vec![]).await
    }

    async fn detail_of(subject: &'static str, rows: Vec<CitationRow>) -> ScrutinDetail {
        let repository = StubScrutinRepository {
            subject,
            sitting_ref: None,
        };
        let citations = InMemoryCitationRepository {
            rows: Mutex::new(rows),
            ..Default::default()
        };
        let actors = StubActorRepository {
            requested: Mutex::new(vec![]),
        };
        let explanations = InMemoryVoteExplanations::default();
        GetScrutinDetail::new(&repository, &citations, &actors, &explanations)
            .execute(&ScrutinUid::new("V1".into()).unwrap())
            .await
            .unwrap()
            .unwrap()
    }

    const FINAL_SUBJECT: &str = "l'ensemble du projet de loi";
    const SITTING_REF: &str = "RUANR5L17S2025IDS1";
    const REPORT_UID: &str = "CRSANR5L17S2025O1N006";

    /// Fiche du scrutin `V1` lue avec ce dépôt d'explications.
    async fn detail_explained(
        subject: &'static str,
        sitting_ref: Option<&'static str>,
        explanations: &InMemoryVoteExplanations,
    ) -> ScrutinDetail {
        let repository = StubScrutinRepository {
            subject,
            sitting_ref,
        };
        let citations = InMemoryCitationRepository::default();
        let actors = StubActorRepository {
            requested: Mutex::new(vec![]),
        };
        GetScrutinDetail::new(&repository, &citations, &actors, explanations)
            .execute(&ScrutinUid::new("V1".into()).unwrap())
            .await
            .unwrap()
            .unwrap()
    }

    fn explanation(rank: u16, speaker: &str, group: SpeakerGroup, text: &str) -> VoteExplanation {
        VoteExplanation::new(NewVoteExplanation {
            scrutin_uid: ScrutinUid::new("V1".into()).unwrap(),
            sitting_record_uid: SittingRecordUid::new(REPORT_UID.into()).unwrap(),
            rank,
            speaker_uid: ActorUid::new(speaker.into()).unwrap(),
            speaker_name: None,
            group,
            anchor: SpeechAnchor::new("3518476".into()).unwrap(),
            heading: "Explications de vote".into(),
            text: text.into(),
        })
    }

    fn sitting(
        version: &str,
        attached: Vec<(ScrutinUid, bool)>,
        explanations: Vec<VoteExplanation>,
    ) -> StoredSitting {
        StoredSitting {
            record_uid: SittingRecordUid::new(REPORT_UID.into()).unwrap(),
            sitting_ref: SittingRef::new(SITTING_REF.into()).unwrap(),
            date: NaiveDate::from_ymd_opt(2025, 3, 27).unwrap(),
            version: version.into(),
            generated_at: NaiveDate::from_ymd_opt(2025, 3, 28)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            keys: SittingKeysFingerprint::from_stored(String::new()),
            counters: Default::default(),
            block_counters: Default::default(),
            attached,
            explanations,
        }
    }

    fn v1() -> ScrutinUid {
        ScrutinUid::new("V1".into()).unwrap()
    }

    fn group_a() -> SpeakerGroup {
        SpeakerGroup::FromScrutin(GroupUid::new("PO_A".into()).unwrap())
    }

    /// Fiche d'un vote sur l'ensemble dont la séance porte ces explications.
    async fn explained(version: &str, explanations: Vec<VoteExplanation>) -> ExplanationsView {
        let repository = InMemoryVoteExplanations::with_sittings(vec![sitting(
            version,
            vec![(v1(), true)],
            explanations,
        )]);
        detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &repository)
            .await
            .explanations
            .unwrap()
    }

    #[tokio::test]
    async fn lists_the_explanations_of_a_final_vote_in_report_order_with_speaker_position_and_group_tally(
    ) {
        let view = explained(
            "avant_JO",
            vec![
                explanation(1, "PA2", SpeakerGroup::Undetermined, "Seconde."),
                explanation(0, "PA1", group_a(), "Première."),
            ],
        )
        .await;

        assert_eq!(view.state, ExplanationsState::Explained);
        let speakers: Vec<&str> = view.items.iter().map(|i| i.speaker_uid.as_str()).collect();
        assert_eq!(speakers, vec!["PA1", "PA2"]);
        let first = &view.items[0];
        assert_eq!(first.rank, 0);
        assert_eq!(first.group_uid.as_deref(), Some("PO_A"));
        assert_eq!(first.speaker_position, Some(VotePosition::For));
        assert_eq!(first.group_tally.unwrap().votes_for, 2);
        let second = &view.items[1];
        assert_eq!(second.group_uid, None);
        assert_eq!(second.speaker_position, Some(VotePosition::Against));
        assert_eq!(second.group_tally, None);
    }

    #[tokio::test]
    async fn previews_the_first_characters_of_an_explanation_and_keeps_its_full_text() {
        let text: String = "a".repeat(400);
        let view = explained("avant_JO", vec![explanation(0, "PA1", group_a(), &text)]).await;

        let item = &view.items[0];
        assert_eq!(item.text, text);
        assert_eq!(item.preview, SummaryPreview::of(Some(&text)));
        assert_eq!(
            item.preview,
            SummaryPreview::Text {
                text: format!("{}\u{2026}", "a".repeat(300)),
                truncated: true,
            }
        );
    }

    #[tokio::test]
    async fn links_each_explanation_to_its_paragraph_in_the_official_report() {
        let view = explained("avant_JO", vec![explanation(0, "PA1", group_a(), "Texte.")]).await;

        assert_eq!(
            view.items[0].url,
            "https://www.assemblee-nationale.fr/dyn/17/comptes-rendus/seance/CRSANR5L17S2025O1N006#3518476"
        );
    }

    #[tokio::test]
    async fn marks_a_speaker_absent_from_the_nominal_votes_as_position_not_published() {
        let view = explained(
            "avant_JO",
            vec![explanation(0, "PA_GHOST", SpeakerGroup::Undetermined, "Texte.")],
        )
        .await;

        assert_eq!(view.items[0].speaker_position, None);
    }

    #[tokio::test]
    async fn lists_after_the_explanations_the_groups_with_no_attached_speaker() {
        let view = explained(
            "avant_JO",
            vec![explanation(0, "PA2", SpeakerGroup::Undetermined, "Texte.")],
        )
        .await;

        assert_eq!(view.groups_without_speaker.len(), 1);
        let group = &view.groups_without_speaker[0];
        assert_eq!(group.group_uid, "PO_A");
        assert_eq!(group.abbrev.as_deref(), Some("A"));
        assert_eq!(group.label.as_deref(), Some("Groupe A"));
    }

    #[tokio::test]
    async fn says_the_report_of_the_sitting_is_not_yet_published() {
        let repository = InMemoryVoteExplanations::default();
        let detail = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &repository).await;

        let view = detail.explanations.unwrap();
        assert_eq!(view.state, ExplanationsState::NotPublished);
        assert!(view.items.is_empty());
        assert_eq!(view.source_url, None);
        assert_eq!(view.report_version, None);
    }

    #[tokio::test]
    async fn says_the_explanations_are_not_attached_when_the_report_cannot_identify_the_scrutin() {
        let repository =
            InMemoryVoteExplanations::with_sittings(vec![sitting("avant_JO", vec![], vec![])]);
        let detail = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &repository).await;

        let view = detail.explanations.unwrap();
        assert_eq!(view.state, ExplanationsState::NotAttached);
        assert!(view.items.is_empty());
        assert_eq!(
            view.source_url.as_deref(),
            Some("https://www.assemblee-nationale.fr/dyn/17/comptes-rendus/seance/CRSANR5L17S2025O1N006")
        );
    }

    #[tokio::test]
    async fn says_the_report_holds_no_explanation_for_the_sitting() {
        let repository = InMemoryVoteExplanations::with_sittings(vec![sitting(
            "avant_JO",
            vec![(v1(), false)],
            vec![],
        )]);
        let detail = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &repository).await;

        let view = detail.explanations.unwrap();
        assert_eq!(view.state, ExplanationsState::NoExplanation);
        assert!(view.items.is_empty());
    }

    #[tokio::test]
    async fn says_the_report_holds_no_explanation_when_the_block_has_no_speaker() {
        let repository = InMemoryVoteExplanations::with_sittings(vec![sitting(
            "avant_JO",
            vec![(v1(), true)],
            vec![],
        )]);
        let detail = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &repository).await;

        let view = detail.explanations.unwrap();
        assert_eq!(view.state, ExplanationsState::NoExplanation);
        assert!(view.items.is_empty());
    }

    #[tokio::test]
    async fn says_the_explanations_are_not_attached_when_the_scrutin_has_no_sitting_reference() {
        let repository = InMemoryVoteExplanations::default();
        let detail = detail_explained(FINAL_SUBJECT, None, &repository).await;

        let view = detail.explanations.unwrap();
        assert_eq!(view.state, ExplanationsState::NotAttached);
        assert_eq!(view.source_url, None);
        assert_eq!(repository.reads(), 0);
    }

    #[tokio::test]
    async fn lists_no_group_without_speaker_outside_the_explained_state() {
        let absent = InMemoryVoteExplanations::default();
        let not_published = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &absent).await;
        let empty = InMemoryVoteExplanations::with_sittings(vec![sitting(
            "avant_JO",
            vec![(v1(), false)],
            vec![],
        )]);
        let none_said = detail_explained(FINAL_SUBJECT, Some(SITTING_REF), &empty).await;

        assert_eq!(
            not_published.explanations.unwrap().state,
            ExplanationsState::NotPublished
        );
        let none_said = none_said.explanations.unwrap();
        assert_eq!(none_said.state, ExplanationsState::NoExplanation);
        assert!(none_said.groups_without_speaker.is_empty());
    }

    #[tokio::test]
    async fn shows_the_raw_version_label_of_the_report() {
        let view = explained(
            "version_brute_de_la_source",
            vec![explanation(0, "PA1", group_a(), "Texte.")],
        )
        .await;

        assert_eq!(
            view.report_version.as_deref(),
            Some("version_brute_de_la_source")
        );
    }

    #[tokio::test]
    async fn returns_no_explanation_section_for_a_scrutin_that_is_not_a_final_vote() {
        let repository = InMemoryVoteExplanations::with_sittings(vec![sitting(
            "avant_JO",
            vec![(v1(), true)],
            vec![explanation(0, "PA1", group_a(), "Texte.")],
        )]);
        let detail = detail_explained(
            "l'amendement n\u{00b0} 12 \u{00e0} l'article 3",
            Some(SITTING_REF),
            &repository,
        )
        .await;

        assert!(detail.explanations.is_none());
        assert_eq!(repository.reads(), 0);
    }

    fn linked_row(rank: u16, number: &str, by_cited_article: bool) -> CitationRow {
        let mut summary = amendment_row();
        summary.uid = format!("AM{number}");
        summary.number = number.into();
        summary.summary = Some("Expos\u{00e9} complet, sans aucune coupure.".into());
        CitationRow {
            scrutin: ScrutinUid::new("V1".into()).unwrap(),
            rank,
            cited_number: Some(number.into()),
            amendment: Some(CitedAmendmentRow {
                dossier_uid: DossierUid::new("DLR5L17N1".into()).unwrap(),
                summary,
            }),
            cause: None,
            method: LinkMethod { by_cited_article },
        }
    }

    fn unlinked_row(rank: u16, number: Option<&str>, cause: UnlinkedCause) -> CitationRow {
        CitationRow {
            scrutin: ScrutinUid::new("V1".into()).unwrap(),
            rank,
            cited_number: number.map(str::to_string),
            amendment: None,
            cause: Some(cause),
            method: LinkMethod {
                by_cited_article: false,
            },
        }
    }

    #[tokio::test]
    async fn should_tell_what_was_put_to_the_vote_with_whole_summary_and_method_when_scrutin_is_linked(
    ) {
        let detail = detail_of(
            "l'amendement n\u{00b0} 12 \u{00e0} l'article 3",
            vec![linked_row(0, "12", true)],
        )
        .await;

        assert_eq!(detail.cited.len(), 1);
        let cited = &detail.cited[0];
        assert_eq!(cited.cause, None);
        let amendment = cited.amendment.as_ref().unwrap();
        assert_eq!(amendment.dossier_uid, "DLR5L17N1");
        assert_eq!(amendment.number, "12");
        assert_eq!(
            amendment.summary.as_deref(),
            Some("Expos\u{00e9} complet, sans aucune coupure.")
        );
        assert_eq!(amendment.author.name.as_deref(), Some("Jean Dupont"));
        assert_eq!(
            amendment.method.unwrap().labels(),
            vec!["Num\u{00e9}ro d'amendement cit\u{00e9}", "Article cit\u{00e9}"]
        );
    }

    #[tokio::test]
    async fn should_give_each_unlinked_number_its_cause_when_subject_cites_several() {
        let detail = detail_of(
            "les amendements n\u{00b0} 7, 99 et 100",
            vec![
                linked_row(0, "7", false),
                unlinked_row(1, Some("99"), UnlinkedCause::Unresolved),
                unlinked_row(2, Some("100"), UnlinkedCause::Ambiguous),
            ],
        )
        .await;

        let causes: Vec<Option<UnlinkedCause>> = detail.cited.iter().map(|c| c.cause).collect();
        assert_eq!(
            causes,
            vec![
                None,
                Some(UnlinkedCause::Unresolved),
                Some(UnlinkedCause::Ambiguous)
            ]
        );
        assert!(detail.cited[0].amendment.is_some());
        assert!(detail.cited[1].amendment.is_none());
        assert_eq!(detail.cited[1].cited_number.as_deref(), Some("99"));
        assert_eq!(detail.cited[2].cited_number.as_deref(), Some("100"));
    }

    #[tokio::test]
    async fn should_give_no_cited_amendment_when_subject_names_no_amendment() {
        let detail = detail_of("l'ensemble du projet de loi", vec![]).await;

        assert!(detail.cited.is_empty());
        assert!(!detail.unnumbered_identical_amendments);
    }

    #[tokio::test]
    async fn should_say_identical_amendments_are_unnumbered_when_subject_announces_them_without_numbers(
    ) {
        let detail = detail_of(
            "les amendements identiques de suppression de l'article 5",
            vec![unlinked_row(0, None, UnlinkedCause::NoNumber)],
        )
        .await;

        assert!(detail.unnumbered_identical_amendments);
        assert_eq!(detail.cited.len(), 1);
        assert_eq!(detail.cited[0].cause, Some(UnlinkedCause::NoNumber));
        assert_eq!(detail.cited[0].cited_number, None);
    }

    #[tokio::test]
    async fn returns_nothing_for_an_unknown_scrutin() {
        let repository = StubScrutinRepository {
            subject: "objet",
            sitting_ref: None,
        };
        let citations = InMemoryCitationRepository::default();
        let actors = StubActorRepository {
            requested: Mutex::new(vec![]),
        };
        let explanations = InMemoryVoteExplanations::default();
        let result = GetScrutinDetail::new(&repository, &citations, &actors, &explanations)
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
