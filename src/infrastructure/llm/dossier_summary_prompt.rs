//! Faits, prompt, schéma et validation de la réponse des synthèses de dossier par
//! groupe, communs aux trois fournisseurs. Les adapters Anthropic, OpenAI et
//! Mistral n'en font que le transport.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::application::ports::dossier_summary_repository::GeneratedGroupSummary;
use crate::application::ports::dossier_summary_generator::SummaryGeneratorError;
use crate::domain::dossier_group_facts::DossierGroupFacts;
use crate::domain::dossier_summary::{SummaryParagraph, SummarySource};

/// Version du prompt, enregistrée avec chaque synthèse.
pub(crate) const PROMPT_VERSION: &str = "dossier-group-summary-v1";

/// Plafond de jetons de sortie d'une synthèse, commun aux trois fournisseurs.
pub(crate) const SUMMARY_MAX_OUTPUT_TOKENS: u32 = 2_400;

pub(crate) fn facts_input(facts: &DossierGroupFacts) -> Value {
    let groups: Vec<Value> = facts
        .groups
        .iter()
        .filter(|group| !group.final_votes.is_empty() || !group.amendments.is_empty())
        .map(|group| {
            json!({
                "group_uid": group.uid,
                "group_label": group.label,
                "group_abbrev": group.abbrev,
                "facts": {
                    "final_votes": group.final_votes.iter().map(|vote| json!({
                        "source_id": format!("scrutin:{}", vote.scrutin_uid),
                        "date": vote.date,
                        "reading": vote.reading,
                        "outcome": vote.outcome_label,
                        "text": vote.text_label,
                    })).collect::<Vec<_>>(),
                    "amendments": group.amendments.iter().map(|amendment| json!({
                        "source_id": format!("amendment:{}", amendment.uid),
                        "number": amendment.number,
                        "target": amendment.target_title,
                        "target_kind": amendment.target_kind,
                        "fate": amendment.fate_label,
                        "deposited_on": amendment.deposited_on,
                        "summary_available": amendment.summary_available,
                    })).collect::<Vec<_>>(),
                }
            })
        })
        .collect();

    json!({
        "title": facts.title,
        "sources": [{
            "source_id": format!("dossier:{}", facts.dossier_uid),
            "kind": "dossier"
        }],
        "groups": groups
    })
}

pub(crate) fn system_prompt() -> &'static str {
    "Tu decris des actes officiels rattaches a un dossier parlementaire. Tu reçois uniquement le titre du dossier, l'identite officielle des groupes, des objets de votes finaux, des objets d'amendements et des identifiants de sources. Tu ne reçois aucun expose sommaire complet, aucune declaration externe et aucun contexte politique. Rends un court paragraphe par groupe qui possede au moins un acte. Le texte doit decrire les textes, lectures, issues et objets d'actes sans attribuer de position globale, sans evaluation, comparaison, classement, causalite ou intention. Ne mets aucun chiffre ni nombre dans le paragraphe; les nombres et repartitions sont rendus par le code. Chaque groupe attendu doit apparaitre exactement une fois. Les source_ids doivent etre recopiees uniquement depuis les sources fournies. N'invente jamais une source."
}

pub(crate) fn response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "groups": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "group_uid": { "type": "string" },
                        "paragraph": { "type": "string", "minLength": 1, "maxLength": 900 },
                        "source_ids": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["group_uid", "paragraph", "source_ids"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["groups"],
        "additionalProperties": false
    })
}

#[derive(Debug, Deserialize)]
struct Answer {
    groups: Vec<AnswerGroup>,
}

#[derive(Debug, Deserialize)]
struct AnswerGroup {
    group_uid: String,
    paragraph: String,
    source_ids: Vec<String>,
}

fn source_catalog(facts: &DossierGroupFacts) -> HashMap<String, SummarySource> {
    let dossier_url = facts.official_url.clone().or_else(|| {
        Some(format!(
            "https://www.assemblee-nationale.fr/dyn/{}/dossiers/{}",
            facts.legislature, facts.dossier_uid
        ))
    });
    let mut sources = HashMap::new();
    let dossier_id = format!("dossier:{}", facts.dossier_uid);
    sources.insert(
        dossier_id.clone(),
        SummarySource {
            source_id: dossier_id,
            kind: "dossier".into(),
            uid: facts.dossier_uid.to_string(),
            label: "Dossier officiel".into(),
            official_url: dossier_url.clone(),
        },
    );
    for group in &facts.groups {
        for vote in &group.final_votes {
            let source_id = format!("scrutin:{}", vote.scrutin_uid);
            sources.insert(
                source_id.clone(),
                SummarySource {
                    source_id,
                    kind: "scrutin".into(),
                    uid: vote.scrutin_uid.to_string(),
                    label: format!("Scrutin n°{}", vote.number),
                    official_url: Some(format!(
                        "https://www.assemblee-nationale.fr/dyn/{}/scrutins/{}",
                        vote.legislature, vote.number
                    )),
                },
            );
        }
        for amendment in &group.amendments {
            let source_id = format!("amendment:{}", amendment.uid);
            sources.insert(
                source_id.clone(),
                SummarySource {
                    source_id,
                    kind: "amendment".into(),
                    uid: amendment.uid.to_string(),
                    label: format!("Amendement n°{} — dossier officiel", amendment.number),
                    // Le format d'une URL directe d'amendement n'est pas
                    // confirmé dans la source. Le dossier officiel reste la
                    // preuve navigable, sans fabriquer un lien mort.
                    official_url: dossier_url.clone(),
                },
            );
        }
    }
    sources
}

pub(crate) fn parse_text(
    text: &str,
    facts: &DossierGroupFacts,
) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
    let answer: Answer = serde_json::from_str(text)
        .map_err(|error| SummaryGeneratorError::Answer(format!("json invalide: {error}")))?;
    validate_answer(answer, facts)
}

fn validate_answer(
    answer: Answer,
    facts: &DossierGroupFacts,
) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
    let expected: HashSet<&str> = facts
        .groups
        .iter()
        .filter(|group| !group.final_votes.is_empty() || !group.amendments.is_empty())
        .map(|group| group.uid.as_str())
        .collect();
    let sources = source_catalog(facts);
    if answer.groups.len() != expected.len() {
        return Err(SummaryGeneratorError::Answer(
            "groupe manquant ou en trop".into(),
        ));
    }

    let mut seen = HashSet::new();
    let mut output = Vec::with_capacity(answer.groups.len());
    for group in answer.groups {
        if !expected.contains(group.group_uid.as_str()) || !seen.insert(group.group_uid.clone()) {
            return Err(SummaryGeneratorError::Answer(
                "groupe inconnu ou duplique".into(),
            ));
        }
        let paragraph = SummaryParagraph::new(group.paragraph)
            .map_err(|error| SummaryGeneratorError::Answer(error.to_string()))?;
        let mut selected = Vec::with_capacity(group.source_ids.len());
        let mut source_seen = HashSet::new();
        for source_id in group.source_ids {
            if !source_seen.insert(source_id.clone()) {
                continue;
            }
            let Some(source) = sources.get(&source_id) else {
                return Err(SummaryGeneratorError::Answer("source inconnue".into()));
            };
            selected.push(source.clone());
        }
        if selected.is_empty() {
            return Err(SummaryGeneratorError::Answer("aucune source citee".into()));
        }
        output.push(GeneratedGroupSummary {
            group_uid: group.group_uid,
            paragraph: paragraph.as_str().to_string(),
            sources: selected,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::actor::GroupUid;
    use crate::domain::amendment::AmendmentUid;
    use crate::domain::dossier::DossierUid;
    use crate::domain::dossier_group_facts::{AmendmentFact, GroupFacts};
    use chrono::NaiveDate;

    fn facts() -> DossierGroupFacts {
        DossierGroupFacts {
            dossier_uid: DossierUid::new("D1".to_string()).unwrap(),
            title: "Dossier".into(),
            official_url: Some("https://example.test/dossier".into()),
            legislature: 17,
            period_start: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            period_end: None,
            groups: vec![GroupFacts {
                uid: GroupUid::new("G1".to_string()).unwrap(),
                abbrev: "G".into(),
                label: "Groupe".into(),
                color: None,
                start_date: None,
                end_date: None,
                final_votes: vec![],
                amendments: vec![AmendmentFact {
                    uid: AmendmentUid::new("A1".to_string()).unwrap(),
                    number: "1".into(),
                    target_title: "Article".into(),
                    target_kind: None,
                    fate_code: "adopted".into(),
                    fate_label: "Adopte".into(),
                    deposited_on: None,
                    summary_available: true,
                }],
            }],
        }
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_text("{", &facts()).is_err());
    }

    #[test]
    fn rejects_missing_group() {
        assert!(parse_text(r#"{"groups":[]}"#, &facts()).is_err());
    }

    #[test]
    fn rejects_unknown_source() {
        let text = r#"{"groups":[{"group_uid":"G1","paragraph":"Une description factuelle.","source_ids":["scrutin:S1"]}]}"#;
        assert!(parse_text(text, &facts()).is_err());
    }

    #[test]
    fn rejects_numbers_and_positioning() {
        for paragraph in [
            "Le groupe compte 2 actes.",
            "Le groupe est favorable au texte.",
        ] {
            let text = json!({"groups":[{"group_uid":"G1","paragraph":paragraph,"source_ids":["amendment:A1"]}]}).to_string();
            assert!(parse_text(&text, &facts()).is_err());
        }
    }

    #[test]
    fn rejects_too_long_text() {
        let paragraph = "a".repeat(901);
        let text = json!({"groups":[{"group_uid":"G1","paragraph":paragraph,"source_ids":["amendment:A1"]}]}).to_string();
        assert!(parse_text(&text, &facts()).is_err());
    }
}
