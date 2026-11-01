//! Generateur de syntheses via l'API OpenAI Responses.
//!
//! Le contrat est le meme que pour Claude: les faits et la validation sont
//! partages, seul le transport fournisseur change.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::dossier_group_actions_repository::{
    DossierGroupFacts, GeneratedGroupSummary,
};
use crate::application::ports::dossier_summary_generator::{
    DossierSummaryGenerator, SummaryGeneratorError,
};

use super::anthropic_dossier_summary::{
    facts_input, parse_text, response_schema, system_prompt, SUMMARY_MAX_OUTPUT_TOKENS,
};
use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT};
use super::openai_support::output_text;

const API_URL: &str = "https://api.openai.com/v1/responses";
const DEFAULT_MODEL: &str = "gpt-5.6-luna";
const PROMPT_VERSION: &str = "dossier-group-summary-v1";
const MAX_ATTEMPTS: u32 = 3;

pub struct OpenAiDossierSummaryGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiDossierSummaryGenerator {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let model = std::env::var("DOSSIER_SUMMARY_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("OPENAI_MODEL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
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
            "store": false,
            "max_output_tokens": SUMMARY_MAX_OUTPUT_TOKENS,
            "input": [
                {
                    "role": "developer",
                    "content": [{ "type": "input_text", "text": system_prompt() }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": facts_input(facts).to_string()
                    }]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "dossier_group_summary",
                    "strict": true,
                    "schema": response_schema()
                },
                "verbosity": "low"
            }
        })
    }
}

#[async_trait]
impl DossierSummaryGenerator for OpenAiDossierSummaryGenerator {
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
            let text = output_text(&payload).map_err(SummaryGeneratorError::Answer)?;
            return parse_text(text, facts);
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

    #[test]
    fn request_uses_strict_structured_output_without_storage() {
        let generator = OpenAiDossierSummaryGenerator {
            http: reqwest::Client::new(),
            api_key: "test-key".into(),
            model: "gpt-test".into(),
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
        assert_eq!(body["store"], false);
        assert_eq!(body["text"]["format"]["type"], "json_schema");
        assert_eq!(body["text"]["format"]["strict"], true);
    }
}
