//! Résumeur « En bref » via l'API Anthropic Messages (SPEC-en-bref CU-02).
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

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-opus-5";
const MAX_ATTEMPTS: u32 = 3;

pub struct AnthropicBriefGenerator {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicBriefGenerator {
    /// `TEXT_BRIEF_MODEL`, sinon `ANTHROPIC_MODEL`, sinon le modèle par défaut.
    pub fn from_env() -> Option<Self> {
        let api_key = env_value("ANTHROPIC_API_KEY")?;
        let model = env_value("TEXT_BRIEF_MODEL")
            .or_else(|| env_value("ANTHROPIC_MODEL"))
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
        "max_tokens": BRIEF_MAX_OUTPUT_TOKENS,
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
            "content": request_input(request)
        }]
    })
}

/// Le texte JSON de la réponse. Une réponse tronquée ou refusée est rejetée :
/// un JSON coupé ne doit jamais atteindre `parse_lines`.
fn answer_text(payload: &Value) -> Result<&str, SummaryGeneratorError> {
    if matches!(
        payload.get("stop_reason").and_then(Value::as_str),
        Some("max_tokens" | "refusal")
    ) {
        return Err(SummaryGeneratorError::Answer(
            "reponse tronquee ou refusee".into(),
        ));
    }
    payload
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        })
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| SummaryGeneratorError::Answer("aucun bloc de texte".into()))
}

#[async_trait]
impl BriefGenerator for AnthropicBriefGenerator {
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
            return parse_lines(answer_text(&payload)?);
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

    fn one_article() -> BriefRequest {
        BriefRequest {
            articles: vec![BriefArticle {
                key: "sha256:aaa".to_string(),
                content: "Contenu.".to_string(),
                cited: Vec::new(),
            }],
        }
    }

    #[test]
    fn builds_a_strict_json_request_with_the_shared_prompt_and_input() {
        let body = request_body("claude-essai", &one_article());

        assert_eq!(body["model"], "claude-essai");
        assert_eq!(body["max_tokens"], BRIEF_MAX_OUTPUT_TOKENS);
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["output_config"]["format"]["schema"], response_schema());
        assert_eq!(body["system"][0]["text"], system_prompt());
        assert_eq!(
            body["messages"][0]["content"],
            request_input(&one_article())
        );
    }

    #[test]
    fn reads_the_first_text_block_of_the_answer() {
        let payload = json!({
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": "..."},
                {"type": "text", "text": "{\"lines\":[]}"}
            ]
        });

        assert_eq!(answer_text(&payload).unwrap(), "{\"lines\":[]}");
    }

    #[test]
    fn rejects_a_truncated_or_refused_answer() {
        for reason in ["max_tokens", "refusal"] {
            let payload = json!({
                "stop_reason": reason,
                "content": [{"type": "text", "text": "{\"lines\":[{\"key\""}]
            });

            assert!(matches!(
                answer_text(&payload),
                Err(SummaryGeneratorError::Answer(_))
            ));
        }
    }

    #[test]
    fn rejects_an_answer_without_text_block() {
        let payload = json!({"stop_reason": "end_turn", "content": []});

        assert!(matches!(
            answer_text(&payload),
            Err(SummaryGeneratorError::Answer(_))
        ));
    }
}
