#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LlmProvider {
    Anthropic,
    OpenAi,
    Mistral,
}

impl LlmProvider {
    /// Le défaut Anthropic conserve le comportement historique si
    /// `LLM_PROVIDER` n'est pas encore ajouté à l'environnement.
    pub fn from_env() -> Result<Self, String> {
        Self::parse(&std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into()))
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Ok(Self::Anthropic),
            "openai" | "chatgpt" => Ok(Self::OpenAi),
            "mistral" => Ok(Self::Mistral),
            value => Err(format!(
                "LLM_PROVIDER invalide: {value} (valeurs attendues: anthropic, openai ou mistral)"
            )),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic/Claude",
            Self::OpenAi => "OpenAI/ChatGPT",
            Self::Mistral => "Mistral",
        }
    }

    pub fn api_key_name(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Mistral => "MISTRAL_API_KEY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_provider_aliases() {
        assert_eq!(
            LlmProvider::parse("claude").unwrap(),
            LlmProvider::Anthropic
        );
        assert_eq!(LlmProvider::parse(" OPENAI ").unwrap(), LlmProvider::OpenAi);
        assert_eq!(LlmProvider::parse("chatgpt").unwrap(), LlmProvider::OpenAi);
        assert_eq!(
            LlmProvider::parse(" Mistral ").unwrap(),
            LlmProvider::Mistral
        );
    }

    #[test]
    fn rejects_unknown_provider() {
        assert!(LlmProvider::parse("gemini").is_err());
    }
}
