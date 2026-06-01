use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
pub struct RullstConfig {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
pub struct AppConfig {
    pub env: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[non_exhaustive]
pub struct DatabaseConfig {
    pub url: Option<String>,
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

impl RullstConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: RullstConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
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
