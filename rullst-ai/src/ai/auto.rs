//! One resolver for automatic clients and their configuration indicators.
use super::providers::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAiProvider,
    openai_compatible::OpenAiCompatibleProvider,
};
use super::{AiClient, AiError, AiProvider, FallbackProvider, mock::ProviderMode};
use rullst_core::config::{ConfigError, Environment, RullstConfig, project_setting};
use std::sync::Arc;

/// The end of every label of a provider in offline mode.
const OFFLINE_LABEL: &str = "(offline mock)";

/// The error for a `RULLST_ENV`, `APP_ENV` or `[app] env` that names no
/// Rullst environment.
const INVALID_ENVIRONMENT: &str =
    "RULLST_ENV, APP_ENV or [app] env is not a valid Rullst environment";

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
    /// Why a snapshot without providers gets no offline fixture: set in
    /// staging and production, and when the environment cannot be resolved.
    unconfigured: Option<String>,
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
    ///
    /// Provider settings come from the process environment. Whether the
    /// application runs in staging or production is resolved as `Server`
    /// resolves it, from the working directory: the process `RULLST_ENV`,
    /// then `APP_ENV`, then either one from `.env`, then `env` under `[app]`
    /// in `Rullst.toml` (since 13.0.0-rc.1). They are read only when a
    /// refusal depends on them.
    pub fn from_env() -> Result<Self, AiError> {
        Self::from_lookup(|name| std::env::var(name).ok(), project_environment)
    }

    /// Configured fallback order, with explicit mocks labeled as offline.
    pub fn provider_names(&self) -> &[&'static str] {
        &self.names
    }

    /// Builds a guarded client from this exact snapshot. No network is used.
    ///
    /// With no configured provider, development and test get the
    /// deterministic offline fixture. In staging and production (resolved as
    /// [`Self::from_env`] says) the client has no provider instead, so every request fails with [`AiError::ConfigError`]
    /// rather than answering with fixture text (since 13.0.0-rc.1).
    pub fn into_client(mut self) -> AiClient {
        if self.providers.is_empty() && self.unconfigured.is_none() {
            self.providers
                .push(Arc::new(OpenAiProvider::new("mock_auto")));
        }
        AiClient::new(FallbackProvider::new(self.providers))
    }

    /// The client of [`AiClient::auto`]: a snapshot without any provider is a
    /// configuration error in staging and production.
    pub(super) fn into_auto_client(self) -> Result<AiClient, AiError> {
        match &self.unconfigured {
            Some(reason) => Err(AiError::ConfigError(reason.clone())),
            None => Ok(self.into_client()),
        }
    }

    fn push(&mut self, provider: impl AiProvider + 'static, name: &'static str) {
        self.providers.push(Arc::new(provider));
        self.names.push(name);
    }

    fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
        environment: impl Fn() -> Result<Environment, AiError>,
    ) -> Result<Self, AiError> {
        let get = |name| lookup(name).filter(|value| !value.trim().is_empty());
        let mut config = Self {
            providers: Vec::new(),
            names: Vec::new(),
            unconfigured: None,
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
        config.refuse_offline_fallback(&environment)?;
        config.unconfigured = config.missing_provider(&environment);
        Ok(config)
    }

    /// With no provider, [`Self::into_client`] answers with an offline
    /// fixture as if a model had replied. In staging and production that
    /// means no key was set, so the snapshot records why instead. An explicit
    /// `mock_*` value is a configured provider and keeps the fixture.
    fn missing_provider(
        &self,
        environment: impl FnOnce() -> Result<Environment, AiError>,
    ) -> Option<String> {
        if !self.providers.is_empty() {
            return None;
        }
        match environment() {
            Ok(environment) if !environment.requires_secure_defaults() => None,
            Ok(environment) => Some(format!(
                "no AI provider is configured in the {environment} environment, so the client \
                 would answer with offline fixture text; set OPENAI_API_KEY, ANTHROPIC_API_KEY, \
                 GEMINI_API_KEY, DEEPSEEK_API_KEY, GROQ_API_KEY or OLLAMA_HOST in the process \
                 environment, or a `mock_` value to keep the offline fixture on purpose"
            )),
            Err(error) => Some(reason(error)),
        }
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

/// The environment `Server` enforces, read from the working directory: the
/// process `RULLST_ENV`, then the process `APP_ENV` (`Environment::detect`
/// reads both), then either one from `.env` (which `project_setting` returns
/// when the process lacks it), then `env` under `[app]` in `Rullst.toml`.
fn project_environment() -> Result<Environment, AiError> {
    let configured = match project_setting("RULLST_ENV").map_err(settings_error)? {
        Some(environment) => Some(environment),
        None => project_setting("APP_ENV").map_err(settings_error)?,
    };
    let configured = match configured {
        Some(environment) => Some(environment),
        None => configured_environment()?,
    };
    Environment::detect(configured.as_deref()).map_err(settings_error)
}

/// `env` under `[app]` in `./Rullst.toml`; a missing file sets none.
fn configured_environment() -> Result<Option<String>, AiError> {
    match std::fs::read_to_string("Rullst.toml") {
        Ok(config) => Ok(RullstConfig::from_toml(&config)
            .map_err(settings_error)?
            .app
            .env),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AiError::ConfigError(format!(
            "Rullst.toml could not be read: {}",
            error.kind()
        ))),
    }
}

/// Core reports these without `.env` or `Rullst.toml` content.
fn settings_error(error: ConfigError) -> AiError {
    match error {
        ConfigError::InvalidEnvironment(_) => AiError::ConfigError(INVALID_ENVIRONMENT.into()),
        other => AiError::ConfigError(format!("the AI environment could not be read: {other}")),
    }
}

/// The text of a configuration error, for the snapshot's refusal.
fn reason(error: AiError) -> String {
    match error {
        AiError::ConfigError(reason) => reason,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A snapshot of `values`, whose `RULLST_ENV` and `APP_ENV` (a blank one
    /// counts as unset) stand in for the project's environment.
    fn resolve(values: &[(&str, &str)]) -> Result<AutoAiConfig, AiError> {
        let lookup = |name: &str| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
                .filter(|value| !value.trim().is_empty())
        };
        AutoAiConfig::from_lookup(lookup, || {
            Environment::resolve(
                lookup("RULLST_ENV").as_deref(),
                lookup("APP_ENV").as_deref(),
                None,
            )
            .map_err(settings_error)
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

    #[tokio::test]
    async fn production_refuses_to_answer_without_any_provider() {
        for (variable, name) in [("RULLST_ENV", "production"), ("APP_ENV", "staging")] {
            let unconfigured = || resolve(&[(variable, name), ("OPENAI_API_KEY", " ")]).unwrap();
            // The snapshot itself stays valid: Nexus reports "no provider".
            assert!(unconfigured().provider_names().is_empty());
            let error = unconfigured().into_auto_client().err().unwrap();
            assert!(
                matches!(&error, AiError::ConfigError(text) if text.contains(name)
                    && text.contains("OPENAI_API_KEY") && text.contains("OLLAMA_HOST")
                    && text.contains("mock_")),
                "{error}"
            );
            // A client built anyway has no provider, so requests fail.
            let reply = unconfigured().into_client().prompt("Hello").await;
            assert!(matches!(reply, Err(AiError::ConfigError(_))), "{reply:?}");

            // An explicit mock is deliberate and answers offline.
            let mock = resolve(&[(variable, name), ("GEMINI_API_KEY", "mock_gemini")]).unwrap();
            let reply = mock.into_auto_client().unwrap().prompt("Hello").await;
            assert!(reply.unwrap().starts_with("Mock response"));
            // A live provider is used as configured.
            let live = resolve(&[(variable, name), ("OPENAI_API_KEY", "sk-fixture-live")]);
            assert!(live.unwrap().into_auto_client().is_ok());
        }
        // An environment that cannot be resolved fails closed as well.
        assert!(matches!(
            resolve(&[("RULLST_ENV", "not-an-environment")]).unwrap().into_auto_client(),
            Err(AiError::ConfigError(text)) if text == INVALID_ENVIRONMENT
        ));
    }

    #[tokio::test]
    async fn development_and_test_keep_the_offline_fixture_without_a_provider() {
        for environment in [
            &[("RULLST_ENV", "development")][..],
            &[("APP_ENV", "test")],
            &[("OPENAI_API_KEY", "")],
        ] {
            let client = resolve(environment).unwrap().into_auto_client().unwrap();
            assert!(
                client
                    .prompt("Hello")
                    .await
                    .unwrap()
                    .starts_with("Mock response")
            );
            let client = resolve(environment).unwrap().into_client();
            assert!(
                client
                    .prompt("Hello")
                    .await
                    .unwrap()
                    .starts_with("Mock response")
            );
        }
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
