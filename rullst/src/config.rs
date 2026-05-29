use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct RullstConfig {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct AppConfig {
    pub env: Option<String>,
    pub port: Option<u16>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            env: None,
            port: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DatabaseConfig {
    pub url: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { url: None }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct SecurityConfig {
    #[serde(default = "default_same_site")]
    pub csrf_same_site: String,
    #[serde(default)]
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

impl Default for RullstConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl RullstConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: RullstConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
