//! Couverture des explications de vote (compteurs de rattachement).

use crate::application::ports::vote_explanation_repository::{
    RepositoryError, VoteExplanationRepository,
};
use crate::domain::vote_explanation::ExplanationCoverage;

pub struct DescribeExplanationCoverage<'a> {
    repository: &'a dyn VoteExplanationRepository,
}

impl<'a> DescribeExplanationCoverage<'a> {
    pub fn new(repository: &'a dyn VoteExplanationRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> Result<ExplanationCoverage, RepositoryError> {
        let (sittings, final_votes) = self.repository.coverage().await?;
        Ok(ExplanationCoverage::of(&sittings, final_votes))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::application::ports::vote_explanation_repository::StoredSitting;
    use crate::application::use_cases::explanation_fakes::InMemoryVoteExplanations;
    use crate::domain::sitting_record::{
        AttachmentCounters, ExplanationCounters, SittingKeysFingerprint, SittingRecordUid,
        SittingRef,
    };
    use crate::domain::vote_explanation::FinalVoteAttachmentCounts;

    fn sitting(
        uid: &str,
        counters: AttachmentCounters,
        block_counters: ExplanationCounters,
    ) -> StoredSitting {
        StoredSitting {
            record_uid: SittingRecordUid::new(uid.into()).unwrap(),
            sitting_ref: SittingRef::new(format!("ref-{uid}")).unwrap(),
            date: NaiveDate::from_ymd_opt(2025, 3, 27).unwrap(),
            version: "1".into(),
            generated_at: NaiveDate::from_ymd_opt(2025, 3, 28)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            keys: SittingKeysFingerprint::from_stored(String::new()),
            counters,
            block_counters,
            attached: vec![],
            explanations: vec![],
        }
    }

    #[tokio::test]
    async fn adds_up_the_attachment_counters_of_every_stored_sitting() {
        let repository = InMemoryVoteExplanations::with_sittings(vec![
            sitting(
                "CR1",
                AttachmentCounters {
                    results_attached: 3,
                    results_unmatched: 1,
                    results_ambiguous: 0,
                },
                ExplanationCounters {
                    blocks_unattached: 1,
                    headings_unrecognized: 2,
                },
            ),
            sitting(
                "CR2",
                AttachmentCounters {
                    results_attached: 4,
                    results_unmatched: 2,
                    results_ambiguous: 5,
                },
                ExplanationCounters {
                    blocks_unattached: 6,
                    headings_unrecognized: 0,
                },
            ),
        ]);

        let coverage = DescribeExplanationCoverage::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(coverage.results_attached, 7);
        assert_eq!(coverage.results_unmatched, 3);
        assert_eq!(coverage.results_ambiguous, 5);
        assert_eq!(coverage.blocks_unattached, 7);
        assert_eq!(coverage.headings_unrecognized, 2);
    }

    #[tokio::test]
    async fn reports_final_votes_without_attachment_or_block_and_speakers_without_group() {
        let repository =
            InMemoryVoteExplanations::default().with_final_vote_counts(FinalVoteAttachmentCounts {
                unattached: 1,
                without_block: 2,
                speakers_without_group: 1,
            });

        let coverage = DescribeExplanationCoverage::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(coverage.final_votes_unattached, 1);
        assert_eq!(coverage.final_votes_without_block, 2);
        assert_eq!(coverage.speakers_without_group, 1);
    }

    #[tokio::test]
    async fn reports_zero_counters_before_any_ingestion() {
        let repository = InMemoryVoteExplanations::default();

        let coverage = DescribeExplanationCoverage::new(&repository)
            .execute()
            .await
            .unwrap();

        assert_eq!(coverage, ExplanationCoverage::default());
    }
}
