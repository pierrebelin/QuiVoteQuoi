//! Double en memoire des votes sur l'ensemble.
//!
//! Port d'etat: la liste rendue suit l'ordre des lignes semees, restreinte aux
//! bornes de date du filtre. Les methodes qu'aucun test n'atteint sont
//! `unreachable!()`.

use async_trait::async_trait;

use crate::application::ports::final_vote_repository::{
    FinalVoteFilter, FinalVotePage, FinalVoteRecord, FinalVoteRepository, FinalVoteTotals,
    GroupOption,
};
use crate::application::ports::RepositoryError;
use crate::domain::scrutin::VoteTally;

/// Vote sur l'ensemble `number` du `date`, sans groupe ni theme.
pub fn final_vote_record(number: &str, date: chrono::NaiveDate) -> FinalVoteRecord {
    FinalVoteRecord {
        scrutin_uid: format!("VTANR5L17V{number}"),
        number: number.into(),
        date,
        subject: "l'ensemble du projet de loi".into(),
        ballot_type_label: "Scrutin public solennel".into(),
        outcome_code: "adopt\u{00e9}".into(),
        outcome_label: "adopt\u{00e9}".into(),
        text_key: "text".into(),
        text_label: "Texte".into(),
        dossier_uid: None,
        dossier_label: None,
        official_text: None,
        synthesis: VoteTally::default(),
        families: vec![],
        tallies: vec![],
    }
}

#[derive(Default)]
pub struct FakeFinalVotes {
    pub records: Vec<FinalVoteRecord>,
}

#[async_trait]
impl FinalVoteRepository for FakeFinalVotes {
    async fn list_final_votes(
        &self,
        filter: &FinalVoteFilter,
    ) -> Result<FinalVotePage, RepositoryError> {
        let items: Vec<FinalVoteRecord> = self
            .records
            .iter()
            .filter(|r| filter.from.is_none_or(|from| from <= r.date))
            .filter(|r| filter.to.is_none_or(|to| r.date <= to))
            .cloned()
            .collect();
        Ok(FinalVotePage {
            total: items.len() as i64,
            items,
        })
    }

    async fn groups(&self) -> Result<Vec<GroupOption>, RepositoryError> {
        unreachable!()
    }

    async fn totals(&self) -> Result<FinalVoteTotals, RepositoryError> {
        unreachable!()
    }
}
