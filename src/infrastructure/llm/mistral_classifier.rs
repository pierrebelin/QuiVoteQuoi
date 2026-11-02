//! Classifieur thematique via l'API Mistral Chat Completions.
//!
//! Meme cadrage, meme schema et meme validation que Claude et OpenAI: seul le
//! transport change. La temperature est nulle pour qu'un meme libelle rende la
//! meme proposition d'une passe a l'autre.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::domain::theme::ProposedFamily;

use super::anthropic_classifier::{numbered_labels, parse_text, response_schema, system_prompt};
use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT};
use super::mistral_support::message_content;

const API_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_MODEL: &str = "mistral-small-latest";
const PROMPT_VERSION: &str = "thematisation-v3";
const BATCH_SIZE: usize = 20;
const MAX_ATTEMPTS: u32 = 3;
const TOKENS_PER_LABEL: usize = 400;
const TOKENS_FLOOR: usize = 2_000;

pub struct MistralThemeClassifier {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl MistralThemeClassifier {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("MISTRAL_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let model = std::env::var("THEME_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("MISTRAL_MODEL")
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

    fn request_body(&self, labels: &[String]) -> Value {
        json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": TOKENS_FLOOR + TOKENS_PER_LABEL * labels.len(),
            "messages": [
                { "role": "system", "content": system_prompt() },
                { "role": "user", "content": numbered_labels(labels) }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "theme_classification",
                    "strict": true,
                    "schema": response_schema(labels.len())
                }
            }
        })
    }
}

#[async_trait]
impl ThemeClassifier for MistralThemeClassifier {
    async fn propose_batch(
        &self,
        labels: &[String],
    ) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
        if labels.is_empty() {
            return Ok(vec![]);
        }

        let body = self.request_body(labels);
        let mut last_error = ClassifierError::Call("aucune tentative".into());
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
                    last_error = ClassifierError::Call(error.to_string());
                    backoff(attempt).await;
                    continue;
                }
            };
            let status = response.status();
            if is_retryable(status) {
                last_error = ClassifierError::Call(format!("HTTP {status}"));
                backoff(attempt).await;
                continue;
            }
            if !status.is_success() {
                return Err(ClassifierError::Call(format!(
                    "HTTP {status}: {}",
                    truncate_detail(&response.text().await.unwrap_or_default())
                )));
            }
            let payload: Value = response
                .json()
                .await
                .map_err(|error| ClassifierError::Answer(error.to_string()))?;
            let text = message_content(&payload).map_err(ClassifierError::Answer)?;
            return parse_text(&text, labels.len());
        }

        Err(last_error)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn prompt_version(&self) -> &str {
        PROMPT_VERSION
    }

    fn batch_size(&self) -> usize {
        BATCH_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_strict_schema_and_zero_temperature() {
        let classifier = MistralThemeClassifier {
            http: reqwest::Client::new(),
            api_key: "test-key".into(),
            model: "mistral-test".into(),
        };
        let body = classifier.request_body(&["un texte".into()]);
        assert_eq!(body["temperature"], 0);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    }
}
