//! Classifieur thématique via l'API Anthropic Messages.
//!
//! Cadrage, schéma et lecture de la réponse sont communs aux trois fournisseurs
//! (`theme_prompt`): ce module n'en fait que le transport.

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::application::ports::theme_classifier::{ClassifierError, ThemeClassifier};
use crate::domain::theme::ProposedFamily;

use super::http_retry::{backoff, is_retryable, truncate_detail, HTTP_TIMEOUT, MAX_ATTEMPTS};
use super::theme_prompt::{
    max_output_tokens, numbered_labels, parse_text, response_schema, system_prompt, BATCH_SIZE,
    PROMPT_VERSION,
};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-opus-5";

pub struct AnthropicThemeClassifier {
    http: reqwest::Client,
    api_key: String,
    model: String,
    system_prompt: String,
}

impl AnthropicThemeClassifier {
    /// BYOK: rend `None` sans cle, le site tourne sans proposition.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())?;
        let model = std::env::var("THEME_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("ANTHROPIC_MODEL")
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
            system_prompt: system_prompt(),
        })
    }

    fn request_body(&self, labels: &[String]) -> Value {
        json!({
            "model": self.model,
            "max_tokens": max_output_tokens(labels.len()),
            // `low`: le rattachement d'un libelle est une tache courte. Le
            // schema fait le reste du travail de cadrage.
            "output_config": {
                "effort": "low",
                "format": { "type": "json_schema", "schema": response_schema(labels.len()) }
            },
            // Bloc systeme marque pour le cache: il est identique d'un lot au
            // suivant. Le fournisseur n'entretient le cache qu'au-dela d'une
            // longueur minimale — sous ce seuil la mention est sans effet, et
            // sans cout.
            "system": [{
                "type": "text",
                "text": self.system_prompt,
                "cache_control": { "type": "ephemeral" }
            }],
            "messages": [{
                "role": "user",
                // Les libelles, rien d'autre (RM-04).
                "content": numbered_labels(labels)
            }]
        })
    }
}

#[async_trait]
impl ThemeClassifier for AnthropicThemeClassifier {
    async fn propose_batch(
        &self,
        labels: &[String],
    ) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
        if labels.is_empty() {
            return Ok(vec![]);
        }

        let body = self.request_body(labels);
        let mut last_error = ClassifierError::Call("aucune tentative".to_string());

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
                Ok(response) => response,
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
                let detail = response.text().await.unwrap_or_default();
                return Err(ClassifierError::Call(format!(
                    "HTTP {status}: {}",
                    truncate_detail(&detail)
                )));
            }

            let payload: Value = response
                .json()
                .await
                .map_err(|e| ClassifierError::Answer(e.to_string()))?;
            return parse_answer(&payload, labels.len());
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

/// Lit la reponse et la reattribue aux libelles par leur numero.
///
/// Une famille hors referentiel ou une justification vide rend l'entree
/// incomplete et la laisse a `None`, pour qu'elle soit reprise. Un numero
/// absent de la reponse est traite de la meme maniere, plutot que compte comme
/// « aucune famille ».
fn parse_answer(
    payload: &Value,
    expected: usize,
) -> Result<Vec<Option<Vec<ProposedFamily>>>, ClassifierError> {
    match payload.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => {
            return Err(ClassifierError::Answer("réponse refusée".to_string()));
        }
        Some("max_tokens") => {
            // Le lot entier est perdu: le JSON tronque ne se relit pas.
            return Err(ClassifierError::Answer("réponse tronquée".to_string()));
        }
        _ => {}
    }

    let text = payload
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        })
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| ClassifierError::Answer("aucun bloc de texte".to_string()))?;

    parse_text(text, expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(text: &str) -> Value {
        json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": text }]
        })
    }

    #[test]
    fn the_token_ceiling_grows_with_the_batch() {
        let classifier = AnthropicThemeClassifier {
            http: reqwest::Client::new(),
            api_key: "clé-de-test".into(),
            model: "modèle".into(),
            system_prompt: system_prompt(),
        };
        let one = classifier.request_body(&["un".into()])["max_tokens"]
            .as_u64()
            .unwrap();
        let ten = classifier.request_body(&vec!["un".to_string(); 10])["max_tokens"]
            .as_u64()
            .unwrap();
        assert!(ten > one);
    }

    #[test]
    fn the_system_block_is_marked_for_caching() {
        let classifier = AnthropicThemeClassifier {
            http: reqwest::Client::new(),
            api_key: "clé-de-test".into(),
            model: "modèle".into(),
            system_prompt: system_prompt(),
        };
        let body = classifier.request_body(&["un".into()]);
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn the_text_block_of_the_message_is_read() {
        let parsed = parse_answer(
            &payload(
                r#"{"textes":[{"numero":1,"familles":[],"motif_absence":"hors_referentiel"}]}"#,
            ),
            1,
        )
        .unwrap();
        assert_eq!(parsed[0].as_ref().unwrap().len(), 0);
    }

    #[test]
    fn a_refusal_is_an_error() {
        let refused = json!({ "stop_reason": "refusal", "content": [] });
        assert!(parse_answer(&refused, 1).is_err());
    }

    #[test]
    fn a_truncated_answer_is_an_error() {
        let truncated = json!({
            "stop_reason": "max_tokens",
            "content": [{ "type": "text", "text": "{\"textes\":[" }]
        });
        assert!(parse_answer(&truncated, 1).is_err());
    }
}
