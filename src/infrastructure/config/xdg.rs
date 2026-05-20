//! XDG config store adapter

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::application::ports::ConfigStore;
use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;

/// Legacy keys removed in the OpenAI-only rewrite.
///
/// When [`XdgConfigStore::load`] sees any of these at the top level it strips
/// them in-place (preserving comments and unrelated lines) and prints a
/// one-line notice on stderr. Subsequent loads then run silently.
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

    /// Parse TOML content into AppConfig.
    ///
    /// Legacy keys are silently dropped here (they no longer exist in
    /// [`AppConfig`]). User-facing notice is handled by [`Self::load`] which
    /// also rewrites the file.
    fn parse_toml(
        content: &str,
        _path_for_warning: Option<&PathBuf>,
    ) -> Result<AppConfig, ConfigError> {
        let config: AppConfig =
            toml::from_str(content).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        Ok(config)
    }

    /// Surgically remove top-level legacy keys from a TOML document.
    ///
    /// Works on a line-by-line basis to preserve comments, blank lines, and
    /// any unrelated formatting the user added by hand. Keys inside a
    /// `[table]` section are left alone (they can't be the deprecated
    /// top-level keys by definition).
    fn strip_legacy_keys(content: &str) -> (String, Vec<&'static str>) {
        let mut output = String::with_capacity(content.len());
        let mut removed: Vec<&'static str> = Vec::new();
        let mut in_table = false;

        for line in content.split_inclusive('\n') {
            let trimmed = line.trim_start();
            // A `[section]` header takes us out of the top-level scope.
            if trimmed.starts_with('[') {
                in_table = true;
                output.push_str(line);
                continue;
            }
            if !in_table {
                if let Some(key_part) = trimmed.split('=').next() {
                    let key = key_part.trim();
                    if let Some(&legacy) = LEGACY_KEYS.iter().find(|&&k| k == key) {
                        removed.push(legacy);
                        continue;
                    }
                }
            }
            output.push_str(line);
        }
        (output, removed)
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

        let (cleaned, removed) = Self::strip_legacy_keys(&content);
        if !removed.is_empty() {
            match fs::write(&self.path, &cleaned).await {
                Ok(()) => eprintln!(
                    "smart-scribe: removed deprecated keys ({}) from {}",
                    removed.join(", "),
                    self.path.display()
                ),
                Err(e) => eprintln!(
                    "warning: {} contains deprecated keys ({}) and could not be auto-cleaned: {e}",
                    self.path.display(),
                    removed.join(", "),
                ),
            }
        }

        Self::parse_toml(&cleaned, Some(&self.path))
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
    fn strip_legacy_keys_removes_top_level_keys() {
        let input = r#"# my smart-scribe config
api_key = "old"
backend = "gemini"
auth = "oauth"
openai_transcribe_model = "gpt-4o-transcribe"
chatgpt_cookie_file = "/tmp/c.json"
domain = "dev"
"#;
        let (out, removed) = XdgConfigStore::strip_legacy_keys(input);
        assert!(out.contains("# my smart-scribe config"));
        assert!(out.contains("auth = \"oauth\""));
        assert!(out.contains("openai_transcribe_model"));
        assert!(!out.contains("api_key"));
        assert!(!out.contains("backend"));
        assert!(!out.contains("chatgpt_cookie_file"));
        assert!(!out.contains("\ndomain = "));
        assert_eq!(removed.len(), 4);
    }

    #[test]
    fn strip_legacy_keys_is_idempotent() {
        let input = "api_key = \"x\"\nauth = \"oauth\"\n";
        let (once, removed1) = XdgConfigStore::strip_legacy_keys(input);
        let (twice, removed2) = XdgConfigStore::strip_legacy_keys(&once);
        assert_eq!(removed1, vec!["api_key"]);
        assert!(removed2.is_empty());
        assert_eq!(once, twice);
    }

    #[test]
    fn strip_legacy_keys_preserves_keys_inside_table() {
        // A nested `api_key` (inside some hypothetical `[creds]` table)
        // must not be stripped.
        let input = r#"auth = "oauth"
[creds]
api_key = "keep-me"
"#;
        let (out, removed) = XdgConfigStore::strip_legacy_keys(input);
        assert!(removed.is_empty(), "got removed={removed:?}");
        assert!(out.contains("api_key = \"keep-me\""));
    }

    #[test]
    fn strip_legacy_keys_ignores_prefix_collisions() {
        // `api_key_other` is NOT a legacy key and must be kept.
        let input = "api_key_other = \"value\"\napi_key = \"legacy\"\n";
        let (out, removed) = XdgConfigStore::strip_legacy_keys(input);
        assert_eq!(removed, vec!["api_key"]);
        assert!(out.contains("api_key_other = \"value\""));
        assert!(!out.contains("api_key = \"legacy\""));
    }

    #[test]
    fn strip_legacy_keys_handles_leading_whitespace() {
        let input = "  api_key = \"x\"\n\tdomain = \"dev\"\nauth = \"oauth\"\n";
        let (out, removed) = XdgConfigStore::strip_legacy_keys(input);
        assert_eq!(removed.len(), 2);
        assert!(!out.contains("api_key"));
        assert!(!out.contains("domain"));
        assert!(out.contains("auth = \"oauth\""));
    }

    #[test]
    fn strip_legacy_keys_no_change_when_clean() {
        let input = "auth = \"oauth\"\nopenai_transcribe_model = \"gpt-4o-transcribe\"\n";
        let (out, removed) = XdgConfigStore::strip_legacy_keys(input);
        assert_eq!(out, input);
        assert!(removed.is_empty());
    }

    #[tokio::test]
    async fn load_rewrites_file_when_legacy_keys_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# user note\napi_key = \"old\"\nbackend = \"gemini\"\nauth = \"oauth\"\n",
        )
        .unwrap();

        let store = XdgConfigStore::with_path(&path);
        let config = store.load().await.unwrap();
        assert_eq!(config.auth.as_deref(), Some("oauth"));

        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert!(rewritten.contains("# user note"));
        assert!(rewritten.contains("auth = \"oauth\""));
        assert!(!rewritten.contains("api_key"));
        assert!(!rewritten.contains("backend"));

        // Second load is a no-op (no further mutation).
        let again = std::fs::read_to_string(&path).unwrap();
        let _ = store.load().await.unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(again, after);
    }

    #[tokio::test]
    async fn load_clean_file_does_not_touch_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = "auth = \"oauth\"\n";
        std::fs::write(&path, original).unwrap();
        let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

        // Sleep briefly so a write would show up in mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));

        let store = XdgConfigStore::with_path(&path);
        let _ = store.load().await.unwrap();

        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(
            mtime_before, mtime_after,
            "clean config must not be rewritten"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
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
