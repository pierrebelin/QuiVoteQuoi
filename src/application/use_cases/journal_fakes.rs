//! Double en memoire du journal d'ingestion.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::application::ports::ingestion_journal::IngestionJournal;
use crate::application::ports::RepositoryError;
use crate::domain::ingestion::{SourceCheck, SourceHistory};

#[derive(Default)]
pub struct InMemoryIngestionJournal {
    checks: Mutex<Vec<SourceCheck>>,
    failing: bool,
    histories: Vec<SourceHistory>,
}

impl InMemoryIngestionJournal {
    /// Journal dont l'ecriture echoue (la tentative reste lisible).
    pub fn failing() -> Self {
        Self {
            checks: Mutex::new(vec![]),
            failing: true,
            histories: vec![],
        }
    }

    /// Journal dont la lecture rend ces historiques.
    pub fn with_histories(histories: Vec<SourceHistory>) -> Self {
        Self {
            histories,
            ..Self::default()
        }
    }

    pub fn recorded(&self) -> Vec<SourceCheck> {
        self.checks.lock().unwrap().clone()
    }
}

#[async_trait]
impl IngestionJournal for InMemoryIngestionJournal {
    async fn record(&self, check: &SourceCheck) -> Result<(), RepositoryError> {
        self.checks.lock().unwrap().push(check.clone());
        if self.failing {
            return Err(RepositoryError::Database(
                "journal indisponible".to_string(),
            ));
        }
        Ok(())
    }

    async fn histories(&self) -> Result<Vec<SourceHistory>, RepositoryError> {
        Ok(self.histories.clone())
    }
}
