use sha2::{Digest, Sha256};

use crate::application::ports::dossier_group_actions_repository::{
    DossierGroupActionsRepository, DossierSummaryRepository, RepositoryError,
};
use crate::application::ports::vote_explanation_repository::{
    ExplanationMention, VoteExplanationRepository,
};
use crate::domain::dossier_group_facts::DossierGroupFacts;
use crate::domain::dossier_summary::{FactsFingerprint, StoredGroupSummary};
use crate::domain::scrutin::ScrutinUid;

#[derive(Debug, Clone)]
pub struct DossierGroupActions {
    pub facts: DossierGroupFacts,
    pub facts_fingerprint: FactsFingerprint,
    pub summaries: Vec<StoredGroupSummary>,
    pub explanations: Vec<ExplanationMention>,
}

pub struct GetDossierGroupActions<'a> {
    facts: &'a dyn DossierGroupActionsRepository,
    summaries: &'a dyn DossierSummaryRepository,
    explanations: &'a dyn VoteExplanationRepository,
}

impl<'a> GetDossierGroupActions<'a> {
    pub fn new(
        facts: &'a dyn DossierGroupActionsRepository,
        summaries: &'a dyn DossierSummaryRepository,
        explanations: &'a dyn VoteExplanationRepository,
    ) -> Self {
        Self {
            facts,
            summaries,
            explanations,
        }
    }

    pub async fn execute(
        &self,
        dossier_uid: &str,
    ) -> Result<Option<DossierGroupActions>, RepositoryError> {
        let Some(facts) = self.facts.load_facts(dossier_uid).await? else {
            return Ok(None);
        };
        let facts_fingerprint = fingerprint(&facts);
        let summaries = self
            .summaries
            .summaries_for(dossier_uid)
            .await?
            .into_iter()
            .filter(|summary| {
                summary.is_shown_for(&facts_fingerprint)
                    && facts
                        .groups
                        .iter()
                        .any(|group| group.uid == summary.group_uid)
            })
            .collect();

        let mut scrutins: Vec<ScrutinUid> = facts
            .groups
            .iter()
            .flat_map(|group| {
                group
                    .final_votes
                    .iter()
                    .map(|vote| vote.scrutin_uid.clone())
            })
            .collect();
        scrutins.sort();
        scrutins.dedup();
        let mut explanations: Vec<ExplanationMention> = self
            .explanations
            .mentions_for(&scrutins)
            .await?
            .into_iter()
            .filter(|mention| {
                facts.groups.iter().any(|group| {
                    group.uid == mention.group_uid
                        && group
                            .final_votes
                            .iter()
                            .any(|vote| vote.scrutin_uid == mention.scrutin_uid)
                })
            })
            .collect();
        explanations.sort_by_key(|mention| mention.rank);

        Ok(Some(DossierGroupActions {
            facts,
            facts_fingerprint,
            summaries,
            explanations,
        }))
    }
}

pub fn fingerprint(facts: &DossierGroupFacts) -> FactsFingerprint {
    let bytes = serde_json::to_vec(facts).expect("facts are serializable");
    let digest = Sha256::digest(bytes);
    FactsFingerprint::new(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::application::ports::dossier_group_actions_repository::GeneratedGroupSummary;
    use crate::application::ports::vote_explanation_repository::{
        ScrutinExplanationsRead, StoredSitting, StoredVersion,
    };
    use crate::domain::actor::{ActorUid, GroupUid};
    use crate::domain::amendment::AmendmentUid;
    use crate::domain::dossier::DossierUid;
    use crate::domain::dossier_group_facts::{AmendmentFact, FinalVoteFact, GroupFacts};
    use crate::domain::dossier_summary::{SummarySource, SummaryStatus};
    use crate::domain::scrutin::{ScrutinUid, VoteTally};
    use crate::domain::sitting_record::{SittingRecordUid, SittingRef};

    fn facts() -> DossierGroupFacts {
        DossierGroupFacts {
            dossier_uid: DossierUid::new("D1".to_string()).unwrap(),
            title: "Texte".into(),
            official_url: Some("https://example.test/dossier".into()),
            legislature: 17,
            period_start: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            period_end: Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            groups: vec![GroupFacts {
                uid: GroupUid::new("G1".to_string()).unwrap(),
                abbrev: "G".into(),
                label: "Groupe".into(),
                color: None,
                start_date: None,
                end_date: None,
                final_votes: vec![FinalVoteFact {
                    scrutin_uid: ScrutinUid::new("S1".to_string()).unwrap(),
                    number: "1".into(),
                    date: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
                    legislature: 17,
                    subject: "l'ensemble du texte (première lecture)".into(),
                    text_label: "texte".into(),
                    reading: Some("première lecture".into()),
                    outcome_code: "adopted".into(),
                    outcome_label: "Adopté".into(),
                    majority_position: Some("pour".into()),
                    member_count: None,
                    tally: VoteTally::default(),
                }],
                amendments: vec![],
            }],
        }
    }

    /// Faits figes, volontairement non triviaux: Some et None, dates, decompte non nul,
    /// un vote final et un amendement. Toute derive de leur serialisation JSON invalide
    /// les syntheses stockees: la valeur attendue ci-dessous ne doit jamais bouger.
    fn stable_facts() -> DossierGroupFacts {
        DossierGroupFacts {
            dossier_uid: DossierUid::new("DLR5L17N50000".to_string()).unwrap(),
            title: "Projet de loi de finances".into(),
            official_url: Some("https://example.test/dossier".into()),
            legislature: 17,
            period_start: Some(NaiveDate::from_ymd_opt(2025, 10, 1).unwrap()),
            period_end: None,
            groups: vec![
                GroupFacts {
                    uid: GroupUid::new("PO800490".to_string()).unwrap(),
                    abbrev: "GRP".into(),
                    label: "Groupe exemple".into(),
                    color: Some("#112233".into()),
                    start_date: Some(NaiveDate::from_ymd_opt(2024, 7, 18).unwrap()),
                    end_date: None,
                    final_votes: vec![FinalVoteFact {
                        scrutin_uid: ScrutinUid::new("VTANR5L17V100".to_string()).unwrap(),
                        number: "100".into(),
                        date: NaiveDate::from_ymd_opt(2025, 12, 16).unwrap(),
                        legislature: 17,
                        subject: "l'ensemble du projet de loi (première lecture)".into(),
                        text_label: "Projet de loi de finances".into(),
                        reading: Some("première lecture".into()),
                        outcome_code: "adopted".into(),
                        outcome_label: "Adopté".into(),
                        majority_position: None,
                        member_count: Some(120),
                        tally: VoteTally {
                            votes_for: 80,
                            votes_against: 20,
                            abstentions: 5,
                            not_voting: 10,
                            voluntary_not_voting: 5,
                        },
                    }],
                    amendments: vec![AmendmentFact {
                        uid: AmendmentUid::new("AMANR5L17PO1".to_string()).unwrap(),
                        number: "42".into(),
                        target_title: "Article 3".into(),
                        target_kind: None,
                        fate_code: "rejected".into(),
                        fate_label: "Rejeté".into(),
                        deposited_on: Some(NaiveDate::from_ymd_opt(2025, 11, 2).unwrap()),
                        summary_available: true,
                    }],
                },
                GroupFacts {
                    uid: GroupUid::new("PO800491".to_string()).unwrap(),
                    abbrev: "AUT".into(),
                    label: "Autre groupe".into(),
                    color: None,
                    start_date: None,
                    end_date: Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()),
                    final_votes: vec![],
                    amendments: vec![],
                },
            ],
        }
    }

    #[test]
    fn fingerprint_is_stable() {
        assert_eq!(
            fingerprint(&stable_facts()).as_str(),
            "5b6335ff95c37010da97ec4e377a8722969e2e816197e8d9e430df98139ecec7"
        );
    }

    struct FakeFacts(DossierGroupFacts);
    #[async_trait]
    impl DossierGroupActionsRepository for FakeFacts {
        async fn load_facts(&self, _: &str) -> Result<Option<DossierGroupFacts>, RepositoryError> {
            Ok(Some(self.0.clone()))
        }
        async fn list_facts(&self, _: usize) -> Result<Vec<DossierGroupFacts>, RepositoryError> {
            Ok(vec![self.0.clone()])
        }
    }

    struct FakeSummaries(Mutex<Vec<StoredGroupSummary>>);
    #[async_trait]
    impl DossierSummaryRepository for FakeSummaries {
        async fn summaries_for(&self, _: &str) -> Result<Vec<StoredGroupSummary>, RepositoryError> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn mark_pending(
            &self,
            _: &str,
            _: &[String],
            _: &str,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn save_ready(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &[GeneratedGroupSummary],
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn hides_stale_summaries_but_keeps_raw_facts() {
        let facts = facts();
        let summaries = FakeSummaries(Mutex::new(vec![StoredGroupSummary {
            group_uid: GroupUid::new("G1".to_string()).unwrap(),
            status: SummaryStatus::Ready,
            paragraph: Some("Texte descriptif".into()),
            facts_fingerprint: "old".into(),
            model: Some("model".into()),
            prompt_version: Some("v1".into()),
            generated_at: None,
            sources: vec![SummarySource {
                source_id: "dossier:D1".into(),
                kind: "dossier".into(),
                uid: "D1".into(),
                label: "Dossier".into(),
                official_url: None,
            }],
        }]));
        let result =
            GetDossierGroupActions::new(&FakeFacts(facts), &summaries, &FakeMentions(vec![]))
                .execute("D1")
                .await
                .unwrap()
                .unwrap();
        assert!(result.summaries.is_empty());
        assert_eq!(result.facts.groups.len(), 1);
    }

    // Local: rend les mentions dans l'ordre donné, sans les trier, pour que le tri soit celui du cas d'usage.
    struct FakeMentions(Vec<ExplanationMention>);
    #[async_trait]
    impl VoteExplanationRepository for FakeMentions {
        async fn coverage(
            &self,
        ) -> Result<
            (
                Vec<(
                    crate::domain::sitting_record::AttachmentCounters,
                    crate::domain::sitting_record::ExplanationCounters,
                )>,
                crate::domain::vote_explanation::FinalVoteAttachmentCounts,
            ),
            RepositoryError,
        > {
            unreachable!()
        }
        async fn stored_versions(
            &self,
        ) -> Result<HashMap<SittingRecordUid, StoredVersion>, RepositoryError> {
            unreachable!()
        }
        async fn replace_sitting(&self, _: &StoredSitting) -> Result<(), RepositoryError> {
            unreachable!()
        }
        async fn explanations_for(
            &self,
            _: &ScrutinUid,
            _: &SittingRef,
        ) -> Result<ScrutinExplanationsRead, RepositoryError> {
            unreachable!()
        }
        async fn mentions_for(
            &self,
            scrutins: &[ScrutinUid],
        ) -> Result<Vec<ExplanationMention>, RepositoryError> {
            Ok(self
                .0
                .iter()
                .filter(|m| scrutins.contains(&m.scrutin_uid))
                .cloned()
                .collect())
        }
    }

    fn mention(scrutin: &str, group: &str, rank: u16, speaker: &str) -> ExplanationMention {
        ExplanationMention {
            scrutin_uid: ScrutinUid::new(scrutin.to_string()).unwrap(),
            group_uid: GroupUid::new(group.to_string()).unwrap(),
            rank,
            speaker_uid: ActorUid::new(speaker.to_string()).unwrap(),
            speaker_name: Some(format!("Orateur {speaker}")),
        }
    }

    async fn run_with(mentions: Vec<ExplanationMention>) -> DossierGroupActions {
        let summaries = FakeSummaries(Mutex::new(vec![]));
        GetDossierGroupActions::new(
            &FakeFacts(two_groups_facts()),
            &summaries,
            &FakeMentions(mentions),
        )
        .execute("D1")
        .await
        .unwrap()
        .unwrap()
    }

    /// G1 a un vote final sur S1, G2 un vote final sur S2.
    fn two_groups_facts() -> DossierGroupFacts {
        let mut facts = facts();
        let mut second = facts.groups[0].clone();
        second.uid = GroupUid::new("G2".to_string()).unwrap();
        second.final_votes[0].scrutin_uid = ScrutinUid::new("S2".to_string()).unwrap();
        facts.groups.push(second);
        facts
    }

    #[tokio::test]
    async fn lists_one_mention_per_attached_speaker_under_the_group_final_vote_in_report_order() {
        let result = run_with(vec![
            mention("S1", "G1", 3, "PA3"),
            mention("S1", "G1", 1, "PA1"),
            mention("S1", "G1", 2, "PA2"),
        ])
        .await;

        let speakers: Vec<(u16, &str)> = result
            .explanations
            .iter()
            .map(|m| (m.rank, m.speaker_uid.as_str()))
            .collect();
        assert_eq!(speakers, vec![(1, "PA1"), (2, "PA2"), (3, "PA3")]);
    }

    #[tokio::test]
    async fn lists_no_mention_for_a_group_without_attached_speaker() {
        let result = run_with(vec![mention("S1", "G1", 1, "PA1")]).await;

        assert!(result
            .explanations
            .iter()
            .all(|m| m.group_uid.as_str() != "G2"));
        assert_eq!(result.explanations.len(), 1);
    }

    #[tokio::test]
    async fn drops_a_mention_whose_group_has_no_final_vote_on_that_scrutin() {
        let result = run_with(vec![
            mention("S1", "G2", 1, "PA1"),
            mention("S1", "G1", 2, "PA2"),
        ])
        .await;

        let kept: Vec<(&str, &str)> = result
            .explanations
            .iter()
            .map(|m| (m.group_uid.as_str(), m.speaker_uid.as_str()))
            .collect();
        assert_eq!(kept, vec![("G1", "PA2")]);
    }

    #[tokio::test]
    async fn keeps_the_facts_fingerprint_unchanged_when_explanations_exist() {
        let result = run_with(vec![mention("S1", "G1", 1, "PA1")]).await;

        assert_eq!(result.explanations.len(), 1);
        assert_eq!(result.facts_fingerprint, fingerprint(&two_groups_facts()));
    }
}
