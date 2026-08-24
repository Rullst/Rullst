use serde::Deserialize;
use std::{fmt, str::FromStr};

/// Validated runtime environment shared by every Rullst subsystem.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Environment {
    /// Local development with developer tooling enabled.
    #[default]
    Development,
    /// Automated test execution.
    Test,
    /// Production-like pre-release deployment.
    Staging,
    /// Public production deployment with all secure defaults enabled.
    Production,
}

impl Environment {
    /// Resolves environment sources using the documented precedence:
    /// `RULLST_ENV`, legacy `APP_ENV`, then `[app].env` from `Rullst.toml`.
    pub fn resolve(
        rullst_env: Option<&str>,
        app_env: Option<&str>,
        configured_env: Option<&str>,
    ) -> Result<Self, ConfigError> {
        rullst_env
            .or(app_env)
            .or(configured_env)
            .map(Self::from_str)
            .transpose()
            .map(|environment| environment.unwrap_or_default())
    }

    /// Resolves the environment from process variables and an optional config value.
    pub fn detect(configured_env: Option<&str>) -> Result<Self, ConfigError> {
        let rullst_env = read_environment_variable("RULLST_ENV")?;
        let app_env = read_environment_variable("APP_ENV")?;
        Self::resolve(rullst_env.as_deref(), app_env.as_deref(), configured_env)
    }

    /// Whether local-only developer tooling may be exposed.
    pub const fn allows_development_tools(self) -> bool {
        matches!(self, Self::Development)
    }

    /// Whether production security layers and secure cookies are mandatory.
    pub const fn requires_secure_defaults(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

impl FromStr for Environment {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" | "local" => Ok(Self::Development),
            "test" | "testing" => Ok(Self::Test),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(ConfigError::InvalidEnvironment(value.to_string())),
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Staging => "staging",
            Self::Production => "production",
        };
        formatter.write_str(value)
    }
}

/// Strongly typed failures while loading Rullst configuration.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigError {
    /// A configured environment name is not recognized.
    #[error("invalid Rullst environment `{0}`")]
    InvalidEnvironment(String),

    /// An environment variable contains non-Unicode data.
    #[error("environment variable `{0}` is not valid Unicode")]
    NonUnicodeEnvironmentVariable(String),

    /// A configuration file could not be read.
    #[error("failed to read Rullst configuration: {0}")]
    Read(String),

    /// TOML configuration is invalid.
    #[error("failed to parse Rullst configuration: {0}")]
    Parse(String),

    /// A security policy is malformed or would create an ambiguous exemption.
    #[error("invalid security configuration: {0}")]
    InvalidSecurityConfiguration(String),
}

fn read_environment_variable(name: &str) -> Result<Option<String>, ConfigError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(ConfigError::NonUnicodeEnvironmentVariable(name.to_string()))
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// Main configuration container for Rullst applications.
pub struct RullstConfig {
    #[serde(default)]
    /// General application settings.
    pub app: AppConfig,
    #[serde(default)]
    /// Database settings.
    pub database: DatabaseConfig,
    #[serde(default)]
    /// Security policies and configuration parameters.
    pub security: SecurityConfig,
    #[serde(default)]
    /// File storage settings.
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// Configuration settings for storage drivers.
pub struct StorageConfig {
    /// The root directory for filesystem storage.
    pub root: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// General configuration options for the application instance.
pub struct AppConfig {
    /// Environment profile of the application (e.g. "development", "production").
    pub env: Option<String>,
    /// The port number that the HTTP server will bind to.
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// Database connection configuration.
pub struct DatabaseConfig {
    /// Database connection URL (e.g., `sqlite://rullst.db`).
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
/// Configuration options for application security layers.
pub struct SecurityConfig {
    /// SameSite policy for the CSRF cookie ("Lax", "Strict", or "None").
    #[serde(default = "default_same_site")]
    pub csrf_same_site: String,
    /// List of allowed origins for Cross-Origin Resource Sharing (CORS).
    #[serde(default)]
    pub cors_allow_origins: Vec<String>,
    /// Content-Security-Policy (CSP) header value.
    #[serde(default = "default_csp")]
    pub csp: String,
    /// User-Agent strings or substrings to block in the WAF middleware.
    #[serde(default = "default_user_agent_blocklist")]
    pub user_agent_blocklist: Vec<String>,
    /// Enable global automatic PII masking middleware on all textual responses (heavy performance cost).
    #[serde(default = "default_false")]
    pub enable_pii_masking: bool,
    /// Exact POST paths that are exempt from browser CSRF tokens because a mandatory signed
    /// webhook middleware authenticates them. Wildcards and route parameters are rejected.
    #[serde(default)]
    pub csrf_signed_webhook_paths: Vec<String>,
}

/// Strict default CSP template. `{NONCE}` is replaced by the secure headers middleware for each
/// request before the header is emitted.
pub const DEFAULT_CSP_TEMPLATE: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'nonce-{NONCE}'; style-src 'self' 'nonce-{NONCE}'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";

fn default_csp() -> String {
    DEFAULT_CSP_TEMPLATE.to_string()
}

fn default_user_agent_blocklist() -> Vec<String> {
    vec![
        "curl".to_string(),
        "wget".to_string(),
        "python-requests".to_string(),
        "go-http-client".to_string(),
        "gptbot".to_string(),
        "chatgpt-user".to_string(),
        "google-extended".to_string(),
        "anthropic-ai".to_string(),
        "claude-web".to_string(),
        "cohere-ai".to_string(),
        "bytespider".to_string(),
        "mj12bot".to_string(),
    ]
}

fn default_same_site() -> String {
    "Lax".to_string()
}

fn default_false() -> bool {
    false
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            csrf_same_site: default_same_site(),
            cors_allow_origins: vec![],
            csp: default_csp(),
            user_agent_blocklist: default_user_agent_blocklist(),
            enable_pii_masking: false,
            csrf_signed_webhook_paths: Vec::new(),
        }
    }
}

impl SecurityConfig {
    /// Validates exact-path CSRF webhook exemptions.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut unique_paths = std::collections::HashSet::new();
        for path in &self.csrf_signed_webhook_paths {
            let is_exact_absolute_path = path.starts_with('/')
                && path.len() > 1
                && !path.contains(['?', '#', '*', '{', '}', ':'])
                && !path
                    .split('/')
                    .any(|segment| segment == ".." || segment == ".");
            if !is_exact_absolute_path {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "CSRF signed-webhook exemption `{path}` must be an exact absolute path"
                )));
            }
            if !unique_paths.insert(path) {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "duplicate CSRF signed-webhook exemption `{path}`"
                )));
            }
        }
        Ok(())
    }
}

static GLOBAL_CONFIG: std::sync::OnceLock<RullstConfig> = std::sync::OnceLock::new();

impl RullstConfig {
    /// Gets the global configuration reference, initializing it with default values if not set.
    pub fn global() -> &'static RullstConfig {
        GLOBAL_CONFIG.get_or_init(Self::default)
    }

    /// Sets the global configuration instance.
    #[allow(clippy::result_large_err)]
    pub fn set_global(config: Self) -> Result<(), Self> {
        GLOBAL_CONFIG.set(config)
    }

    /// Creates a new `RullstConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates cross-field configuration invariants before any global state is published.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.security.validate()
    }

    /// Parses a complete `Rullst.toml` document.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(|error| ConfigError::Parse(error.to_string()))
    }

    /// Resolves the validated runtime environment for this configuration.
    pub fn environment(&self) -> Result<Environment, ConfigError> {
        Environment::detect(self.app.env.as_deref())
    }

    /// Loads and parses the configuration from a TOML file.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|error| ConfigError::Read(error.to_string()))?;
        Self::from_toml(&content)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_global_config_access() {
        let config1 = RullstConfig::global();
        let config2 = RullstConfig::global();
        assert!(
            std::ptr::eq(config1, config2),
            "global() should return the same instance"
        );
        assert_eq!(config1.security.csrf_same_site, "Lax");
    }

    #[tokio::test]
    async fn test_load_config_from_file() {
        let temp_dir = "test_config_dir";
        let _ = std::fs::create_dir_all(temp_dir);
        let path = format!("{}/Rullst.toml", temp_dir);

        let toml_content = r#"
[app]
env = "production"
port = 8080

[database]
url = "sqlite::memory:"

[security]
csrf_same_site = "Strict"
cors_allow_origins = ["https://example.com"]
"#;
        tokio::fs::write(&path, toml_content).await.unwrap();

        let config = RullstConfig::load_from_file(&path).await.unwrap();

        assert_eq!(config.app.env.unwrap(), "production");
        assert_eq!(config.app.port.unwrap(), 8080);
        assert_eq!(config.database.url.unwrap(), "sqlite::memory:");
        assert_eq!(config.security.csrf_same_site, "Strict");
        assert_eq!(config.security.cors_allow_origins.len(), 1);
        assert_eq!(config.security.cors_allow_origins[0], "https://example.com");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.csrf_same_site, "Lax");
        assert!(config.csp.contains("default-src"));
        assert!(!config.csp.contains("unsafe-inline"));
        assert!(!config.csp.contains("unsafe-eval"));
        assert!(config.user_agent_blocklist.contains(&"curl".to_string()));
        assert!(config.csrf_signed_webhook_paths.is_empty());
    }

    #[test]
    fn test_set_global_config() {
        let mut config = RullstConfig::new();
        config.app.env = Some("test_env".to_string());
        let result = RullstConfig::set_global(config);
        match result {
            Ok(_) => assert_eq!(RullstConfig::global().app.env.as_deref(), Some("test_env")),
            Err(c) => assert_eq!(c.app.env.as_deref(), Some("test_env")),
        }
    }

    #[test]
    fn test_deserialize_security_config_defaults() {
        let config: SecurityConfig = toml::from_str("").unwrap();
        assert!(!config.enable_pii_masking);
    }

    #[test]
    fn environment_resolution_has_one_precedence_and_validated_aliases() {
        assert_eq!(
            Environment::resolve(Some("prod"), Some("test"), Some("development")).unwrap(),
            Environment::Production
        );
        assert_eq!(
            Environment::resolve(None, Some("STAGE"), Some("development")).unwrap(),
            Environment::Staging
        );
        assert_eq!(
            Environment::resolve(None, None, Some("testing")).unwrap(),
            Environment::Test
        );
        assert_eq!(
            Environment::resolve(None, None, None).unwrap(),
            Environment::Development
        );
        assert!(Environment::resolve(Some("unknown"), None, None).is_err());
    }

    #[test]
    fn only_development_exposes_developer_tools() {
        assert!(Environment::Development.allows_development_tools());
        assert!(!Environment::Test.allows_development_tools());
        assert!(Environment::Staging.requires_secure_defaults());
        assert!(Environment::Production.requires_secure_defaults());
    }

    #[test]
    fn signed_webhook_csrf_exemptions_must_be_exact_paths() {
        let mut config = SecurityConfig::default();
        config.csrf_signed_webhook_paths = vec!["/billing/webhook".to_owned()];
        assert!(config.validate().is_ok());

        for invalid in [
            "billing/webhook",
            "/billing/:provider",
            "/billing/{provider}",
            "/billing/*path",
            "/billing/../admin",
            "/billing/webhook?provider=x",
        ] {
            config.csrf_signed_webhook_paths = vec![invalid.to_owned()];
            assert!(config.validate().is_err(), "{invalid} must be rejected");
        }

        config.csrf_signed_webhook_paths =
            vec!["/billing/webhook".to_owned(), "/billing/webhook".to_owned()];
        assert!(config.validate().is_err());
    }
}
