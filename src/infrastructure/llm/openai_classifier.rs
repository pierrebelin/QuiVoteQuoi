//! Classifieur thematique via l'API OpenAI Responses.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::domain::theme::ProposedFamily;

use super::anthropic_classifier::{numbered_labels, parse_text, response_schema, system_prompt};
use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT};
use super::openai_support::output_text;

const API_URL: &str = "https://api.openai.com/v1/responses";
const DEFAULT_MODEL: &str = "gpt-5.6-luna";
const PROMPT_VERSION: &str = "thematisation-v3";
const BATCH_SIZE: usize = 20;
const MAX_ATTEMPTS: u32 = 3;
const TOKENS_PER_LABEL: usize = 400;
const TOKENS_FLOOR: usize = 2_000;

pub struct OpenAiThemeClassifier {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiThemeClassifier {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let model = std::env::var("THEME_MODEL")
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

    fn request_body(&self, labels: &[String]) -> Value {
        json!({
            "model": self.model,
            "store": false,
            "max_output_tokens": TOKENS_FLOOR + TOKENS_PER_LABEL * labels.len(),
            "input": [
                {
                    "role": "developer",
                    "content": [{ "type": "input_text", "text": system_prompt() }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": numbered_labels(labels)
                    }]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "theme_classification",
                    "strict": true,
                    "schema": response_schema(labels.len())
                },
                "verbosity": "low"
            }
        })
    }
}

#[async_trait]
impl ThemeClassifier for OpenAiThemeClassifier {
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
            let text = output_text(&payload).map_err(ClassifierError::Answer)?;
            return parse_text(text, labels.len());
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
    fn request_uses_strict_structured_output_without_storage() {
        let classifier = OpenAiThemeClassifier {
            http: reqwest::Client::new(),
            api_key: "test-key".into(),
            model: "gpt-test".into(),
        };
        let body = classifier.request_body(&["un texte".into()]);
        assert_eq!(body["store"], false);
        assert_eq!(body["text"]["format"]["type"], "json_schema");
        assert_eq!(body["text"]["format"]["strict"], true);
    }
}
