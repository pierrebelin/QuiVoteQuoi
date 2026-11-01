//! Generateur de syntheses via l'API Mistral Chat Completions.
//!
//! Le contrat est le meme que pour Claude: les faits et la validation sont
//! partages, seul le transport fournisseur change.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::dossier_group_actions_repository::{
    DossierGroupFacts, GeneratedGroupSummary,
};
use crate::application::ports::dossier_summary_generator::{
    DossierSummaryGenerator, SummaryGeneratorError,
};

use super::anthropic_dossier_summary::{facts_input, parse_text, response_schema, system_prompt};
use super::mistral_support::message_content;

const API_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_MODEL: &str = "mistral-small-latest";
const PROMPT_VERSION: &str = "dossier-group-summary-v1";
const MAX_ATTEMPTS: u32 = 3;

pub struct MistralDossierSummaryGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl MistralDossierSummaryGenerator {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("MISTRAL_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let model = std::env::var("DOSSIER_SUMMARY_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("MISTRAL_MODEL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
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
            "temperature": 0,
            "max_tokens": 2400,
            "messages": [
                { "role": "system", "content": system_prompt() },
                { "role": "user", "content": facts_input(facts).to_string() }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "dossier_group_summary",
                    "strict": true,
                    "schema": response_schema()
                }
            }
        })
    }
}

#[async_trait]
impl DossierSummaryGenerator for MistralDossierSummaryGenerator {
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
                .bearer_auth(&self.api_key)
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
            if status.as_u16() == 429 || status.is_server_error() {
                last_error = SummaryGeneratorError::Call(format!("HTTP {status}"));
                backoff(attempt).await;
                continue;
            }
            if !status.is_success() {
                return Err(SummaryGeneratorError::Call(format!(
                    "HTTP {status}: {}",
                    response
                        .text()
                        .await
                        .unwrap_or_default()
                        .chars()
                        .take(300)
                        .collect::<String>()
                )));
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|error| SummaryGeneratorError::Answer(error.to_string()))?;
            let text = message_content(&payload).map_err(SummaryGeneratorError::Answer)?;
            return parse_text(&text, facts);
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

async fn backoff(attempt: u32) {
    tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_strict_schema_and_zero_temperature() {
        let generator = MistralDossierSummaryGenerator {
            http: reqwest::Client::new(),
            api_key: "test-key".into(),
            model: "mistral-test".into(),
        };
        let facts = DossierGroupFacts {
            dossier_uid: "D1".into(),
            title: "Dossier".into(),
            official_url: None,
            legislature: 17,
            period_start: None,
            period_end: None,
            groups: vec![],
        };
        let body = generator.request_body(&facts);
        assert_eq!(body["temperature"], 0);
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    }
}
