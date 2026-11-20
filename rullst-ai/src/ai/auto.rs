//! One resolver for automatic clients and their configuration indicators.
use super::providers::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAiProvider,
    openai_compatible::OpenAiCompatibleProvider,
};
use super::{AiClient, AiError, AiProvider, FallbackProvider, mock::ProviderMode};
use rullst_core::config::Environment;
use std::sync::Arc;

/// The end of every label of a provider in offline mode.
const OFFLINE_LABEL: &str = "(offline mock)";

/// Labels a value offline exactly when the provider constructor selects mock mode.
fn offline_credential(credential: &str) -> bool {
    ProviderMode::from_credential(credential).is_mock()
}

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
            let offline = offline_credential(&key);
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
            let name = if offline_credential(&key) {
                "Anthropic Claude (offline mock)"
            } else {
                "Anthropic Claude"
            };
            config.push(AnthropicProvider::new(key), name);
        }
        if let Some(key) = get("GEMINI_API_KEY") {
            let name = if offline_credential(&key) {
                "Google Gemini (offline mock)"
            } else {
                "Google Gemini"
            };
            config.push(GeminiProvider::new(key), name);
        }
        if let Some(key) = get("DEEPSEEK_API_KEY") {
            let name = if offline_credential(&key) {
                "DeepSeek (offline mock)"
            } else {
                "DeepSeek"
            };
            config.push(OpenAiCompatibleProvider::deepseek(key)?, name);
        }
        if let Some(key) = get("GROQ_API_KEY") {
            let name = if offline_credential(&key) {
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
            let name = if offline_credential(&host) {
                "Ollama Local (offline mock)"
            } else {
                "Ollama Local"
            };
            let model = get("OLLAMA_MODEL").unwrap_or_else(|| "llama3".into());
            config.push(OpenAiCompatibleProvider::ollama(host, model)?, name);
        }
        config.refuse_offline_fallback(|| {
            Environment::resolve(
                get("RULLST_ENV").as_deref(),
                get("APP_ENV").as_deref(),
                None,
            )
            .map_err(|_| {
                AiError::ConfigError(
                    "RULLST_ENV or APP_ENV is not a valid Rullst environment".into(),
                )
            })
        })?;
        Ok(config)
    }

    /// The chain tries the next provider after any error, so an offline mock
    /// behind a live provider answers with fixture text whenever the live one
    /// fails, as if the request had succeeded. In staging and production that
    /// is a leftover `mock_*` value, so the configuration is refused there.
    /// A chain of mocks only, or of live providers only, is unaffected.
    fn refuse_offline_fallback(
        &self,
        environment: impl FnOnce() -> Result<Environment, AiError>,
    ) -> Result<(), AiError> {
        let offline: Vec<&str> = self
            .names
            .iter()
            .copied()
            .filter(|name| name.ends_with(OFFLINE_LABEL))
            .collect();
        if offline.is_empty() || offline.len() == self.names.len() {
            return Ok(());
        }
        let environment = environment()?;
        if environment.requires_secure_defaults() {
            return Err(AiError::ConfigError(format!(
                "{} would answer with mock output whenever a live provider fails in the \
                 {environment} environment; remove its mock_* value",
                offline.join(", ")
            )));
        }
        Ok(())
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
    async fn labels_follow_the_provider_mode_for_padded_mock_values() {
        // OpenAI-compatible endpoints (custom and Groq) reject padded keys as
        // malformed, so only these providers can run with one.
        let config = resolve(&[
            ("OPENAI_API_KEY", " mock_openai"),
            ("ANTHROPIC_API_KEY", " mock_demo"),
            ("GEMINI_API_KEY", "\tmock_gemini"),
            ("DEEPSEEK_API_KEY", " mock_deepseek"),
            ("OLLAMA_HOST", " mock_ollama"),
        ])
        .unwrap();
        assert_eq!(
            config.provider_names(),
            [
                "OpenAI (offline mock)",
                "Anthropic Claude (offline mock)",
                "Google Gemini (offline mock)",
                "DeepSeek (offline mock)",
                "Ollama Local (offline mock)",
            ]
        );
        // The providers really run offline, as the labels now state.
        assert!(
            config
                .into_client()
                .prompt("Hello")
                .await
                .unwrap()
                .starts_with("Mock response")
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

    #[test]
    fn production_refuses_a_mock_behind_a_live_provider() {
        let mixed = [
            ("OPENAI_API_KEY", "sk-fixture-live"),
            ("GEMINI_API_KEY", "mock_gemini"),
        ];
        let with = |environment: &[(&'static str, &'static str)]| {
            let mut values = mixed.to_vec();
            values.extend_from_slice(environment);
            resolve(&values)
        };
        for environment in [("RULLST_ENV", "production"), ("APP_ENV", "staging")] {
            let error = with(&[environment]).unwrap_err();
            assert!(
                matches!(&error, AiError::ConfigError(text)
                    if text.contains("Google Gemini (offline mock)") && text.contains(environment.1)
                        && !text.contains("mock_gemini") && !text.contains("sk-fixture")),
                "{error}"
            );
        }
        // Development and test keep the chain, and so does an unset environment.
        for environment in [
            &[("RULLST_ENV", "development")][..],
            &[("APP_ENV", "test")],
            &[],
        ] {
            assert_eq!(
                with(environment).unwrap().provider_names(),
                ["OpenAI", "Google Gemini (offline mock)"]
            );
        }
        // Mocks only, or live providers only, are allowed everywhere.
        let production = ("RULLST_ENV", "production");
        let mocks = resolve(&[
            ("OPENAI_API_KEY", "mock_openai"),
            ("GEMINI_API_KEY", "mock_gemini"),
            production,
        ])
        .unwrap();
        assert_eq!(mocks.provider_names().len(), 2);
        let live = resolve(&[
            ("OPENAI_API_KEY", "sk-fixture-live"),
            ("GEMINI_API_KEY", "AIza-fixture"),
            production,
        ])
        .unwrap();
        assert_eq!(live.provider_names(), ["OpenAI", "Google Gemini"]);
        // An environment that cannot be resolved fails closed for a mixed chain.
        assert!(matches!(
            with(&[("RULLST_ENV", "not-an-environment")]),
            Err(AiError::ConfigError(text)) if text.contains("RULLST_ENV")
        ));
    }

    #[test]
    fn live_keys_are_labelled_live_and_debug_lists_only_names() {
        let config = resolve(&[("OPENAI_API_KEY", "sk-fixture-live")]).unwrap();
        assert_eq!(config.provider_names(), ["OpenAI"]);
        assert_eq!(
            format!("{config:?}"),
            r#"AutoAiConfig { providers: ["OpenAI"], .. }"#
        );
    }
}
