//! XDG config store adapter

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::application::ports::ConfigStore;
use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;

/// XDG-compliant config store
pub struct XdgConfigStore {
    path: PathBuf,
}

impl XdgConfigStore {
    /// Create a new XDG config store with default path
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("smart-scribe");

        Self {
            path: config_dir.join("config.toml"),
        }
    }

    /// Create with custom path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Parse TOML content into AppConfig
    fn parse_toml(content: &str) -> Result<AppConfig, ConfigError> {
        // Try to parse as flat format first
        let config: AppConfig = toml::from_str(content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Serialize AppConfig to TOML
    fn to_toml(config: &AppConfig) -> Result<String, ConfigError> {
        toml::to_string_pretty(config)
            .map_err(|e| ConfigError::WriteError(e.to_string()))
    }
}

impl Default for XdgConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigStore for XdgConfigStore {
    async fn load(&self) -> Result<AppConfig, ConfigError> {
        if !self.exists() {
            // Return empty config if file doesn't exist
            return Ok(AppConfig::empty());
        }

        let content = fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::ReadError(e.to_string()))?;

        Self::parse_toml(&content)
    }

    async fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ConfigError::WriteError(e.to_string()))?;
        }

        let content = Self::to_toml(config)?;

        fs::write(&self.path, content)
            .await
            .map_err(|e| ConfigError::WriteError(e.to_string()))?;

        Ok(())
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    async fn init(&self) -> Result<(), ConfigError> {
        if self.exists() {
            return Err(ConfigError::AlreadyExists(
                self.path.to_string_lossy().to_string(),
            ));
        }

        let defaults = AppConfig::defaults();
        self.save(&defaults).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_xdg() {
        let store = XdgConfigStore::new();
        let path = store.path();
        assert!(path.to_string_lossy().contains("smart-scribe"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    fn custom_path() {
        let store = XdgConfigStore::with_path("/custom/path/config.toml");
        assert_eq!(store.path(), PathBuf::from("/custom/path/config.toml"));
    }

    #[test]
    fn parse_toml_flat_format() {
        let content = r#"
api_key = "test-key"
duration = "30s"
domain = "dev"
clipboard = true
"#;

        let config = XdgConfigStore::parse_toml(content).unwrap();
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.duration, Some("30s".to_string()));
        assert_eq!(config.domain, Some("dev".to_string()));
        assert_eq!(config.clipboard, Some(true));
    }

    #[test]
    fn to_toml_round_trip() {
        let config = AppConfig {
            api_key: Some("test-key".to_string()),
            duration: Some("30s".to_string()),
            domain: Some("dev".to_string()),
            clipboard: Some(true),
            ..Default::default()
        };

        let toml = XdgConfigStore::to_toml(&config).unwrap();
        let parsed = XdgConfigStore::parse_toml(&toml).unwrap();

        assert_eq!(config.api_key, parsed.api_key);
        assert_eq!(config.duration, parsed.duration);
        assert_eq!(config.domain, parsed.domain);
        assert_eq!(config.clipboard, parsed.clipboard);
    }
}
