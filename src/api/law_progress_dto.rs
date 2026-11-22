//! DTO de « Où en est la loi » (todo/navette/SPEC-navette.md) : l'état d'un
//! dossier (RM-01 à RM-04), ses jalons (RM-05) et la réponse à « Ce texte
//! est-il devenu loi ? » (RM-06).
//!
//! Les phrases sont des gabarits fermés à trous : dates et numéros viennent
//! de la source, aucun mot n'est produit par un LLM (README.md §8).
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::application::ports::dossier_repository::LawStateCounts;
use crate::domain::dossier::{Chamber, DossierOutcome, LegislativeDossier, LegislativeStage};
use crate::domain::law_progress::{
    step_of_vote, Decision, DecisionSense, DecisionStage, LawProgress, LawState, LawStateSummary,
    Milestone, MilestoneStatus, MilestoneStep,
};
use crate::domain::scrutin::ScrutinUid;

const MONTHS: [&str; 12] = [
    "janvier",
    "f\u{e9}vrier",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "ao\u{fb}t",
    "septembre",
    "octobre",
    "novembre",
    "d\u{e9}cembre",
];

/// Date écrite en toutes lettres : « 1er octobre 2026 », « 20 novembre 2026 ».
pub fn french_date(date: NaiveDate) -> String {
    let day = match date.day() {
        1 => "1er".to_string(),
        day => day.to_string(),
    };
    format!("{day} {} {}", MONTHS[date.month0() as usize], date.year())
}

/// `an`, `sn` ou `cmp` ; `None` quand la source ne nomme pas la chambre.
pub fn chamber_code(chamber: Chamber) -> Option<&'static str> {
    match chamber {
        Chamber::AssembleeNationale => Some("an"),
        Chamber::Senat => Some("sn"),
        Chamber::Joint => Some("cmp"),
        Chamber::None => None,
    }
}

/// Dernière décision d'un dossier (RM-03 : « modifié par le Sénat le 20 nov. »).
#[derive(Debug, Serialize)]
pub struct LastDecisionDto {
    pub chamber: Option<&'static str>,
    pub sense: &'static str,
    pub date: NaiveDate,
}

/// État d'un dossier, le même sur toutes les pages (SPEC-navette §1). Absent
/// (`null`) : le dossier n'a pas d'état (RM-04).
#[derive(Debug, Serialize)]
pub struct LawStateDto {
    /// `in_shuttle`, `definitively_adopted` ou `became_law`.
    pub kind: &'static str,
    pub date: NaiveDate,
    pub last_decision: Option<LastDecisionDto>,
    /// Devenu loi seulement (RM-01), repris du sort promulgué.
    pub law_code: Option<String>,
    pub jo_date: Option<NaiveDate>,
    pub legifrance_url: Option<String>,
}

impl From<(&LawStateSummary, &DossierOutcome)> for LawStateDto {
    fn from((summary, outcome): (&LawStateSummary, &DossierOutcome)) -> Self {
        let publication = match outcome {
            DossierOutcome::Promulgated { publication, .. } => Some(publication),
            _ => None,
        };
        Self {
            kind: summary.kind().as_str(),
            date: summary.date(),
            last_decision: summary.last_decision().map(|d| LastDecisionDto {
                chamber: chamber_code(d.chamber),
                sense: d.sense.as_str(),
                date: d.date,
            }),
            law_code: publication.and_then(|p| p.law_code.clone()),
            jo_date: publication.and_then(|p| p.jo_date),
            legifrance_url: publication.and_then(|p| p.legifrance_url.clone()),
        }
    }
}

/// État d'un dossier tel que son résumé le porte (calculé ou relu en base).
impl From<&LegislativeDossier> for Option<LawStateDto> {
    fn from(dossier: &LegislativeDossier) -> Self {
        dossier
            .law_state()
            .map(|summary| LawStateDto::from((summary, dossier.outcome())))
    }
}

/// Compteurs des trois états sur le périmètre affiché (SPEC-navette Q2).
#[derive(Debug, Serialize)]
pub struct LawStateCountsDto {
    pub in_shuttle: i64,
    pub definitively_adopted: i64,
    pub became_law: i64,
}

impl From<LawStateCounts> for LawStateCountsDto {
    fn from(counts: LawStateCounts) -> Self {
        Self {
            in_shuttle: counts.in_shuttle,
            definitively_adopted: counts.definitively_adopted,
            became_law: counts.became_law,
        }
    }
}

/// Un jalon de RM-05 ; les sous-titres sont composés par l'interface.
#[derive(Debug, Serialize)]
pub struct MilestoneDto {
    /// `chamber`, `agreement` ou `law`.
    pub step: &'static str,
    /// `an` ou `sn` pour une étape de chambre, absent sinon.
    pub chamber: Option<&'static str>,
    /// `done`, `current` ou `upcoming`.
    pub status: &'static str,
    pub sense: Option<&'static str>,
    pub date: Option<NaiveDate>,
    /// Jalon Accord : `same_text`, `joint_committee` ou `final_reading`.
    pub agreement: Option<&'static str>,
    /// Jalon Loi d'un dossier devenu loi.
    pub law_code: Option<String>,
    pub jo_date: Option<NaiveDate>,
    /// Fiche de vote : l'étape que ce scrutin a tranchée (« · ce vote »).
    pub is_this_vote: bool,
}

impl From<(&Milestone, bool)> for MilestoneDto {
    fn from((milestone, is_this_vote): (&Milestone, bool)) -> Self {
        let (step, chamber) = match milestone.step {
            MilestoneStep::Chamber(chamber) => ("chamber", chamber_code(chamber)),
            MilestoneStep::Agreement => ("agreement", None),
            MilestoneStep::Law => ("law", None),
        };
        Self {
            step,
            chamber,
            status: match milestone.status {
                MilestoneStatus::Done => "done",
                MilestoneStatus::Current => "current",
                MilestoneStatus::Upcoming => "upcoming",
            },
            sense: milestone.sense.map(|sense| sense.as_str()),
            date: milestone.date,
            agreement: milestone.agreement.map(|agreement| agreement.as_str()),
            law_code: milestone
                .publication
                .as_ref()
                .and_then(|p| p.law_code.clone()),
            jo_date: milestone.publication.as_ref().and_then(|p| p.jo_date),
            is_this_vote,
        }
    }
}

/// Encadré « Ce texte est-il devenu loi ? » (RM-06).
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct LawAnswerDto {
    /// `not_yet` (« Pas encore. »), `soon` (« Bientôt. ») ou `yes` (« Oui. »).
    pub kind: &'static str,
    /// Une seule phrase, sans le lien Légifrance que l'interface ajoute.
    pub sentence: String,
}

impl From<&LawProgress> for Option<LawAnswerDto> {
    fn from(progress: &LawProgress) -> Self {
        let (kind, sentence) = match progress.state()? {
            LawState::BecameLaw { date, publication } => {
                let mut sentence = match &publication.law_code {
                    Some(code) => format!("Loi n\u{b0} {code} du {}", french_date(*date)),
                    None => format!("Loi du {}", french_date(*date)),
                };
                if let Some(jo_date) = publication.jo_date {
                    sentence.push_str(&format!(
                        ", publi\u{e9}e au Journal officiel du {}",
                        french_date(jo_date)
                    ));
                }
                sentence.push('.');
                ("yes", sentence)
            }
            LawState::DefinitivelyAdopted { date, .. } => (
                "soon",
                format!(
                    "Adopt\u{e9} d\u{e9}finitivement le {}, il deviendra loi \u{e0} sa promulgation.",
                    french_date(*date)
                ),
            ),
            LawState::InShuttle { last_decision } => (
                "not_yet",
                shuttle_sentence(last_decision, progress.decisions()),
            ),
        };
        Some(LawAnswerDto { kind, sentence })
    }
}

/// Sujet d'une phrase : « L'Assemblée », « Le Sénat ».
fn subject(chamber: Chamber) -> Option<&'static str> {
    match chamber {
        Chamber::AssembleeNationale => Some("L'Assembl\u{e9}e"),
        Chamber::Senat => Some("Le S\u{e9}nat"),
        Chamber::Joint | Chamber::None => None,
    }
}

/// Complément du vote restant : « de l'Assemblée », « du Sénat ».
fn of_chamber(chamber: Chamber) -> &'static str {
    match chamber {
        Chamber::Senat => "du S\u{e9}nat",
        _ => "de l'Assembl\u{e9}e",
    }
}

/// Destination du texte : « à l'Assemblée », « au Sénat ».
fn to_chamber(chamber: Chamber) -> &'static str {
    match chamber {
        Chamber::Senat => "au S\u{e9}nat",
        _ => "\u{e0} l'Assembl\u{e9}e",
    }
}

fn other(chamber: Chamber) -> Chamber {
    match chamber {
        Chamber::Senat => Chamber::AssembleeNationale,
        _ => Chamber::Senat,
    }
}

fn participle(sense: DecisionSense) -> &'static str {
    match sense {
        DecisionSense::Modified => "modifi\u{e9}",
        DecisionSense::Rejected | DecisionSense::CmpDisagreement => "rejet\u{e9}",
        DecisionSense::Adopted | DecisionSense::AdoptedUnchanged | DecisionSense::CmpAgreement => {
            "adopt\u{e9}"
        }
    }
}

fn ordinal(reading: usize) -> String {
    match reading {
        2 => "deuxi\u{e8}me".into(),
        3 => "troisi\u{e8}me".into(),
        4 => "quatri\u{e8}me".into(),
        n => format!("{n}e"),
    }
}

/// Phrase d'un texte en navette (RM-06), selon sa dernière décision.
fn shuttle_sentence(last: &Decision, decisions: &[Decision]) -> String {
    let date = french_date(last.date);
    let agreement_date = || {
        decisions
            .iter()
            .filter(|d| {
                d.stage == DecisionStage::JointCommittee
                    && d.sense == DecisionSense::CmpAgreement
                    && d.date <= last.date
            })
            .map(|d| d.date)
            .max()
    };
    match (last.stage, subject(last.chamber)) {
        (DecisionStage::JointCommittee, _) => match last.sense {
            DecisionSense::CmpDisagreement => format!(
                "D\u{e9}saccord en commission mixte paritaire le {date} : le texte repart en nouvelle lecture."
            ),
            _ => format!(
                "Accord en commission mixte paritaire le {date} : reste le vote des deux chambres."
            ),
        },
        (DecisionStage::JointCommitteeReading, Some(subject)) if last.sense.adopts() => {
            match agreement_date() {
                Some(agreement) => format!(
                    "Accord en commission mixte paritaire le {} : reste le vote {}.",
                    french_date(agreement),
                    of_chamber(other(last.chamber))
                ),
                None => format!(
                    "{subject} a adopt\u{e9} le texte de la commission mixte paritaire le {date} : reste le vote {}.",
                    of_chamber(other(last.chamber))
                ),
            }
        }
        (DecisionStage::JointCommitteeReading, Some(subject)) => format!(
            "{subject} a rejet\u{e9} le texte de la commission mixte paritaire le {date}."
        ),
        (DecisionStage::JointCommitteeReading, None) => format!(
            "Le texte de la commission mixte paritaire a \u{e9}t\u{e9} {} le {date}.",
            participle(last.sense)
        ),
        (DecisionStage::FinalReading, _) => format!(
            "L'Assembl\u{e9}e l'a {} en lecture d\u{e9}finitive le {date}.",
            participle(last.sense)
        ),
        (DecisionStage::ChamberReading, Some(subject)) => format!(
            "{subject} l'a {} le {date} : {}.",
            participle(last.sense),
            next_reading(last, decisions)
        ),
        (DecisionStage::ChamberReading, None) => {
            format!("Derni\u{e8}re d\u{e9}cision le {date}.")
        }
    }
}

/// Suite de la navette après la lecture d'une chambre (RM-06) : l'autre
/// chambre l'examine pour la première fois, ou il lui revient pour une
/// n-ième lecture ; après un désaccord en CMP, en nouvelle lecture, puis en
/// lecture définitive une fois la nouvelle lecture du Sénat faite.
fn next_reading(last: &Decision, decisions: &[Decision]) -> String {
    let next = other(last.chamber);
    let disagreement = decisions.iter().any(|d| {
        d.stage == DecisionStage::JointCommittee && d.sense == DecisionSense::CmpDisagreement
    });
    if disagreement {
        let reading = if LegislativeStage::from_code(last.reading())
            == Some(LegislativeStage::NewReadingSenate)
        {
            "une lecture d\u{e9}finitive"
        } else {
            "une nouvelle lecture"
        };
        return format!("il revient {} pour {reading}", to_chamber(next));
    }
    let mut readings: Vec<&str> = decisions
        .iter()
        .filter(|d| d.stage == DecisionStage::ChamberReading && d.chamber == next)
        .map(Decision::reading)
        .collect();
    readings.dedup();
    if readings.is_empty() {
        let examiner = match next {
            Chamber::Senat => "le S\u{e9}nat",
            _ => "l'Assembl\u{e9}e",
        };
        return format!("{examiner} doit encore l'examiner");
    }
    format!(
        "il revient {} pour une {} lecture",
        to_chamber(next),
        ordinal(readings.len() + 1)
    )
}

/// Où en est la loi pour un dossier lu avec ses actes : état, jalons et
/// réponse, tous tirés d'un seul `LawProgress`. Sur la fiche de vote, le
/// scrutin marque le jalon qu'il a tranché.
#[derive(Debug, Serialize)]
pub struct LawProgressDto {
    pub law_state: Option<LawStateDto>,
    pub milestones: Vec<MilestoneDto>,
    pub answer: Option<LawAnswerDto>,
}

impl From<(&LegislativeDossier, Option<&ScrutinUid>)> for LawProgressDto {
    fn from((dossier, vote): (&LegislativeDossier, Option<&ScrutinUid>)) -> Self {
        let progress = dossier.law_progress();
        let vote_step = vote.and_then(|vote| step_of_vote(dossier.acts(), vote));
        Self {
            law_state: LawStateSummary::from_progress(&progress)
                .map(|summary| LawStateDto::from((&summary, dossier.outcome()))),
            milestones: progress
                .milestones()
                .iter()
                .map(|m| MilestoneDto::from((m, vote_step == Some(m.step))))
                .collect(),
            answer: Option::<LawAnswerDto>::from(&progress),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::application::use_cases::read_fakes::listed_dossier;
    use crate::domain::dossier::{LawPublication, LegislativeAct, NewLegislativeDossier};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn act(code: &str, on: NaiveDate) -> LegislativeAct {
        LegislativeAct {
            date: on,
            label: code.into(),
            code: Some(code.into()),
            decision_vote: None,
            conclusion_label: None,
            conclusion_code: None,
        }
    }

    fn decision(code: &str, on: NaiveDate, label: &str) -> LegislativeAct {
        LegislativeAct {
            conclusion_label: Some(label.into()),
            ..act(code, on)
        }
    }

    fn promulgated(on: NaiveDate, law_code: &str, jo: NaiveDate) -> DossierOutcome {
        DossierOutcome::Promulgated {
            date: on,
            publication: LawPublication {
                law_code: Some(law_code.into()),
                jo_date: Some(jo),
                legifrance_url: Some("https://www.legifrance.gouv.fr/jorf/id/JORFTEXT1".into()),
            },
        }
    }

    fn answer(acts: &[LegislativeAct], outcome: &DossierOutcome) -> Option<LawAnswerDto> {
        Option::<LawAnswerDto>::from(&LawProgress::from_acts(acts, outcome))
    }

    fn sentence(acts: &[LegislativeAct]) -> String {
        answer(acts, &DossierOutcome::NoRecordedConclusion)
            .unwrap()
            .sentence
    }

    fn dossier(acts: Vec<LegislativeAct>, outcome: DossierOutcome) -> LegislativeDossier {
        let base = listed_dossier("DLR5L17N51037", outcome, None);
        LegislativeDossier::new(NewLegislativeDossier {
            uid: base.uid().clone(),
            title: base.title().into(),
            procedure: base.procedure().into(),
            legislature: base.legislature(),
            url: None,
            summary: None,
            deposit_date: None,
            last_activity_date: base.last_activity_date(),
            last_activity_label: base.last_activity_label().into(),
            acts,
            documents: vec![],
            score: base.score().clone(),
            current_stage: None,
            initiators: vec![],
            committee: None,
            outcome: base.outcome().clone(),
        })
    }

    fn self_defence() -> Vec<LegislativeAct> {
        vec![
            act("AN1-DEPOT", date(2024, 12, 3)),
            decision("AN1-DEBATS-DEC", date(2026, 7, 7), "adopt\u{e9}e"),
            decision("SN1-DEBATS-DEC", date(2026, 11, 20), "modifi\u{e9}e"),
        ]
    }

    #[test]
    fn dates_are_written_in_full_with_the_first_of_the_month() {
        assert_eq!(french_date(date(2026, 11, 20)), "20 novembre 2026");
        assert_eq!(french_date(date(2026, 10, 1)), "1er octobre 2026");
        assert_eq!(french_date(date(2026, 8, 3)), "3 ao\u{fb}t 2026");
    }

    // Témoin DLR5L17N51037 (légitime défense).
    #[test]
    fn a_text_modified_by_the_senate_returns_to_the_assembly_for_a_second_reading() {
        let answer = answer(&self_defence(), &DossierOutcome::NoRecordedConclusion).unwrap();

        assert_eq!(answer.kind, "not_yet");
        assert_eq!(
            answer.sentence,
            "Le S\u{e9}nat l'a modifi\u{e9} le 20 novembre 2026 : il revient \u{e0} l'Assembl\u{e9}e pour une deuxi\u{e8}me lecture."
        );
    }

    // Témoin DLR5L17N51962 (Territoires zéro chômeur).
    #[test]
    fn a_definitively_adopted_text_will_become_law_at_its_promulgation() {
        let acts = [
            act("AN1-DEPOT", date(2025, 4, 17)),
            decision("AN1-DEBATS-DEC", date(2026, 1, 27), "adopt\u{e9}e"),
            decision("SN1-DEBATS-DEC", date(2026, 6, 9), "modifi\u{e9}e"),
            decision(
                "AN2-DEBATS-DEC",
                date(2026, 11, 19),
                "adopt\u{e9}e sans modification",
            ),
        ];

        let answer = answer(&acts, &DossierOutcome::NoRecordedConclusion).unwrap();

        assert_eq!(answer.kind, "soon");
        assert_eq!(
            answer.sentence,
            "Adopt\u{e9} d\u{e9}finitivement le 19 novembre 2026, il deviendra loi \u{e0} sa promulgation."
        );
    }

    // Témoin DLR5L17N54096 (CREPS de Vichy).
    #[test]
    fn a_text_that_became_law_gives_its_number_and_its_official_journal() {
        let acts = [
            act("SN1-DEPOT", date(2026, 3, 1)),
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adopt\u{e9}e"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 10, 12),
                "adopt\u{e9}e sans modification",
            ),
            act("PROM-PUB", date(2026, 10, 26)),
        ];
        let outcome = promulgated(date(2026, 10, 26), "2026-992", date(2026, 10, 27));

        let answer = answer(&acts, &outcome).unwrap();

        assert_eq!(answer.kind, "yes");
        assert_eq!(
            answer.sentence,
            "Loi n\u{b0} 2026-992 du 26 octobre 2026, publi\u{e9}e au Journal officiel du 27 octobre 2026."
        );
    }

    #[test]
    fn a_text_adopted_by_the_assembly_awaits_the_senate() {
        let acts = &self_defence()[..2];

        assert_eq!(
            sentence(acts),
            "L'Assembl\u{e9}e l'a adopt\u{e9} le 7 juillet 2026 : le S\u{e9}nat doit encore l'examiner."
        );
    }

    #[test]
    fn a_joint_committee_agreement_read_by_one_chamber_awaits_the_other() {
        let mut acts = self_defence();
        acts.push(decision("CMP-DEC", date(2026, 12, 2), "Accord"));
        acts.push(decision(
            "CMP-DEBATS-SN-DEC",
            date(2026, 12, 9),
            "adopt\u{e9}e",
        ));

        assert_eq!(
            sentence(&acts),
            "Accord en commission mixte paritaire le 2 d\u{e9}cembre 2026 : reste le vote de l'Assembl\u{e9}e."
        );
    }

    #[test]
    fn a_joint_committee_agreement_alone_awaits_both_chambers() {
        let mut acts = self_defence();
        acts.push(decision("CMP-DEC", date(2026, 12, 2), "Accord"));

        assert_eq!(
            sentence(&acts),
            "Accord en commission mixte paritaire le 2 d\u{e9}cembre 2026 : reste le vote des deux chambres."
        );
    }

    #[test]
    fn after_a_joint_committee_disagreement_the_text_goes_to_a_new_reading() {
        let mut acts = self_defence();
        acts.push(decision("CMP-DEC", date(2026, 12, 2), "D\u{e9}saccord"));

        assert_eq!(
            sentence(&acts),
            "D\u{e9}saccord en commission mixte paritaire le 2 d\u{e9}cembre 2026 : le texte repart en nouvelle lecture."
        );

        acts.push(decision(
            "ANNLEC-DEBATS-DEC",
            date(2026, 12, 10),
            "adopt\u{e9}e",
        ));
        assert_eq!(
            sentence(&acts),
            "L'Assembl\u{e9}e l'a adopt\u{e9} le 10 d\u{e9}cembre 2026 : il revient au S\u{e9}nat pour une nouvelle lecture."
        );

        acts.push(decision(
            "SNNLEC-DEBATS-DEC",
            date(2026, 12, 15),
            "rejet\u{e9}e",
        ));
        assert_eq!(
            sentence(&acts),
            "Le S\u{e9}nat l'a rejet\u{e9} le 15 d\u{e9}cembre 2026 : il revient \u{e0} l'Assembl\u{e9}e pour une lecture d\u{e9}finitive."
        );
    }

    #[test]
    fn an_adopted_final_reading_is_a_definitive_adoption() {
        let mut acts = self_defence();
        acts.push(decision("CMP-DEC", date(2026, 12, 2), "D\u{e9}saccord"));
        acts.push(decision(
            "ANLDEF-DEBATS-DEC",
            date(2026, 12, 18),
            "adopt\u{e9}e",
        ));

        let answer = answer(&acts, &DossierOutcome::NoRecordedConclusion).unwrap();

        assert_eq!(answer.kind, "soon");
        assert_eq!(
            answer.sentence,
            "Adopt\u{e9} d\u{e9}finitivement le 18 d\u{e9}cembre 2026, il deviendra loi \u{e0} sa promulgation."
        );
    }

    #[test]
    fn a_rejection_then_an_adoption_by_the_other_chamber_returns_for_a_second_reading() {
        let acts = [
            act("AN1-DEPOT", date(2026, 10, 7)),
            decision("AN1-DEBATS-DEC", date(2026, 11, 4), "rejet\u{e9}e"),
            decision("SN1-DEBATS-DEC", date(2026, 11, 26), "adopt\u{e9}e"),
        ];

        assert_eq!(
            sentence(&acts),
            "Le S\u{e9}nat l'a adopt\u{e9} le 26 novembre 2026 : il revient \u{e0} l'Assembl\u{e9}e pour une deuxi\u{e8}me lecture."
        );
    }

    #[test]
    fn a_third_reading_is_counted_from_the_readings_of_the_next_chamber() {
        let mut acts = self_defence();
        acts.push(decision(
            "AN2-DEBATS-DEC",
            date(2027, 1, 12),
            "modifi\u{e9}e",
        ));
        acts.push(decision(
            "SN2-DEBATS-DEC",
            date(2027, 2, 3),
            "modifi\u{e9}e",
        ));

        assert!(sentence(&acts).ends_with("pour une troisi\u{e8}me lecture."));
    }

    #[test]
    fn a_text_without_state_has_no_answer() {
        let acts = [decision("AN1-DEBATS-DEC", date(2026, 3, 1), "rejet\u{e9}e")];

        assert_eq!(answer(&acts, &DossierOutcome::NoRecordedConclusion), None);
    }

    #[test]
    fn law_state_of_a_promulgated_dossier_carries_its_publication() {
        let acts = vec![
            act("SN1-DEPOT", date(2026, 3, 1)),
            decision("SN1-DEBATS-DEC", date(2026, 6, 15), "adopt\u{e9}e"),
            act("PROM-PUB", date(2026, 10, 26)),
        ];
        let dossier = dossier(
            acts,
            promulgated(date(2026, 10, 26), "2026-992", date(2026, 10, 27)),
        );

        let json = serde_json::to_value(Option::<LawStateDto>::from(&dossier)).unwrap();

        assert_eq!(
            json,
            json!({
                "kind": "became_law",
                "date": "2026-10-26",
                "last_decision": { "chamber": "sn", "sense": "adopted", "date": "2026-06-15" },
                "law_code": "2026-992",
                "jo_date": "2026-10-27",
                "legifrance_url": "https://www.legifrance.gouv.fr/jorf/id/JORFTEXT1",
            })
        );
    }

    #[test]
    fn milestones_mark_the_step_decided_by_this_vote() {
        let vote = ScrutinUid::new("VTANR5L17V7987".into()).unwrap();
        let mut acts = self_defence();
        acts[1].decision_vote = Some(vote.clone());
        let dossier = dossier(acts, DossierOutcome::NoRecordedConclusion);

        let json: Value =
            serde_json::to_value(LawProgressDto::from((&dossier, Some(&vote)))).unwrap();

        assert_eq!(json["law_state"]["kind"], "in_shuttle");
        assert_eq!(json["answer"]["kind"], "not_yet");
        let milestones = json["milestones"].as_array().unwrap();
        assert_eq!(milestones.len(), 4);
        assert_eq!(
            milestones[0],
            json!({
                "step": "chamber",
                "chamber": "an",
                "status": "done",
                "sense": "adopted",
                "date": "2026-07-07",
                "agreement": null,
                "law_code": null,
                "jo_date": null,
                "is_this_vote": true,
            })
        );
        assert_eq!(milestones[1]["chamber"], "sn");
        assert_eq!(milestones[1]["status"], "current");
        assert_eq!(milestones[1]["is_this_vote"], false);
        assert_eq!(milestones[2]["step"], "agreement");
        assert_eq!(milestones[2]["status"], "upcoming");
        assert_eq!(milestones[3]["step"], "law");
    }

    #[test]
    fn a_law_milestone_carries_the_law_number_and_its_official_journal() {
        let acts = vec![
            act("SN1-DEPOT", date(2026, 3, 1)),
            decision("SN1-DEBATS-DEC", date(2026, 5, 28), "adopt\u{e9}e"),
            decision(
                "AN1-DEBATS-DEC",
                date(2026, 10, 12),
                "adopt\u{e9}e sans modification",
            ),
            act("PROM-PUB", date(2026, 10, 21)),
        ];
        let dossier = dossier(
            acts,
            promulgated(date(2026, 10, 21), "2026-975", date(2026, 10, 22)),
        );

        let json: Value = serde_json::to_value(LawProgressDto::from((&dossier, None))).unwrap();

        let milestones = json["milestones"].as_array().unwrap();
        assert_eq!(milestones[0]["chamber"], "sn");
        assert_eq!(milestones[2]["agreement"], "same_text");
        assert_eq!(milestones[2]["date"], "2026-10-12");
        assert_eq!(milestones[3]["law_code"], "2026-975");
        assert_eq!(milestones[3]["jo_date"], "2026-10-22");
        assert_eq!(json["answer"]["kind"], "yes");
    }

    #[test]
    fn a_dossier_without_state_serves_no_state_no_milestone_and_no_answer() {
        let dossier = dossier(
            vec![decision("AN1-DEBATS-DEC", date(2026, 3, 1), "rejet\u{e9}e")],
            DossierOutcome::NoRecordedConclusion,
        );

        let json: Value = serde_json::to_value(LawProgressDto::from((&dossier, None))).unwrap();

        assert!(json["law_state"].is_null());
        assert_eq!(json["milestones"], json!([]));
        assert!(json["answer"].is_null());
    }
}
