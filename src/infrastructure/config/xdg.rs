//! XDG config store adapter

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::application::ports::ConfigStore;
use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;

/// Legacy keys removed in the OpenAI-only rewrite. When present we warn the
/// user once so they know to clean up their config.
const LEGACY_KEYS: &[&str] = &["api_key", "backend", "chatgpt_cookie_file", "domain"];

/// XDG-compliant config store
pub struct XdgConfigStore {
    path: PathBuf,
}

impl XdgConfigStore {
    /// Create a new XDG config store with default path
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .or_else(|| {
                // Fallback: use home_dir/.config on Unix-like systems
                dirs::home_dir().map(|home| home.join(".config"))
            })
            .unwrap_or_else(|| {
                // Last resort: use current directory
                PathBuf::from(".")
            })
            .join("smart-scribe");

        Self {
            path: config_dir.join("config.toml"),
        }
    }

    /// Create with custom path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Parse TOML content into AppConfig, warning about deprecated keys.
    fn parse_toml(
        content: &str,
        path_for_warning: Option<&PathBuf>,
    ) -> Result<AppConfig, ConfigError> {
        // First pass: detect legacy keys at the top level.
        if let Ok(toml::Value::Table(table)) = content.parse::<toml::Value>() {
            let stale: Vec<&str> = LEGACY_KEYS
                .iter()
                .copied()
                .filter(|k| table.contains_key(*k))
                .collect();
            if !stale.is_empty() {
                let loc = path_for_warning
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "config".to_string());
                eprintln!(
                    "warning: {loc} contains deprecated keys ({}). They are ignored. \
Remove them with `smart-scribe config set` or by editing the file.",
                    stale.join(", ")
                );
            }
        }

        let config: AppConfig =
            toml::from_str(content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Serialize AppConfig to TOML
    fn to_toml(config: &AppConfig) -> Result<String, ConfigError> {
        toml::to_string_pretty(config).map_err(|e| ConfigError::WriteError(e.to_string()))
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

        Self::parse_toml(&content, Some(&self.path))
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
auth = "oauth"
openai_api_key = "sk-test"
duration = "30s"
clipboard = true
"#;

        let config = XdgConfigStore::parse_toml(content, None).unwrap();
        assert_eq!(config.auth.as_deref(), Some("oauth"));
        assert_eq!(config.openai_api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.duration, Some("30s".to_string()));
        assert_eq!(config.clipboard, Some(true));
    }

    #[test]
    fn parse_toml_ignores_legacy_keys() {
        let content = r#"
api_key = "old-gemini-key"
backend = "gemini"
chatgpt_cookie_file = "/tmp/cookies.json"
domain = "dev"
auth = "oauth"
"#;
        let config = XdgConfigStore::parse_toml(content, None).unwrap();
        assert_eq!(config.auth.as_deref(), Some("oauth"));
        // Legacy fields are not present in AppConfig anymore.
    }

    #[test]
    fn to_toml_round_trip() {
        let config = AppConfig {
            auth: Some("api_key".to_string()),
            openai_api_key: Some("sk-test".to_string()),
            duration: Some("30s".to_string()),
            clipboard: Some(true),
            ..Default::default()
        };

        let toml = XdgConfigStore::to_toml(&config).unwrap();
        let parsed = XdgConfigStore::parse_toml(&toml, None).unwrap();

        assert_eq!(config.auth, parsed.auth);
        assert_eq!(config.openai_api_key, parsed.openai_api_key);
        assert_eq!(config.duration, parsed.duration);
        assert_eq!(config.clipboard, parsed.clipboard);
    }
}
