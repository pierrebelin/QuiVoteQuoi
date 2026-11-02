//! Faits officiels d'un dossier, groupe par groupe: la matiere dont l'empreinte
//! decide si une synthese stockee est encore montrable.
//!
//! La serialisation JSON de ces structures est hachee par le cas d'usage
//! `get_dossier_group_actions::fingerprint`: renommer, reordonner ou retyper un
//! champ, ou modifier un attribut serde, invalide toutes les syntheses stockees.
//! Le test `fingerprint_is_stable` fige l'empreinte attendue.

use chrono::NaiveDate;
use serde::Serialize;

use super::scrutin::VoteTally;

#[derive(Debug, Clone, Serialize)]
pub struct DossierGroupFacts {
    pub dossier_uid: String,
    pub title: String,
    pub official_url: Option<String>,
    pub legislature: u16,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub groups: Vec<GroupFacts>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupFacts {
    pub uid: String,
    pub abbrev: String,
    pub label: String,
    pub color: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub final_votes: Vec<FinalVoteFact>,
    pub amendments: Vec<AmendmentFact>,
}

impl GroupFacts {
    /// Un groupe a des actes a decrire des qu'il porte au moins un vote final ou
    /// un amendement. Sans acte il n'y a rien a resumer: ni synthese attendue,
    /// ni synthese generee.
    pub fn has_actions(&self) -> bool {
        !self.final_votes.is_empty() || !self.amendments.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FinalVoteFact {
    pub scrutin_uid: String,
    pub number: String,
    pub date: NaiveDate,
    pub legislature: u16,
    pub subject: String,
    pub text_label: String,
    pub reading: Option<String>,
    pub outcome_code: String,
    pub outcome_label: String,
    pub majority_position: Option<String>,
    pub member_count: Option<u16>,
    pub tally: VoteTally,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmendmentFact {
    pub uid: String,
    pub number: String,
    pub target_title: String,
    pub target_kind: Option<String>,
    pub fate_code: String,
    pub fate_label: String,
    pub deposited_on: Option<NaiveDate>,
    pub summary_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(final_votes: Vec<FinalVoteFact>, amendments: Vec<AmendmentFact>) -> GroupFacts {
        GroupFacts {
            uid: "G1".into(),
            abbrev: "G".into(),
            label: "Groupe".into(),
            color: None,
            start_date: None,
            end_date: None,
            final_votes,
            amendments,
        }
    }

    fn final_vote() -> FinalVoteFact {
        FinalVoteFact {
            scrutin_uid: "S1".into(),
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
        }
    }

    fn amendment() -> AmendmentFact {
        AmendmentFact {
            uid: "A1".into(),
            number: "1".into(),
            target_title: "Article".into(),
            target_kind: None,
            fate_code: "adopted".into(),
            fate_label: "Adopte".into(),
            deposited_on: None,
            summary_available: false,
        }
    }

    #[test]
    fn a_group_with_only_a_final_vote_has_actions() {
        assert!(group(vec![final_vote()], vec![]).has_actions());
    }

    #[test]
    fn a_group_with_only_an_amendment_has_actions() {
        assert!(group(vec![], vec![amendment()]).has_actions());
    }

    #[test]
    fn a_group_with_neither_final_vote_nor_amendment_has_no_actions() {
        assert!(!group(vec![], vec![]).has_actions());
    }
}
