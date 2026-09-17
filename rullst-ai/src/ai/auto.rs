//! One resolver for automatic clients and their configuration indicators.
use super::providers::{
    anthropic::AnthropicProvider, deepseek::DeepSeekProvider, gemini::GeminiProvider,
    ollama::OllamaProvider, openai::OpenAiProvider, openai_compatible::OpenAiCompatibleProvider,
};
use super::{AiClient, AiError, AiProvider, FallbackProvider};
use std::sync::Arc;

/// Validated provider configuration captured without sending requests.
///
/// Names describe configuration, not health or live-account acceptance. Empty
/// environment values are absent; explicit `mock_*` values remain offline.
/// Credentials, model names and endpoint URLs are never included in Debug.
pub struct AutoAiConfig {
    providers: Vec<Arc<dyn AiProvider>>,
    names: Vec<&'static str>,
}

impl std::fmt::Debug for AutoAiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoAiConfig")
            .field("providers", &self.names)
            .finish_non_exhaustive()
    }
}

impl AutoAiConfig {
    /// Captures the environment once using the same order as [`AiClient::auto`].
    pub fn from_env() -> Result<Self, AiError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Configured fallback order, with explicit mocks labeled as offline.
    pub fn provider_names(&self) -> &[&'static str] {
        &self.names
    }

    /// Builds a guarded client from this exact snapshot. No network is used.
    /// With no configured provider, preserves the deterministic offline fallback.
    pub fn into_client(mut self) -> AiClient {
        if self.providers.is_empty() {
            self.providers
                .push(Arc::new(OpenAiProvider::new("mock_auto")));
        }
        AiClient::new(FallbackProvider::new(self.providers))
    }

    fn push(&mut self, provider: impl AiProvider + 'static, name: &'static str) {
        self.providers.push(Arc::new(provider));
        self.names.push(name);
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, AiError> {
        let get = |name| lookup(name).filter(|value| !value.trim().is_empty());
        let mut config = Self {
            providers: Vec::new(),
            names: Vec::new(),
        };
        let base_url = get("OPENAI_BASE_URL");
        let openai_key = get("OPENAI_API_KEY");
        if base_url.is_some() && openai_key.is_none() {
            return Err(AiError::ConfigError("OPENAI_BASE_URL requires OPENAI_API_KEY and OPENAI_MODEL; use an explicit provider for unauthenticated local servers".into()));
        }
        if let Some(key) = openai_key {
            let offline = key.starts_with("mock_");
            if let Some(base) = base_url {
                let model = get("OPENAI_MODEL").ok_or_else(|| {
                    AiError::ConfigError("a custom OPENAI_BASE_URL requires OPENAI_MODEL".into())
                })?;
                config.push(
                    OpenAiCompatibleProvider::try_cloud(base, key, model)?,
                    if offline {
                        "Custom LLM (offline mock)"
                    } else {
                        "Custom LLM Endpoint"
                    },
                );
            } else {
                let mut provider = OpenAiProvider::new(key);
                if let Some(model) = get("OPENAI_MODEL") {
                    provider = provider.with_model(model);
                }
                config.push(
                    provider,
                    if offline {
                        "OpenAI (offline mock)"
                    } else {
                        "OpenAI"
                    },
                );
            }
        }
        if let Some(key) = get("ANTHROPIC_API_KEY") {
            let name = if key.starts_with("mock_") {
                "Anthropic Claude (offline mock)"
            } else {
                "Anthropic Claude"
            };
            config.push(AnthropicProvider::new(key), name);
        }
        if let Some(key) = get("GEMINI_API_KEY") {
            let name = if key.starts_with("mock_") {
                "Google Gemini (offline mock)"
            } else {
                "Google Gemini"
            };
            config.push(GeminiProvider::new(key), name);
        }
        if let Some(key) = get("DEEPSEEK_API_KEY") {
            let name = if key.starts_with("mock_") {
                "DeepSeek (offline mock)"
            } else {
                "DeepSeek"
            };
            config.push(DeepSeekProvider::new(key), name);
        }
        if let Some(key) = get("GROQ_API_KEY") {
            let name = if key.starts_with("mock_") {
                "Groq (offline mock)"
            } else {
                "Groq"
            };
            let model = get("GROQ_MODEL").ok_or_else(|| {
                AiError::ConfigError(
                    "GROQ_API_KEY requires an explicit GROQ_MODEL available to this account".into(),
                )
            })?;
            config.push(
                OpenAiCompatibleProvider::try_cloud("https://api.groq.com/openai/v1", key, model)?,
                name,
            );
        }
        if let Some(host) = get("OLLAMA_HOST") {
            let name = if host.starts_with("mock_") {
                "Ollama Local (offline mock)"
            } else {
                "Ollama Local"
            };
            let model = get("OLLAMA_MODEL").unwrap_or_else(|| "llama3".into());
            config.push(OllamaProvider::new(host, model), name);
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(values: &[(&str, &str)]) -> Result<AutoAiConfig, AiError> {
        AutoAiConfig::from_lookup(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    #[tokio::test]
    async fn empty_values_cannot_mask_a_configured_groq_provider() {
        let config = resolve(&[
            ("OPENAI_API_KEY", ""),
            ("GEMINI_API_KEY", " "),
            ("GROQ_API_KEY", "mock_groq"),
            ("GROQ_MODEL", "fixture-model"),
        ])
        .unwrap();
        assert_eq!(config.provider_names(), ["Groq (offline mock)"]);
        assert!(!format!("{config:?}").contains("mock_groq"));
        let client = config.into_client();
        assert!(!client.capabilities().embeddings);
        assert!(
            client
                .prompt("Hello")
                .await
                .unwrap()
                .contains("fixture-model")
        );
    }

    #[tokio::test]
    async fn custom_configuration_is_explicit_and_validated_before_dispatch() {
        for values in [
            vec![("OPENAI_BASE_URL", "https://example.invalid/v1")],
            vec![
                ("OPENAI_BASE_URL", "https://example.invalid/v1"),
                ("OPENAI_API_KEY", "mock_key"),
            ],
            vec![("GROQ_API_KEY", "mock_key")],
        ] {
            assert!(matches!(resolve(&values), Err(AiError::ConfigError(_))));
        }
        for url in [
            "http://example.invalid/v1",
            "https://user:secret@example.invalid/v1",
            "https://example.invalid/v1?secret=value",
        ] {
            assert!(
                resolve(&[
                    ("OPENAI_BASE_URL", url),
                    ("OPENAI_API_KEY", "mock_key"),
                    ("OPENAI_MODEL", "fixture")
                ])
                .is_err()
            );
        }
        let config = resolve(&[
            ("OPENAI_BASE_URL", "https://example.invalid/v1"),
            ("OPENAI_API_KEY", "mock_key"),
            ("OPENAI_MODEL", "fixture"),
        ])
        .unwrap();
        assert_eq!(config.provider_names(), ["Custom LLM (offline mock)"]);
        assert!(
            !config
                .into_client()
                .prompt("Hello")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn resolver_preserves_precedence_and_offline_fallback() {
        let config = resolve(&[
            ("GEMINI_API_KEY", "mock_key"),
            ("DEEPSEEK_API_KEY", "mock_key"),
            ("OPENAI_API_KEY", "mock_key"),
        ])
        .unwrap();
        assert_eq!(
            config.provider_names(),
            [
                "OpenAI (offline mock)",
                "Google Gemini (offline mock)",
                "DeepSeek (offline mock)"
            ]
        );
        let empty = resolve(&[]).unwrap();
        assert!(empty.provider_names().is_empty());
        assert!(
            !empty
                .into_client()
                .prompt("Hello")
                .await
                .unwrap()
                .is_empty()
        );
    }
}
