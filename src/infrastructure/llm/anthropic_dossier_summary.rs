//! Generateur de paragraphes descriptifs, strictement borne par des faits.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::application::ports::dossier_group_actions_repository::GeneratedGroupSummary;
use crate::application::ports::dossier_summary_generator::{
    DossierSummaryGenerator, SummaryGeneratorError,
};
use crate::domain::dossier_group_facts::DossierGroupFacts;
use crate::domain::dossier_summary::{SummaryParagraph, SummarySource};

use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-opus-5";
const PROMPT_VERSION: &str = "dossier-group-summary-v1";
const MAX_ATTEMPTS: u32 = 3;

/// Plafond de jetons de sortie d'une synthèse, commun aux trois fournisseurs.
pub(crate) const SUMMARY_MAX_OUTPUT_TOKENS: u32 = 2_400;

pub struct AnthropicDossierSummaryGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicDossierSummaryGenerator {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())?;
        let model = std::env::var("DOSSIER_SUMMARY_MODEL")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("ANTHROPIC_MODEL")
                    .ok()
                    .filter(|v| !v.is_empty())
            })
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .ok()?;
        Some(Self {
            http,
            api_key,
            model,
        })
    }

    fn request_body(&self, facts: &DossierGroupFacts) -> Value {
        json!({
            "model": self.model,
            "max_tokens": SUMMARY_MAX_OUTPUT_TOKENS,
            "output_config": {
                "effort": "low",
                "format": { "type": "json_schema", "schema": response_schema() }
            },
            "system": [{
                "type": "text",
                "text": system_prompt(),
                "cache_control": { "type": "ephemeral" }
            }],
            "messages": [{
                "role": "user",
                "content": facts_input(facts).to_string()
            }]
        })
    }
}

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
            uid: facts.dossier_uid.clone(),
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
                    uid: vote.scrutin_uid.clone(),
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
                    uid: amendment.uid.clone(),
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

fn parse_answer(
    payload: &Value,
    facts: &DossierGroupFacts,
) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
    if matches!(
        payload.get("stop_reason").and_then(Value::as_str),
        Some("max_tokens" | "refusal")
    ) {
        return Err(SummaryGeneratorError::Answer(
            "reponse tronquee ou refusee".into(),
        ));
    }
    let text = payload
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        })
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| SummaryGeneratorError::Answer("aucun bloc de texte".into()))?;
    parse_text(text, facts)
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

#[async_trait]
impl DossierSummaryGenerator for AnthropicDossierSummaryGenerator {
    async fn generate(
        &self,
        facts: &DossierGroupFacts,
    ) -> Result<Vec<GeneratedGroupSummary>, SummaryGeneratorError> {
        let body = self.request_body(facts);
        let mut last_error = SummaryGeneratorError::Call("aucune tentative".into());
        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .http
                .post(API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", API_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await;
            let response = match response {
                Ok(value) => value,
                Err(error) => {
                    last_error = SummaryGeneratorError::Call(error.to_string());
                    backoff(attempt).await;
                    continue;
                }
            };
            let status = response.status();
            if is_retryable(status) {
                last_error = SummaryGeneratorError::Call(format!("HTTP {status}"));
                backoff(attempt).await;
                continue;
            }
            if !status.is_success() {
                return Err(SummaryGeneratorError::Call(format!(
                    "HTTP {status}: {}",
                    truncate_detail(&response.text().await.unwrap_or_default())
                )));
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|error| SummaryGeneratorError::Answer(error.to_string()))?;
            return parse_answer(&payload, facts);
        }
        Err(last_error)
    }

    fn model(&self) -> &str {
        &self.model
    }
    fn prompt_version(&self) -> &str {
        PROMPT_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dossier_group_facts::{AmendmentFact, GroupFacts};
    use chrono::NaiveDate;

    fn facts() -> DossierGroupFacts {
        DossierGroupFacts {
            dossier_uid: "D1".into(),
            title: "Dossier".into(),
            official_url: Some("https://example.test/dossier".into()),
            legislature: 17,
            period_start: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            period_end: None,
            groups: vec![GroupFacts {
                uid: "G1".into(),
                abbrev: "G".into(),
                label: "Groupe".into(),
                color: None,
                start_date: None,
                end_date: None,
                final_votes: vec![],
                amendments: vec![AmendmentFact {
                    uid: "A1".into(),
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

    fn payload(text: &str) -> Value {
        json!({"stop_reason":"end_turn","content":[{"type":"text","text":text}]})
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_answer(&payload("{"), &facts()).is_err());
    }

    #[test]
    fn rejects_missing_group() {
        let value = payload(r#"{"groups":[]}"#);
        assert!(parse_answer(&value, &facts()).is_err());
    }

    #[test]
    fn rejects_unknown_source() {
        let value = payload(
            r#"{"groups":[{"group_uid":"G1","paragraph":"Une description factuelle.","source_ids":["scrutin:S1"]}]}"#,
        );
        assert!(parse_answer(&value, &facts()).is_err());
    }

    #[test]
    fn rejects_numbers_and_positioning() {
        for paragraph in [
            "Le groupe compte 2 actes.",
            "Le groupe est favorable au texte.",
        ] {
            let value = payload(&json!({"groups":[{"group_uid":"G1","paragraph":paragraph,"source_ids":["amendment:A1"]}]}).to_string());
            assert!(parse_answer(&value, &facts()).is_err());
        }
    }

    #[test]
    fn rejects_too_long_text() {
        let paragraph = "a".repeat(901);
        let value = payload(&json!({"groups":[{"group_uid":"G1","paragraph":paragraph,"source_ids":["amendment:A1"]}]}).to_string());
        assert!(parse_answer(&value, &facts()).is_err());
    }
}
