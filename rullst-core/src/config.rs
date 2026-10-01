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

#[derive(Clone, Deserialize, Default)]
#[non_exhaustive]
/// Database connection configuration.
///
/// `Debug` prints only the URL scheme (for example `postgres://<redacted>`),
/// never credentials, host, path or query parameters.
pub struct DatabaseConfig {
    /// Database connection URL (e.g., `sqlite://rullst.db`).
    pub url: Option<String>,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &self.url.as_deref().map(redacted_url))
            .finish()
    }
}

/// Debug form of a connection URL: the scheme only, or `<redacted>` when no
/// plain scheme is present. Userinfo, host, path and query are never shown.
pub(crate) fn redacted_url(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, _))
            if (1..=32).contains(&scheme.len())
                && scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) =>
        {
            format!("{scheme}://<redacted>")
        }
        _ => "<redacted>".to_string(),
    }
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
    /// Allow browser credentials only for the exact configured CORS origins.
    #[serde(default = "default_false")]
    pub cors_allow_credentials: bool,
    /// Content-Security-Policy (CSP) header value.
    #[serde(default = "default_csp")]
    pub csp: String,
    /// Cross-Origin-Embedder-Policy value. The strict default is `require-corp`;
    /// applications that deliberately embed reviewed credentialless cross-origin
    /// media can opt into `credentialless`, while `unsafe-none` must be an
    /// explicit application-level decision.
    #[serde(default = "default_coep")]
    pub coep: String,
    /// Case-insensitive User-Agent substrings to block in the WAF middleware.
    /// Defaults cover selected crawlers, not general HTTP clients or health probes.
    /// This forgeable header is a traffic preference, never authentication.
    /// An empty or whitespace-only entry is rejected by [`SecurityConfig::validate`],
    /// because every User-Agent contains it.
    #[serde(default = "default_user_agent_blocklist")]
    pub user_agent_blocklist: Vec<String>,
    /// Enable global automatic PII masking middleware on all textual responses (heavy performance cost).
    #[serde(default = "default_false")]
    pub enable_pii_masking: bool,
    /// Exact POST paths that are exempt from browser CSRF tokens because a mandatory signed
    /// webhook middleware authenticates them. Wildcards and route parameters are rejected.
    #[serde(default)]
    pub csrf_signed_webhook_paths: Vec<String>,
    /// Reverse-proxy addresses or CIDR networks allowed to report the client
    /// address. Empty (the default) never reads forwarding headers. List only
    /// the real proxy networks: any host inside them can choose the client
    /// identity. At most 64 entries; `/0` and host bits are rejected.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    /// Forwarding header read from `trusted_proxies`: `x-forwarded-for`
    /// (default) or the RFC 7239 `forwarded` header. Never both.
    #[serde(default = "default_trusted_proxy_header")]
    pub trusted_proxy_header: String,
    /// Report the scheme received by a trusted proxy (`X-Forwarded-Proto` or
    /// `Forwarded: proto=`). Enable only when every trusted proxy overwrites it.
    #[serde(default = "default_false")]
    pub trust_forwarded_proto: bool,
}

/// Strict default CSP template. `{NONCE}` is replaced by the secure headers middleware for each
/// request before the header is emitted.
pub const DEFAULT_CSP_TEMPLATE: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'nonce-{NONCE}'; style-src 'self' 'nonce-{NONCE}'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";

fn default_csp() -> String {
    DEFAULT_CSP_TEMPLATE.to_string()
}

fn default_coep() -> String {
    "require-corp".to_string()
}

fn default_user_agent_blocklist() -> Vec<String> {
    vec![
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

fn default_trusted_proxy_header() -> String {
    "x-forwarded-for".to_string()
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
            cors_allow_credentials: false,
            csp: default_csp(),
            coep: default_coep(),
            user_agent_blocklist: default_user_agent_blocklist(),
            enable_pii_masking: false,
            csrf_signed_webhook_paths: Vec::new(),
            trusted_proxies: Vec::new(),
            trusted_proxy_header: default_trusted_proxy_header(),
            trust_forwarded_proto: false,
        }
    }
}

impl SecurityConfig {
    /// Validates browser security policy, exact-path CSRF webhook exemptions
    /// and the trusted-proxy settings.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !matches!(self.csrf_same_site.as_str(), "Lax" | "Strict" | "None") {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "CSRF SameSite policy `{}` must be Lax, Strict, or None",
                self.csrf_same_site
            )));
        }
        let rendered_csp = self.csp.replace("{NONCE}", "validation-nonce");
        if self.csp.trim().is_empty() || rendered_csp.parse::<http::HeaderValue>().is_err() {
            return Err(ConfigError::InvalidSecurityConfiguration(
                "CSP must be a non-empty valid HTTP header value".to_string(),
            ));
        }
        if !matches!(
            self.coep.as_str(),
            "require-corp" | "credentialless" | "unsafe-none"
        ) {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "COEP policy `{}` must be require-corp, credentialless, or unsafe-none",
                self.coep
            )));
        }
        if let Some(position) = self
            .user_agent_blocklist
            .iter()
            .position(|agent| agent.trim().is_empty())
        {
            return Err(ConfigError::InvalidSecurityConfiguration(format!(
                "user_agent_blocklist entry {} is empty; it would block every request",
                position + 1
            )));
        }
        let mut unique_origins = std::collections::HashSet::new();
        for origin in &self.cors_allow_origins {
            let uri = origin.parse::<http::Uri>().map_err(|_| {
                ConfigError::InvalidSecurityConfiguration(format!(
                    "CORS origin `{origin}` must be an exact HTTP(S) origin"
                ))
            })?;
            let valid_scheme = uri
                .scheme_str()
                .is_some_and(|scheme| matches!(scheme, "http" | "https"));
            let valid_origin = valid_scheme
                && uri.authority().is_some()
                && uri.path() == "/"
                && uri.query().is_none()
                && !origin.ends_with('/')
                && !origin.contains(['@', '#', '*'])
                && origin.parse::<http::HeaderValue>().is_ok();
            if !valid_origin {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "CORS origin `{origin}` must be an exact HTTP(S) origin without path, query, credentials, wildcard, or trailing slash"
                )));
            }
            if !unique_origins.insert(origin) {
                return Err(ConfigError::InvalidSecurityConfiguration(format!(
                    "duplicate CORS origin `{origin}`"
                )));
            }
        }
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
        #[cfg(not(target_arch = "wasm32"))]
        crate::security::TrustedProxyConfig::from_security_config(self)
            .map_err(|error| ConfigError::InvalidSecurityConfiguration(error.to_string()))?;
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
    ///
    /// A failure reports only the line and column. The `toml` error text quotes
    /// the offending source line and can echo values, such as `app_key` or a
    /// database URL, so it never reaches `ConfigError::Parse`.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(|error: toml::de::Error| {
            ConfigError::Parse(match error.span() {
                Some(span) => {
                    let before = content.get(..span.start).unwrap_or(content);
                    let line = before.matches('\n').count() + 1;
                    let column = before
                        .rsplit('\n')
                        .next()
                        .map_or(0, |text| text.chars().count())
                        + 1;
                    format!("invalid configuration at line {line}, column {column}")
                }
                None => "invalid configuration".to_string(),
            })
        })
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
#[path = "config_tests.rs"]
mod tests;
