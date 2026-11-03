//! Use case de fraicheur des sources.
use chrono::{DateTime, NaiveDate, Utc};

use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::application::ports::scrutin_repository::{RepositoryError, ScrutinRepository};
use crate::domain::ingestion::{Freshness, IngestionSource, SourceHistory};

/// Etat d'une source: son historique et l'etat qui en decoule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFreshness {
    pub history: SourceHistory,
    pub freshness: Freshness,
}

/// Les quatre sources dans un ordre fixe, et la date du dernier scrutin donnee a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshnessOverview {
    pub sources: Vec<SourceFreshness>,
    pub last_scrutin_date: Option<NaiveDate>,
}

pub struct DescribeFreshness<'a> {
    journal: &'a dyn IngestionJournal,
    scrutins: &'a dyn ScrutinRepository,
}

impl<'a> DescribeFreshness<'a> {
    pub fn new(journal: &'a dyn IngestionJournal, scrutins: &'a dyn ScrutinRepository) -> Self {
        Self { journal, scrutins }
    }

    pub async fn execute(
        &self,
        now: DateTime<Utc>,
    ) -> Result<FreshnessOverview, RepositoryError> {
        let histories = self.journal.histories().await?;
        let last_scrutin_date = self.scrutins.dataset_shape().await?.last_scrutin_date;

        let sources = IngestionSource::ALL
            .into_iter()
            .map(|source| {
                let history = histories
                    .iter()
                    .find(|history| history.source() == source)
                    .cloned()
                    .unwrap_or_else(|| SourceHistory::never(source));
                let freshness = history.freshness(now);
                SourceFreshness { history, freshness }
            })
            .collect();

        Ok(FreshnessOverview {
            sources,
            last_scrutin_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::application::use_cases::journal_fakes::InMemoryIngestionJournal;
    use crate::application::use_cases::read_fakes::FakeScrutins;
    use crate::domain::dataset::DatasetShape;
    use crate::domain::ingestion::{IngestionSource, LastAttempt};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, 20, 0, 0).unwrap()
    }

    fn succeeded_at(source: IngestionSource, hour: u32) -> SourceHistory {
        let at = Utc.with_ymd_and_hms(2026, 11, 2, hour, 0, 0).unwrap();
        SourceHistory::new(source, LastAttempt::new(at, true), Some(at), 0).unwrap()
    }

    fn scrutins_with(last: Option<NaiveDate>) -> FakeScrutins {
        FakeScrutins {
            rows: vec![],
            total: 0,
            shape: DatasetShape {
                last_scrutin_date: last,
                ..DatasetShape::default()
            },
        }
    }

    #[tokio::test]
    async fn should_describe_every_source_in_fixed_order_when_journal_lists_them_shuffled() {
        let journal = InMemoryIngestionJournal::with_histories(vec![
            succeeded_at(IngestionSource::Amendments, 19),
            succeeded_at(IngestionSource::Scrutins, 10),
            succeeded_at(IngestionSource::ActorRegistry, 18),
            succeeded_at(IngestionSource::Dossiers, 19),
        ]);

        let overview = DescribeFreshness::new(&journal, &scrutins_with(None))
            .execute(now())
            .await
            .unwrap();

        let order: Vec<IngestionSource> =
            overview.sources.iter().map(|s| s.history.source()).collect();
        assert_eq!(order, IngestionSource::ALL.to_vec());
        let states: Vec<Freshness> = overview.sources.iter().map(|s| s.freshness).collect();
        assert_eq!(
            states,
            vec![
                Freshness::UpToDate,
                Freshness::UpToDate,
                Freshness::Late,
                Freshness::UpToDate
            ]
        );
    }

    #[tokio::test]
    async fn should_describe_source_as_never_synchronised_when_journal_has_no_attempt_for_it() {
        let journal = InMemoryIngestionJournal::with_histories(vec![
            succeeded_at(IngestionSource::ActorRegistry, 19),
            succeeded_at(IngestionSource::Scrutins, 19),
        ]);

        let overview = DescribeFreshness::new(&journal, &scrutins_with(None))
            .execute(now())
            .await
            .unwrap();

        assert_eq!(overview.sources.len(), 4);
        assert_eq!(
            overview.sources[1].history,
            SourceHistory::never(IngestionSource::Dossiers)
        );
        assert_eq!(overview.sources[1].freshness, Freshness::NeverSynchronised);
        assert_eq!(overview.sources[3].freshness, Freshness::NeverSynchronised);
        assert_eq!(overview.sources[0].freshness, Freshness::UpToDate);
    }

    #[tokio::test]
    async fn should_give_last_published_scrutin_date_whatever_the_sources_state() {
        let last = NaiveDate::from_ymd_opt(2026, 10, 28);
        let journal = InMemoryIngestionJournal::with_histories(vec![succeeded_at(
            IngestionSource::Scrutins,
            1,
        )]);

        let overview = DescribeFreshness::new(&journal, &scrutins_with(last))
            .execute(now())
            .await
            .unwrap();

        assert_eq!(overview.sources[2].freshness, Freshness::Late);
        assert_eq!(overview.last_scrutin_date, last);
    }

    #[tokio::test]
    async fn should_give_no_scrutin_date_when_dataset_holds_no_scrutin() {
        let journal = InMemoryIngestionJournal::default();

        let overview = DescribeFreshness::new(&journal, &scrutins_with(None))
            .execute(now())
            .await
            .unwrap();

        assert_eq!(overview.sources.len(), 4);
        assert_eq!(overview.last_scrutin_date, None);
    }
}
