use super::get_dossier_group_actions::fingerprint;
use crate::application::ports::dossier_group_actions_repository::{
    DossierGroupActionsRepository, DossierSummaryRepository, RepositoryError,
};
use crate::application::ports::dossier_summary_generator::DossierSummaryGenerator;

#[derive(Debug, Clone, Default)]
pub struct DossierSummaryRefreshReport {
    pub dossiers_seen: usize,
    pub dossiers_refreshed: usize,
    pub summaries_ready: usize,
    pub summaries_pending: usize,
    pub anomaly: Option<String>,
}

pub struct RefreshDossierGroupSummaries<'a> {
    facts: &'a dyn DossierGroupActionsRepository,
    summaries: &'a dyn DossierSummaryRepository,
    generator: &'a dyn DossierSummaryGenerator,
}

impl<'a> RefreshDossierGroupSummaries<'a> {
    pub fn new(
        facts: &'a dyn DossierGroupActionsRepository,
        summaries: &'a dyn DossierSummaryRepository,
        generator: &'a dyn DossierSummaryGenerator,
    ) -> Self {
        Self {
            facts,
            summaries,
            generator,
        }
    }

    pub async fn execute(
        &self,
        batch: usize,
    ) -> Result<DossierSummaryRefreshReport, RepositoryError> {
        let facts_list = self.facts.list_facts(batch.max(1)).await?;
        let mut report = DossierSummaryRefreshReport {
            dossiers_seen: facts_list.len(),
            ..Default::default()
        };

        for facts in facts_list {
            let fingerprint = fingerprint(&facts);
            let expected: Vec<String> = facts
                .groups
                .iter()
                .filter(|group| group.has_actions())
                .map(|group| group.uid.to_string())
                .collect();
            let stored = self
                .summaries
                .summaries_for(facts.dossier_uid.as_str())
                .await?;
            let already_ready = !expected.is_empty()
                && expected.iter().all(|uid| {
                    stored.iter().any(|summary| {
                        summary.group_uid.as_str() == uid
                            && summary.is_up_to_date(
                                &fingerprint,
                                self.generator.model(),
                                self.generator.prompt_version(),
                            )
                    })
                });
            if already_ready {
                report.summaries_ready += expected.len();
                continue;
            }

            self.summaries
                .mark_pending(facts.dossier_uid.as_str(), &expected, fingerprint.as_str())
                .await?;
            report.dossiers_refreshed += 1;

            if expected.is_empty() {
                continue;
            }

            match self.generator.generate(&facts).await {
                Ok(generated) => {
                    report.summaries_ready += generated.len();
                    self.summaries
                        .save_ready(
                            facts.dossier_uid.as_str(),
                            fingerprint.as_str(),
                            self.generator.model(),
                            self.generator.prompt_version(),
                            &generated,
                        )
                        .await?;
                }
                Err(error) => {
                    report.summaries_pending += expected.len();
                    report.anomaly.get_or_insert_with(|| error.to_string());
                }
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::sync::Mutex;

    use crate::application::ports::dossier_group_actions_repository::GeneratedGroupSummary;
    use crate::application::ports::dossier_summary_generator::SummaryGeneratorError;
    use crate::domain::actor::GroupUid;
    use crate::domain::dossier::DossierUid;
    use crate::domain::dossier_group_facts::{DossierGroupFacts, FinalVoteFact, GroupFacts};
    use crate::domain::dossier_summary::{StoredGroupSummary, SummarySource, SummaryStatus};
    use crate::domain::scrutin::{ScrutinUid, VoteTally};

    fn facts(with_vote: bool) -> DossierGroupFacts {
        DossierGroupFacts {
            dossier_uid: DossierUid::new("D1".to_string()).unwrap(),
            title: "Dossier".into(),
            official_url: None,
            legislature: 17,
            period_start: None,
            period_end: None,
            groups: vec![GroupFacts {
                uid: GroupUid::new("G1".to_string()).unwrap(),
                abbrev: "G".into(),
                label: "Groupe".into(),
                color: None,
                start_date: None,
                end_date: None,
                final_votes: if with_vote {
                    vec![FinalVoteFact {
                        scrutin_uid: ScrutinUid::new("S1".to_string()).unwrap(),
                        number: "1".into(),
                        date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        legislature: 17,
                        subject: "l'ensemble du texte".into(),
                        text_label: "texte".into(),
                        reading: None,
                        outcome_code: "adopted".into(),
                        outcome_label: "Adopte".into(),
                        majority_position: None,
                        member_count: None,
                        tally: VoteTally::default(),
                    }]
                } else {
                    vec![]
                },
                amendments: vec![],
            }],
        }
    }

    struct FactsRepo(DossierGroupFacts);
    #[async_trait]
    impl DossierGroupActionsRepository for FactsRepo {
        async fn load_facts(&self, _: &str) -> Result<Option<DossierGroupFacts>, RepositoryError> {
            Ok(Some(self.0.clone()))
        }
        async fn list_facts(&self, _: usize) -> Result<Vec<DossierGroupFacts>, RepositoryError> {
            Ok(vec![self.0.clone()])
        }
    }

    struct SummaryRepo {
        ready: Mutex<Vec<StoredGroupSummary>>,
        pending: Mutex<usize>,
    }
    #[async_trait]
    impl DossierSummaryRepository for SummaryRepo {
        async fn summaries_for(&self, _: &str) -> Result<Vec<StoredGroupSummary>, RepositoryError> {
            Ok(self.ready.lock().unwrap().clone())
        }
        async fn mark_pending(
            &self,
            _: &str,
            groups: &[String],
            _: &str,
        ) -> Result<(), RepositoryError> {
            *self.pending.lock().unwrap() += groups.len();
            Ok(())
        }
        async fn save_ready(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            summaries: &[GeneratedGroupSummary],
        ) -> Result<(), RepositoryError> {
            self.ready
                .lock()
                .unwrap()
                .extend(summaries.iter().map(|s| StoredGroupSummary {
                    group_uid: GroupUid::new(s.group_uid.clone()).unwrap(),
                    status: SummaryStatus::Ready,
                    paragraph: Some(s.paragraph.clone()),
                    facts_fingerprint: "current".into(),
                    model: None,
                    prompt_version: None,
                    generated_at: None,
                    sources: s.sources.clone(),
                }));
            Ok(())
        }
    }

    struct Generator {
        fails: bool,
    }
    #[async_trait]
    impl DossierSummaryGenerator for Generator {
        async fn generate(
            &self,
            _: &DossierGroupFacts,
        ) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
            if self.fails {
                return Err(SummaryGeneratorError::Unavailable("test".into()));
            }
            Ok(vec![GeneratedGroupSummary {
                group_uid: "G1".into(),
                paragraph: "Description factuelle".into(),
                sources: vec![SummarySource {
                    source_id: "dossier:D1".into(),
                    kind: "dossier".into(),
                    uid: "D1".into(),
                    label: "Dossier".into(),
                    official_url: None,
                }],
            }])
        }
        fn model(&self) -> &str {
            "test"
        }
        fn prompt_version(&self) -> &str {
            "test"
        }
    }

    #[tokio::test]
    async fn generator_failure_leaves_summary_pending_and_reports_anomaly() {
        let repo = SummaryRepo {
            ready: Mutex::new(vec![]),
            pending: Mutex::new(0),
        };
        let report = RefreshDossierGroupSummaries::new(
            &FactsRepo(facts(true)),
            &repo,
            &Generator { fails: true },
        )
        .execute(10)
        .await
        .unwrap();
        assert_eq!(report.summaries_pending, 1);
        assert!(report.anomaly.is_some());
        assert_eq!(*repo.pending.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn group_without_actions_is_not_filled_with_a_summary() {
        let repo = SummaryRepo {
            ready: Mutex::new(vec![]),
            pending: Mutex::new(0),
        };
        let report = RefreshDossierGroupSummaries::new(
            &FactsRepo(facts(false)),
            &repo,
            &Generator { fails: false },
        )
        .execute(10)
        .await
        .unwrap();
        assert_eq!(report.summaries_ready, 0);
        assert_eq!(*repo.pending.lock().unwrap(), 0);
    }
}
