use serde_json::Value;

/// Extrait le texte JSON d'une réponse Chat Completions de Mistral.
///
/// Seul le premier choix est lu. Une réponse tronquée (`length`) ou en erreur
/// est rejetée: un JSON coupé ne doit jamais atteindre le parseur partagé.
/// Le contenu peut être une chaîne ou une liste de blocs; seuls les blocs
/// `text` sont retenus.
pub(crate) fn message_content(payload: &Value) -> Result<String, String> {
    let choice = payload
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| "aucun choix dans la reponse".to_string())?;

    if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
        if reason == "length" || reason == "error" {
            return Err(format!("reponse interrompue: {reason}"));
        }
    }

    let content = choice
        .get("message")
        .and_then(|message| message.get("content"))
        .ok_or_else(|| "aucun contenu dans la reponse".to_string())?;

    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }

    let text: String = content
        .as_array()
        .map(|chunks| {
            chunks
                .iter()
                .filter(|chunk| chunk.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|chunk| chunk.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if text.is_empty() {
        return Err("aucun bloc texte dans la reponse".into());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_string_content() {
        let payload = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"role": "assistant", "content": "{\"ok\":true}"}
            }]
        });
        assert_eq!(message_content(&payload).unwrap(), "{\"ok\":true}");
    }

    #[test]
    fn reads_text_chunks_and_skips_others() {
        let payload = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": {"content": [
                    {"type": "thinking", "thinking": []},
                    {"type": "text", "text": "{}"}
                ]}
            }]
        });
        assert_eq!(message_content(&payload).unwrap(), "{}");
    }

    #[test]
    fn rejects_truncated_responses() {
        let payload = json!({
            "choices": [{"finish_reason": "length", "message": {"content": "{\"tex"}}]
        });
        assert!(message_content(&payload).is_err());
    }

    #[test]
    fn rejects_responses_without_choice() {
        assert!(message_content(&json!({"choices": []})).is_err());
    }
}
