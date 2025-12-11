//! Application configuration value object

use serde::{Deserialize, Serialize};

use crate::domain::recording::Duration;
use crate::domain::transcription::DomainId;

/// Application configuration.
/// All fields are optional to support partial configs and merging.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_key: Option<String>,
    pub duration: Option<String>,
    pub max_duration: Option<String>,
    pub domain: Option<String>,
    pub clipboard: Option<bool>,
    pub keystroke: Option<bool>,
    pub notify: Option<bool>,
}

impl AppConfig {
    /// Create config with default values
    pub fn defaults() -> Self {
        Self {
            api_key: None,
            duration: Some("10s".to_string()),
            max_duration: Some("60s".to_string()),
            domain: Some("general".to_string()),
            clipboard: Some(false),
            keystroke: Some(false),
            notify: Some(false),
        }
    }

    /// Create an empty config (all None)
    pub fn empty() -> Self {
        Self::default()
    }

    /// Merge this config with another, where other takes precedence.
    /// Only non-None values from other will override this.
    pub fn merge(self, other: Self) -> Self {
        Self {
            api_key: other.api_key.or(self.api_key),
            duration: other.duration.or(self.duration),
            max_duration: other.max_duration.or(self.max_duration),
            domain: other.domain.or(self.domain),
            clipboard: other.clipboard.or(self.clipboard),
            keystroke: other.keystroke.or(self.keystroke),
            notify: other.notify.or(self.notify),
        }
    }

    /// Get duration as parsed Duration, or default if not set/invalid
    pub fn duration_or_default(&self) -> Duration {
        self.duration
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Duration::default_duration)
    }

    /// Get max_duration as parsed Duration, or default if not set/invalid
    pub fn max_duration_or_default(&self) -> Duration {
        self.max_duration
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Duration::default_max_duration)
    }

    /// Get domain as parsed DomainId, or default if not set/invalid
    pub fn domain_or_default(&self) -> DomainId {
        self.domain
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }

    /// Get clipboard setting, or false if not set
    pub fn clipboard_or_default(&self) -> bool {
        self.clipboard.unwrap_or(false)
    }

    /// Get keystroke setting, or false if not set
    pub fn keystroke_or_default(&self) -> bool {
        self.keystroke.unwrap_or(false)
    }

    /// Get notify setting, or false if not set
    pub fn notify_or_default(&self) -> bool {
        self.notify.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_expected_values() {
        let config = AppConfig::defaults();
        assert!(config.api_key.is_none());
        assert_eq!(config.duration, Some("10s".to_string()));
        assert_eq!(config.max_duration, Some("60s".to_string()));
        assert_eq!(config.domain, Some("general".to_string()));
        assert_eq!(config.clipboard, Some(false));
        assert_eq!(config.keystroke, Some(false));
        assert_eq!(config.notify, Some(false));
    }

    #[test]
    fn empty_has_all_none() {
        let config = AppConfig::empty();
        assert!(config.api_key.is_none());
        assert!(config.duration.is_none());
        assert!(config.domain.is_none());
        assert!(config.clipboard.is_none());
    }

    #[test]
    fn merge_other_takes_precedence() {
        let base = AppConfig {
            api_key: Some("base_key".to_string()),
            duration: Some("10s".to_string()),
            domain: Some("general".to_string()),
            ..Default::default()
        };

        let other = AppConfig {
            api_key: Some("other_key".to_string()),
            duration: None, // Should not override
            domain: Some("dev".to_string()),
            ..Default::default()
        };

        let merged = base.merge(other);

        assert_eq!(merged.api_key, Some("other_key".to_string()));
        assert_eq!(merged.duration, Some("10s".to_string())); // Kept from base
        assert_eq!(merged.domain, Some("dev".to_string()));
    }

    #[test]
    fn merge_preserves_base_when_other_is_none() {
        let base = AppConfig {
            api_key: Some("key".to_string()),
            clipboard: Some(true),
            ..Default::default()
        };

        let other = AppConfig::empty();
        let merged = base.merge(other);

        assert_eq!(merged.api_key, Some("key".to_string()));
        assert_eq!(merged.clipboard, Some(true));
    }

    #[test]
    fn duration_or_default_parses() {
        let config = AppConfig {
            duration: Some("30s".to_string()),
            ..Default::default()
        };
        assert_eq!(config.duration_or_default().as_secs(), 30);
    }

    #[test]
    fn duration_or_default_uses_default_on_invalid() {
        let config = AppConfig {
            duration: Some("invalid".to_string()),
            ..Default::default()
        };
        assert_eq!(config.duration_or_default().as_secs(), 10);
    }

    #[test]
    fn duration_or_default_uses_default_on_none() {
        let config = AppConfig::empty();
        assert_eq!(config.duration_or_default().as_secs(), 10);
    }

    #[test]
    fn domain_or_default_parses() {
        let config = AppConfig {
            domain: Some("dev".to_string()),
            ..Default::default()
        };
        assert_eq!(config.domain_or_default(), DomainId::Dev);
    }

    #[test]
    fn domain_or_default_uses_default_on_invalid() {
        let config = AppConfig {
            domain: Some("invalid".to_string()),
            ..Default::default()
        };
        assert_eq!(config.domain_or_default(), DomainId::General);
    }

    #[test]
    fn boolean_defaults() {
        let config = AppConfig::empty();
        assert!(!config.clipboard_or_default());
        assert!(!config.keystroke_or_default());
        assert!(!config.notify_or_default());
    }
}
