use serde::Deserialize;
use std::{fmt, str::FromStr};

mod access_log;
mod csrf_static;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod project_setting;
mod security_validation;

pub use access_log::AccessLogPaths;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use project_setting::parse_dotenv_entries;
#[cfg(not(target_arch = "wasm32"))]
pub use project_setting::project_setting;

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

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[non_exhaustive]
/// Configuration settings for storage drivers.
pub struct StorageConfig {
    /// The root directory for filesystem storage.
    pub root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[non_exhaustive]
/// General configuration options for the application instance.
pub struct AppConfig {
    /// Environment profile of the application (e.g. "development", "production").
    pub env: Option<String>,
    /// The port number that the HTTP server will bind to.
    pub port: Option<u16>,
    /// The request path the access log writes (`access_log_paths = "route"`,
    /// `"full"` or `"auto"`, the default). Added in 13.0.0-alpha.5.
    #[serde(default)]
    pub access_log_paths: AccessLogPaths,
}

#[derive(Clone, PartialEq, Deserialize, Default)]
#[non_exhaustive]
/// Database connection configuration.
///
/// `Debug` prints only the URL scheme (for example `postgres://<redacted>`),
/// never credentials, host, path or query parameters.
pub struct DatabaseConfig {
    /// Database connection URL (e.g., `sqlite://rullst.db`).
    pub url: Option<String>,
    /// Journal mode that `Server` and Artisan set on a SQLite database file
    /// (`sqlite_journal_mode = "wal"` or `"delete"`); a PostgreSQL or MySQL
    /// URL and an in-memory database ignore it. Without the key the database
    /// keeps the mode recorded in its file. Projects generated since
    /// 13.0.0-alpha.6 set `"wal"`; keep `"delete"` for a database on a
    /// network filesystem (NFS, SMB, Azure Files). See
    /// [`rullst_orm::SqliteJournalMode`]. Added in 13.0.0-alpha.6.
    #[cfg(feature = "orm")]
    #[serde(default)]
    pub sqlite_journal_mode: Option<rullst_orm::SqliteJournalMode>,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("DatabaseConfig");
        debug.field("url", &self.url.as_deref().map(redacted_url));
        #[cfg(feature = "orm")]
        debug.field("sqlite_journal_mode", &self.sqlite_journal_mode);
        debug.finish()
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    /// Path prefixes under which the application serves static files. A
    /// `GET` or `HEAD` below one of them never mints a CSRF token or sets the
    /// `rullst_csrf` cookie (it still receives the `CsrfToken` of a
    /// valid cookie the browser sends); every other page does, whatever
    /// its extension. Each entry starts and ends with `/`. Default
    /// `["/static/", "/.well-known/"]`; a list in `Rullst.toml` replaces it,
    /// and [`SecurityConfig::with_csrf_static_prefix`] adds one. State-changing
    /// requests need the token on every path. Added in 13.0.0-alpha.6, which
    /// replaced the file-extension rule (`.json`, `.js`, `.png`, …).
    #[serde(default = "csrf_static::default_prefixes")]
    pub csrf_static_prefixes: Vec<String>,
    /// Exact paths of static files served outside `csrf_static_prefixes`,
    /// treated the same way. Default `["/favicon.ico", "/robots.txt",
    /// "/sitemap.xml", "/manifest.webmanifest", "/sw.js"]`; a list in
    /// `Rullst.toml` replaces it, and [`SecurityConfig::with_csrf_static_path`]
    /// adds one. Wildcards and route parameters are rejected, as in
    /// `csrf_signed_webhook_paths`. Added in 13.0.0-alpha.6.
    #[serde(default = "csrf_static::default_paths")]
    pub csrf_static_paths: Vec<String>,
    /// Exact request paths whose bodies the WAF neither reads nor inspects,
    /// for routes that must accept free text its signatures refuse, such as
    /// code samples. Each entry is a deliberate risk acceptance for that
    /// route: only its handler (parameterized SQL, output encoding,
    /// validation) protects it from what the body carries. The path, query
    /// string, cookies, `Referer` and `User-Agent` stay inspected. Wildcards
    /// and route parameters are rejected, as in `csrf_signed_webhook_paths`.
    #[serde(default)]
    pub waf_body_exempt_paths: Vec<String>,
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
    /// `max-age` of the `Strict-Transport-Security` header, in seconds: how
    /// long browsers keep using HTTPS for the host. Default 31,536,000 (one
    /// year); `0` tells browsers to forget the policy.
    #[serde(default = "default_hsts_max_age_seconds")]
    pub hsts_max_age_seconds: u64,
    /// Add `includeSubDomains` to HSTS (default `true`), so browsers also use
    /// HTTPS for every subdomain of the host.
    #[serde(default = "default_true")]
    pub hsts_include_subdomains: bool,
    /// Add `preload` to HSTS (default `false`). This consents to the browsers'
    /// built-in HSTS preload list once the domain is submitted there, which is
    /// slow to undo; [`SecurityConfig::validate`] requires
    /// `hsts_include_subdomains` and a `hsts_max_age_seconds` of at least
    /// 31,536,000 with it, as the list does.
    #[serde(default = "default_false")]
    pub hsts_preload: bool,
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

fn default_true() -> bool {
    true
}

/// One year, the HSTS `max-age` most deployments use.
fn default_hsts_max_age_seconds() -> u64 {
    31_536_000
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
            csrf_static_prefixes: csrf_static::default_prefixes(),
            csrf_static_paths: csrf_static::default_paths(),
            waf_body_exempt_paths: Vec::new(),
            trusted_proxies: Vec::new(),
            trusted_proxy_header: default_trusted_proxy_header(),
            trust_forwarded_proto: false,
            hsts_max_age_seconds: default_hsts_max_age_seconds(),
            hsts_include_subdomains: true,
            hsts_preload: false,
        }
    }
}

/// The process-wide configuration and whether it is only the defaults that an
/// early [`RullstConfig::global`] read installed.
struct GlobalSlot {
    config: &'static RullstConfig,
    from_defaults: bool,
}

static GLOBAL_CONFIG: GlobalCell = std::sync::RwLock::new(None);

type GlobalCell = std::sync::RwLock<Option<GlobalSlot>>;

fn read_global(cell: &'static GlobalCell) -> &'static RullstConfig {
    if let Some(slot) = cell
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
    {
        return slot.config;
    }
    cell.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get_or_insert_with(|| GlobalSlot {
            // One small allocation, made at most once for the defaults and
            // once for the configuration that replaces them.
            config: Box::leak(Box::new(RullstConfig::default())),
            from_defaults: true,
        })
        .config
}

fn publish_in(cell: &GlobalCell, config: &RullstConfig) -> GlobalOutcome {
    let mut slot = cell
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (outcome, replace) = match slot.as_ref() {
        None => (GlobalOutcome::Installed, true),
        Some(current) => {
            let differs = *current.config != *config;
            if current.from_defaults {
                (GlobalOutcome::ReplacedDefaults { differs }, true)
            } else {
                (GlobalOutcome::Kept { differs }, false)
            }
        }
    };
    if replace {
        *slot = Some(GlobalSlot {
            config: Box::leak(Box::new(config.clone())),
            from_defaults: false,
        });
    }
    outcome
}

/// What [`RullstConfig::set_global`] did with the configuration it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) enum GlobalOutcome {
    /// Nothing was installed yet; the configuration is now the global one.
    Installed,
    /// [`RullstConfig::global`] had already handed out the defaults; the
    /// configuration replaced them. `differs` is whether it is not the defaults.
    ReplacedDefaults { differs: bool },
    /// A configuration was set explicitly before and stays. `differs` is whether
    /// the one offered now is another.
    Kept { differs: bool },
}

/// `RULLST_ENV` (else `APP_ENV`) from the `.env` the last started `Server`
/// read, which `Server` places between the process variables and `[app].env`.
static PROJECT_ENVIRONMENT_SELECTOR: std::sync::RwLock<Option<String>> =
    std::sync::RwLock::new(None);

/// Records the `.env` environment selector of a starting `Server`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn record_project_environment_selector(selector: Option<String>) {
    *PROJECT_ENVIRONMENT_SELECTOR
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = selector;
}

fn project_environment_selector() -> Option<String> {
    PROJECT_ENVIRONMENT_SELECTOR
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

impl RullstConfig {
    /// Gets the global configuration reference.
    ///
    /// A [`crate::Server`] publishes the configuration it loaded from
    /// `Rullst.toml` when it starts. A read before that returns the defaults;
    /// the `Server` then replaces them, so every later call sees the real
    /// configuration, and it logs a warning naming this call when the two
    /// differ. A reference obtained before the replacement keeps the defaults:
    /// read the configuration after the `Server` has started, or load it with
    /// [`RullstConfig::load_from_file`] and pass it to [`RullstConfig::set_global`].
    pub fn global() -> &'static RullstConfig {
        read_global(&GLOBAL_CONFIG)
    }

    /// Sets the global configuration instance.
    ///
    /// Returns the configuration back when another one was set explicitly
    /// before. Defaults that an earlier [`RullstConfig::global`] read installed
    /// do not count: they are replaced.
    #[allow(clippy::result_large_err)]
    pub fn set_global(config: Self) -> Result<(), Self> {
        match Self::publish_global(&config) {
            GlobalOutcome::Kept { .. } => Err(config),
            GlobalOutcome::Installed | GlobalOutcome::ReplacedDefaults { .. } => Ok(()),
        }
    }

    /// [`Self::set_global`] that also says what happened to an earlier value.
    pub(crate) fn publish_global(config: &Self) -> GlobalOutcome {
        publish_in(&GLOBAL_CONFIG, config)
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
    ///
    /// The precedence is the process `RULLST_ENV`, then the process `APP_ENV`,
    /// then — once a [`crate::Server`] has started in this process —
    /// `RULLST_ENV` or `APP_ENV` from the `.env` file it read, then
    /// `[app].env`. This is the environment the `Server` enforces. Before a
    /// `Server` starts (or without one) no `.env` is consulted, exactly like
    /// [`Environment::detect`], which itself never reads `.env`.
    pub fn environment(&self) -> Result<Environment, ConfigError> {
        let selector = project_environment_selector();
        let rullst_env = read_environment_variable("RULLST_ENV")?;
        let app_env = read_environment_variable("APP_ENV")?;
        Environment::resolve(
            rullst_env.as_deref(),
            app_env.as_deref(),
            selector.as_deref().or(self.app.env.as_deref()),
        )
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
