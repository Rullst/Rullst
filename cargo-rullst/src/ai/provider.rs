//! The providers `cargo rullst ai` can use and how each is reached through
//! `rullst-ai`.

use std::fmt;

/// A supported model provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
    DeepSeek,
    Ollama,
}

/// Environment lookup order when no provider is selected or stored. It
/// matches `rullst_ai::AutoAiConfig`.
pub(super) const DETECTION_ORDER: [Provider; 5] = [
    Provider::OpenAi,
    Provider::Anthropic,
    Provider::Gemini,
    Provider::DeepSeek,
    Provider::Ollama,
];

/// The default local Ollama endpoint.
pub(super) const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";

impl Provider {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "deepseek" => Some(Self::DeepSeek),
            "ollama" => Some(Self::Ollama),
            _ => None,
        }
    }

    /// Stable identifier stored in the credentials file.
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Ollama => "ollama",
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic Claude",
            Self::Gemini => "Google Gemini",
            Self::DeepSeek => "DeepSeek",
            Self::Ollama => "Ollama (local)",
        }
    }

    /// The variable holding the secret (or, for Ollama, the host).
    pub(super) const fn env_var(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
            Self::DeepSeek => "DEEPSEEK_API_KEY",
            Self::Ollama => "OLLAMA_HOST",
        }
    }

    /// Ollama is addressed by host; every other provider needs an API key.
    pub(super) const fn uses_api_key(self) -> bool {
        !matches!(self, Self::Ollama)
    }

    /// The model used when none is configured. These mirror the defaults of
    /// the `rullst-ai` provider constructors (and `AutoAiConfig` for Ollama).
    pub(super) const fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::Anthropic => "claude-sonnet-5",
            Self::Gemini => "gemini-2.5-flash-lite",
            Self::DeepSeek => "deepseek-v4-flash",
            Self::Ollama => "llama3",
        }
    }

    /// Whether answers arrive incrementally. OpenAI, DeepSeek and a loopback
    /// Ollama use the bounded OpenAI-compatible SSE transport; the native
    /// Anthropic and Gemini transports in `rullst-ai` answer at once.
    pub(super) const fn streams(self) -> bool {
        matches!(self, Self::OpenAi | Self::DeepSeek | Self::Ollama)
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Model identifiers are passed to provider URLs and request bodies, so only
/// a conservative token grammar is accepted.
pub(super) fn valid_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 128
        && !model.contains("..")
        && model.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
        })
}

/// A credential is one printable ASCII token: no whitespace or controls that
/// could split a header, and a bounded size.
pub(super) fn valid_secret(secret: &str) -> bool {
    !secret.is_empty()
        && secret.len() <= 8 * 1024
        && secret.bytes().all(|byte| byte.is_ascii_graphic())
}

/// Empty and `mock_*` credentials select the deterministic offline assistant
/// (AGENTS.md 3.5), exactly like the `rullst-ai` provider constructors.
pub(super) fn is_mock_credential(secret: &str) -> bool {
    let secret = secret.trim();
    secret.is_empty() || secret.starts_with("mock_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_aliases_parse() {
        for provider in DETECTION_ORDER {
            assert_eq!(Provider::parse(provider.id()), Some(provider));
        }
        assert_eq!(Provider::parse("Claude"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("shell"), None);
    }

    #[test]
    fn models_and_secrets_use_bounded_token_grammars() {
        for model in [
            "gpt-4o-mini",
            "llama3:8b",
            "hf.co/org/model",
            "claude-sonnet-5",
        ] {
            assert!(valid_model(model), "{model}");
        }
        for model in ["", "a b", "../x", "m?x=1", "m#f", &"x".repeat(129)] {
            assert!(!valid_model(model), "{model}");
        }
        assert!(valid_secret("sk-abc_123"));
        for secret in ["", "two words", "line\nbreak", "tab\t", "é"] {
            assert!(!valid_secret(secret));
        }
        assert!(is_mock_credential("") && is_mock_credential(" mock_demo"));
        assert!(!is_mock_credential("sk-live"));
    }
}
