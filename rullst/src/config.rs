use serde::Deserialize;

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
}

fn default_csp() -> String {
    "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval';".to_string()
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

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            csrf_same_site: default_same_site(),
            cors_allow_origins: vec![],
            csp: default_csp(),
            user_agent_blocklist: default_user_agent_blocklist(),
        }
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

    /// Loads and parses the configuration from a TOML file.
    pub async fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: RullstConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

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
}
