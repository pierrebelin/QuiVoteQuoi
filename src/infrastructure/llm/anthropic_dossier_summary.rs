//! Generateur de paragraphes descriptifs, strictement borne par des faits.
//!
//! Faits, prompt, schéma et validation de la réponse sont communs aux trois
//! fournisseurs (`dossier_summary_prompt`): ce module n'en fait que le transport.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::dossier_summary_repository::GeneratedGroupSummary;
use crate::application::ports::dossier_summary_generator::{
    DossierSummaryGenerator, SummaryGeneratorError,
};
use crate::domain::dossier_group_facts::DossierGroupFacts;

use super::dossier_summary_prompt::{
    facts_input, parse_text, response_schema, system_prompt, PROMPT_VERSION,
    SUMMARY_MAX_OUTPUT_TOKENS,
};
use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT, MAX_ATTEMPTS};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-opus-5";

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
    use crate::domain::dossier::DossierUid;

    #[test]
    fn the_text_block_of_the_message_is_read() {
        let facts = DossierGroupFacts {
            dossier_uid: DossierUid::new("D1".to_string()).unwrap(),
            title: "Dossier".into(),
            official_url: None,
            legislature: 17,
            period_start: None,
            period_end: None,
            groups: vec![],
        };
        let payload = json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": r#"{"groups":[]}"# }]
        });
        assert!(parse_answer(&payload, &facts).unwrap().is_empty());
    }
}
