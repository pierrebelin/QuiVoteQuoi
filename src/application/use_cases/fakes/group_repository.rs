//! Fake d'etat du port `GroupRepository`: des groupes en memoire, et la
//! derniere requete de statistiques recue.

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::application::ports::group_repository::{
    GroupRecord, GroupRepository, GroupStatisticsRecord, RepositoryError,
};

#[derive(Default)]
pub struct InMemoryGroupRepository {
    pub records: Mutex<Vec<GroupRecord>>,
    pub statistics: Mutex<GroupStatisticsRecord>,
    pub last_uids: Mutex<Option<Vec<String>>>,
    pub last_reference_date: Mutex<Option<NaiveDate>>,
}

#[async_trait]
impl GroupRepository for InMemoryGroupRepository {
    async fn list_groups(
        &self,
        _today: NaiveDate,
    ) -> Result<Vec<GroupRecord>, RepositoryError> {
        Ok(self.records.lock().unwrap().clone())
    }

    async fn statistics(
        &self,
        group_uids: &[String],
        reference_date: NaiveDate,
    ) -> Result<GroupStatisticsRecord, RepositoryError> {
        *self.last_uids.lock().unwrap() = Some(group_uids.to_vec());
        *self.last_reference_date.lock().unwrap() = Some(reference_date);
        Ok(self.statistics.lock().unwrap().clone())
    }
}
