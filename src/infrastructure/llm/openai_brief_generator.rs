//! Résumeur « En bref » via l'API OpenAI Responses (SPEC-en-bref CU-02).
//!
//! Le prompt, le schéma et la lecture des lignes sont ceux de `brief_prompt` ;
//! ce module n'en fait que le transport.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::brief_generator::{
    BriefGenerator, BriefRequest, GeneratedLine, SummaryGeneratorError,
};

use super::brief_prompt::{
    parse_lines, request_input, response_schema, system_prompt, BRIEF_MAX_OUTPUT_TOKENS,
    PROMPT_VERSION,
};
use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT};
use super::openai_support::output_text;

const API_URL: &str = "https://api.openai.com/v1/responses";
const DEFAULT_MODEL: &str = "gpt-5.6-luna";
const MAX_ATTEMPTS: u32 = 3;

pub struct OpenAiBriefGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiBriefGenerator {
    /// `TEXT_BRIEF_MODEL`, sinon `OPENAI_MODEL`, sinon le modèle par défaut.
    pub fn from_env() -> Option<Self> {
        let api_key = env_value("OPENAI_API_KEY")?;
        let model = env_value("TEXT_BRIEF_MODEL")
            .or_else(|| env_value("OPENAI_MODEL"))
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
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn request_body(model: &str, request: &BriefRequest) -> Value {
    json!({
        "model": model,
        "store": false,
        "max_output_tokens": BRIEF_MAX_OUTPUT_TOKENS,
        "input": [
            {
                "role": "developer",
                "content": [{ "type": "input_text", "text": system_prompt() }]
            },
            {
                "role": "user",
                "content": [{ "type": "input_text", "text": request_input(request) }]
            }
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "text_brief",
                "strict": true,
                "schema": response_schema()
            },
            "verbosity": "low"
        }
    })
}

#[async_trait]
impl BriefGenerator for OpenAiBriefGenerator {
    async fn generate(
        &self,
        request: &BriefRequest,
    ) -> Result<Vec<GeneratedLine>, SummaryGeneratorError> {
        let body = request_body(&self.model, request);
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
            return parse_lines(text);
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
    use crate::application::ports::brief_generator::BriefArticle;

    #[test]
    fn builds_a_strict_json_request_with_the_shared_prompt_and_input() {
        let request = BriefRequest {
            articles: vec![BriefArticle {
                key: "sha256:aaa".to_string(),
                content: "Contenu.".to_string(),
                cited: Vec::new(),
            }],
        };

        let body = request_body("gpt-essai", &request);

        assert_eq!(body["model"], "gpt-essai");
        assert_eq!(body["store"], false);
        assert_eq!(body["max_output_tokens"], BRIEF_MAX_OUTPUT_TOKENS);
        assert_eq!(body["input"][0]["content"][0]["text"], system_prompt());
        assert_eq!(
            body["input"][1]["content"][0]["text"],
            request_input(&request)
        );
        assert_eq!(body["text"]["format"]["strict"], true);
        assert_eq!(body["text"]["format"]["schema"], response_schema());
    }
}
