use std::fmt::Display;

use chrono::{DateTime, Utc};

use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::domain::ingestion::{IngestionSource, SourceCheck};

#[derive(Clone, Copy)]
pub struct Journaling<'a> {
    target: Option<(&'a dyn IngestionJournal, DateTime<Utc>)>,
}

impl<'a> Journaling<'a> {
    pub fn off() -> Self {
        Self { target: None }
    }

    pub fn to(journal: &'a dyn IngestionJournal, attempted_at: DateTime<Utc>) -> Self {
        Self {
            target: Some((journal, attempted_at)),
        }
    }

    pub async fn record<T, E: Display>(
        &self,
        source: IngestionSource,
        result: &Result<T, E>,
        items_read: impl Fn(&T) -> u64,
    ) {
        let Some((journal, attempted_at)) = self.target else {
            return;
        };
        let check = match result {
            Ok(value) => SourceCheck::succeeded(source, attempted_at, items_read(value)),
            Err(e) => SourceCheck::failed(source, attempted_at, e.to_string()),
        };
        if let Err(e) = journal.record(&check).await {
            tracing::warn!("Ingestion journal write failed for {}: {e}", source.as_str());
        }
    }
}
