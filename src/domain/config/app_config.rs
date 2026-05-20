//! Application configuration value object

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::domain::recording::Duration;

/// Auth mode selecting which transcription backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    /// ChatGPT subscription via the Codex OAuth client.
    #[default]
    Oauth,
    /// OpenAI API key against `api.openai.com/v1/audio/transcriptions`.
    ApiKey,
}

impl AuthMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Oauth => "oauth",
            Self::ApiKey => "api_key",
        }
    }
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AuthMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "oauth" | "chatgpt" => Ok(Self::Oauth),
            "api_key" | "api-key" | "apikey" | "openai" => Ok(Self::ApiKey),
            other => Err(format!(
                "Invalid auth mode '{other}'. Valid options: oauth, api_key"
            )),
        }
    }
}

/// Default transcription model when using the OpenAI API.
pub const DEFAULT_OPENAI_TRANSCRIBE_MODEL: &str = "gpt-4o-mini-transcribe";

/// Linux-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinuxConfig {
    pub keystroke_tool: Option<String>,
    pub indicator: Option<bool>,
    pub indicator_position: Option<String>,
    pub paste: Option<bool>,
}

/// Windows-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowsConfig {
    /// Show system tray icon while the daemon is recording/processing.
    pub indicator: Option<bool>,
    /// Show Windows balloon notifications on state transitions.
    pub show_balloon: Option<bool>,
}

/// Application configuration.
/// All fields are optional to support partial configs and merging.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Auth mode: "oauth" (default) or "api_key".
    pub auth: Option<String>,
    /// OpenAI API key, used when `auth = "api_key"`.
    pub openai_api_key: Option<String>,
    /// Transcription model for the OpenAI API (e.g. `gpt-4o-mini-transcribe`).
    pub openai_transcribe_model: Option<String>,
    pub duration: Option<String>,
    pub max_duration: Option<String>,
    pub clipboard: Option<bool>,
    pub keystroke: Option<bool>,
    pub notify: Option<bool>,
    pub audio_cue: Option<bool>,
    pub linux: Option<LinuxConfig>,
    pub windows: Option<WindowsConfig>,
}

impl AppConfig {
    /// Create config with default values
    pub fn defaults() -> Self {
        Self {
            auth: Some(AuthMode::default().to_string()),
            openai_api_key: None,
            openai_transcribe_model: Some(DEFAULT_OPENAI_TRANSCRIBE_MODEL.to_string()),
            duration: None,
            max_duration: None,
            clipboard: Some(false),
            keystroke: Some(false),
            notify: Some(false),
            audio_cue: Some(false),
            linux: Some(LinuxConfig {
                keystroke_tool: Some("enigo".to_string()),
                indicator: Some(false),
                indicator_position: Some("top-right".to_string()),
                paste: Some(false),
            }),
            windows: Some(WindowsConfig {
                indicator: Some(false),
                show_balloon: Some(false),
            }),
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
            auth: other.auth.or(self.auth),
            openai_api_key: other.openai_api_key.or(self.openai_api_key),
            openai_transcribe_model: other
                .openai_transcribe_model
                .or(self.openai_transcribe_model),
            duration: other.duration.or(self.duration),
            max_duration: other.max_duration.or(self.max_duration),
            clipboard: other.clipboard.or(self.clipboard),
            keystroke: other.keystroke.or(self.keystroke),
            notify: other.notify.or(self.notify),
            audio_cue: other.audio_cue.or(self.audio_cue),
            linux: Self::merge_linux_config(self.linux, other.linux),
            windows: Self::merge_windows_config(self.windows, other.windows),
        }
    }

    /// Merge Linux config sections
    fn merge_linux_config(
        base: Option<LinuxConfig>,
        other: Option<LinuxConfig>,
    ) -> Option<LinuxConfig> {
        match (base, other) {
            (None, None) => None,
            (Some(b), None) => Some(b),
            (None, Some(o)) => Some(o),
            (Some(b), Some(o)) => Some(LinuxConfig {
                keystroke_tool: o.keystroke_tool.or(b.keystroke_tool),
                indicator: o.indicator.or(b.indicator),
                indicator_position: o.indicator_position.or(b.indicator_position),
                paste: o.paste.or(b.paste),
            }),
        }
    }

    /// Merge Windows config sections
    fn merge_windows_config(
        base: Option<WindowsConfig>,
        other: Option<WindowsConfig>,
    ) -> Option<WindowsConfig> {
        match (base, other) {
            (None, None) => None,
            (Some(b), None) => Some(b),
            (None, Some(o)) => Some(o),
            (Some(b), Some(o)) => Some(WindowsConfig {
                indicator: o.indicator.or(b.indicator),
                show_balloon: o.show_balloon.or(b.show_balloon),
            }),
        }
    }

    /// Parse `auth` field. Falls back to [`AuthMode::default`] on missing or
    /// invalid values.
    pub fn auth_or_default(&self) -> AuthMode {
        self.auth
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }

    /// Get the configured OpenAI transcription model, or the default if unset.
    pub fn openai_transcribe_model_or_default(&self) -> &str {
        self.openai_transcribe_model
            .as_deref()
            .unwrap_or(DEFAULT_OPENAI_TRANSCRIBE_MODEL)
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

    /// Get audio_cue setting, or false if not set
    pub fn audio_cue_or_default(&self) -> bool {
        self.audio_cue.unwrap_or(false)
    }

    /// Get indicator setting, or false if not set (Linux only)
    #[cfg(target_os = "linux")]
    pub fn indicator_or_default(&self) -> bool {
        self.linux
            .as_ref()
            .and_then(|l| l.indicator)
            .unwrap_or(false)
    }

    /// Get indicator position setting, or "top-right" if not set (Linux only)
    #[cfg(target_os = "linux")]
    pub fn indicator_position_or_default(&self) -> &str {
        self.linux
            .as_ref()
            .and_then(|l| l.indicator_position.as_deref())
            .unwrap_or("top-right")
    }

    /// Get paste setting, or false if not set (Linux only)
    #[cfg(target_os = "linux")]
    pub fn paste_or_default(&self) -> bool {
        self.linux.as_ref().and_then(|l| l.paste).unwrap_or(false)
    }

    /// Get keystroke tool preference, or "enigo" if not set
    pub fn keystroke_tool_or_default(&self) -> &str {
        self.linux
            .as_ref()
            .and_then(|l| l.keystroke_tool.as_deref())
            .unwrap_or("enigo")
    }

    /// Get indicator setting, or false if not set (Windows only)
    #[cfg(target_os = "windows")]
    pub fn indicator_or_default(&self) -> bool {
        self.windows
            .as_ref()
            .and_then(|w| w.indicator)
            .unwrap_or(false)
    }

    /// Get balloon-notification setting, or false if not set (Windows only)
    #[cfg(target_os = "windows")]
    pub fn show_balloon_or_default(&self) -> bool {
        self.windows
            .as_ref()
            .and_then(|w| w.show_balloon)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_expected_values() {
        let config = AppConfig::defaults();
        assert_eq!(config.auth.as_deref(), Some("oauth"));
        assert_eq!(
            config.openai_transcribe_model.as_deref(),
            Some(DEFAULT_OPENAI_TRANSCRIBE_MODEL)
        );
        assert!(config.openai_api_key.is_none());
        assert!(config.duration.is_none());
        assert!(config.max_duration.is_none());
        assert_eq!(config.clipboard, Some(false));
        assert_eq!(config.keystroke, Some(false));
        assert_eq!(config.notify, Some(false));
        assert_eq!(config.audio_cue, Some(false));
        assert_eq!(config.keystroke_tool_or_default(), "enigo");
        // Linux-specific defaults
        let linux = config.linux.as_ref().unwrap();
        assert_eq!(linux.indicator, Some(false));
        assert_eq!(linux.indicator_position, Some("top-right".to_string()));
        // Windows-specific defaults
        let windows = config.windows.as_ref().unwrap();
        assert_eq!(windows.indicator, Some(false));
        assert_eq!(windows.show_balloon, Some(false));
    }

    #[test]
    fn empty_has_all_none() {
        let config = AppConfig::empty();
        assert!(config.auth.is_none());
        assert!(config.openai_api_key.is_none());
        assert!(config.duration.is_none());
        assert!(config.clipboard.is_none());
        assert!(config.linux.is_none());
        assert!(config.windows.is_none());
    }

    #[test]
    fn merge_other_takes_precedence() {
        let base = AppConfig {
            openai_api_key: Some("base_key".to_string()),
            duration: Some("10s".to_string()),
            auth: Some("oauth".to_string()),
            ..Default::default()
        };

        let other = AppConfig {
            openai_api_key: Some("other_key".to_string()),
            duration: None, // Should not override
            auth: Some("api_key".to_string()),
            ..Default::default()
        };

        let merged = base.merge(other);

        assert_eq!(merged.openai_api_key, Some("other_key".to_string()));
        assert_eq!(merged.duration, Some("10s".to_string())); // Kept from base
        assert_eq!(merged.auth.as_deref(), Some("api_key"));
    }

    #[test]
    fn merge_preserves_base_when_other_is_none() {
        let base = AppConfig {
            openai_api_key: Some("key".to_string()),
            clipboard: Some(true),
            ..Default::default()
        };

        let other = AppConfig::empty();
        let merged = base.merge(other);

        assert_eq!(merged.openai_api_key, Some("key".to_string()));
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
    fn auth_mode_parses() {
        assert_eq!(AuthMode::from_str("oauth"), Ok(AuthMode::Oauth));
        assert_eq!(AuthMode::from_str("OAuth"), Ok(AuthMode::Oauth));
        assert_eq!(AuthMode::from_str("api_key"), Ok(AuthMode::ApiKey));
        assert_eq!(AuthMode::from_str("API-Key"), Ok(AuthMode::ApiKey));
        assert!(AuthMode::from_str("nope").is_err());
    }

    #[test]
    fn auth_or_default_falls_back_to_oauth() {
        let config = AppConfig::empty();
        assert_eq!(config.auth_or_default(), AuthMode::Oauth);
        let bad = AppConfig {
            auth: Some("nonsense".into()),
            ..Default::default()
        };
        assert_eq!(bad.auth_or_default(), AuthMode::Oauth);
    }

    #[test]
    fn boolean_defaults() {
        let config = AppConfig::empty();
        assert!(!config.clipboard_or_default());
        assert!(!config.keystroke_or_default());
        assert!(!config.notify_or_default());
        assert!(!config.audio_cue_or_default());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn indicator_or_default_returns_false() {
        let config = AppConfig::empty();
        assert!(!config.indicator_or_default());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn indicator_position_or_default_returns_top_right() {
        let config = AppConfig::empty();
        assert_eq!(config.indicator_position_or_default(), "top-right");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn indicator_position_or_default_returns_configured() {
        let config = AppConfig {
            linux: Some(LinuxConfig {
                indicator_position: Some("bottom-left".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(config.indicator_position_or_default(), "bottom-left");
    }

    #[test]
    fn keystroke_tool_or_default_returns_platform_default() {
        let config = AppConfig::empty();
        assert_eq!(config.keystroke_tool_or_default(), "enigo");
    }

    #[test]
    fn keystroke_tool_or_default_returns_configured() {
        let config = AppConfig {
            linux: Some(LinuxConfig {
                keystroke_tool: Some("xdotool".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(config.keystroke_tool_or_default(), "xdotool");
    }

    #[test]
    fn merge_linux_config() {
        let base = AppConfig {
            linux: Some(LinuxConfig {
                keystroke_tool: Some("enigo".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let other = AppConfig {
            linux: Some(LinuxConfig {
                keystroke_tool: Some("xdotool".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let merged = base.merge(other);
        assert_eq!(merged.keystroke_tool_or_default(), "xdotool");
    }

    #[test]
    fn merge_linux_config_preserves_base() {
        let base = AppConfig {
            linux: Some(LinuxConfig {
                keystroke_tool: Some("ydotool".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let other = AppConfig::empty();
        let merged = base.merge(other);
        assert_eq!(merged.keystroke_tool_or_default(), "ydotool");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_indicator_or_default_returns_false() {
        let config = AppConfig::empty();
        assert!(!config.indicator_or_default());
        assert!(!config.show_balloon_or_default());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_indicator_or_default_returns_configured() {
        let config = AppConfig {
            windows: Some(WindowsConfig {
                indicator: Some(true),
                show_balloon: Some(true),
            }),
            ..Default::default()
        };
        assert!(config.indicator_or_default());
        assert!(config.show_balloon_or_default());
    }

    #[test]
    fn merge_windows_config_indicator_field() {
        let base = AppConfig {
            windows: Some(WindowsConfig {
                indicator: Some(false),
                show_balloon: Some(false),
            }),
            ..Default::default()
        };
        let other = AppConfig {
            windows: Some(WindowsConfig {
                indicator: Some(true),
                show_balloon: None,
            }),
            ..Default::default()
        };
        let merged = base.merge(other);
        let w = merged.windows.as_ref().unwrap();
        assert_eq!(w.indicator, Some(true));
        assert_eq!(w.show_balloon, Some(false));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn merge_linux_config_indicator_fields() {
        let base = AppConfig {
            linux: Some(LinuxConfig {
                indicator: Some(false),
                indicator_position: Some("top-right".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let other = AppConfig {
            linux: Some(LinuxConfig {
                indicator: Some(true),
                indicator_position: Some("bottom-left".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let merged = base.merge(other);
        assert!(merged.indicator_or_default());
        assert_eq!(merged.indicator_position_or_default(), "bottom-left");
    }
}
