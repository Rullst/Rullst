use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// [TODO] Missing documentation.
pub struct RullstConfig {
    #[serde(default)]
    /// [TODO] Missing documentation.
    pub app: AppConfig,
    #[serde(default)]
    /// [TODO] Missing documentation.
    pub database: DatabaseConfig,
    #[serde(default)]
    /// [TODO] Missing documentation.
    pub security: SecurityConfig,
    #[serde(default)]
    /// [TODO] Missing documentation.
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// [TODO] Missing documentation.
pub struct StorageConfig {
    /// [TODO] Missing documentation.
    pub root: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// [TODO] Missing documentation.
pub struct AppConfig {
    /// [TODO] Missing documentation.
    pub env: Option<String>,
    /// [TODO] Missing documentation.
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
/// [TODO] Missing documentation.
pub struct DatabaseConfig {
    /// [TODO] Missing documentation.
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
/// [TODO] Missing documentation.
pub struct SecurityConfig {
    #[serde(default = "default_same_site")]
    /// [TODO] Missing documentation.
    pub csrf_same_site: String,
    #[serde(default)]
    /// [TODO] Missing documentation.
    pub cors_allow_origins: Vec<String>,
}

fn default_same_site() -> String {
    "Lax".to_string()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            csrf_same_site: default_same_site(),
            cors_allow_origins: vec![],
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

    /// [TODO] Missing documentation.
    pub fn new() -> Self {
        Self::default()
    }

    /// [TODO] Missing documentation.
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
