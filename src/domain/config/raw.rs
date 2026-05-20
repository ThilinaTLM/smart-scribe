//! Raw (persisted) configuration schema.
//!
//! This is the on-disk shape: every field is optional so partial configs and
//! layer-by-layer merging work cleanly. The validated runtime type that the
//! rest of the codebase consumes lives in [`super::AppConfig`].
//!
//! The split exists to avoid primitive-obsession in the runtime type — every
//! `*_or_default()` helper we used to need on a unified config went away once
//! validation happened in one place ([`super::AppConfig::try_from`]).
//!
//! The type derives `Serialize` / `Deserialize` so it can be loaded from TOML
//! by the infrastructure adapter, but the TOML crate itself never leaks into
//! the domain.

use serde::{Deserialize, Serialize};

use super::app_config::DEFAULT_OPENAI_TRANSCRIBE_MODEL;
use super::AuthMode;

/// Linux-specific raw configuration (all fields optional).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawLinuxConfig {
    pub keystroke_tool: Option<String>,
    pub indicator: Option<bool>,
    pub indicator_position: Option<String>,
    pub paste: Option<bool>,
}

/// Windows-specific raw configuration (all fields optional).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawWindowsConfig {
    pub indicator: Option<bool>,
    pub show_balloon: Option<bool>,
}

/// Raw (unvalidated) application configuration as persisted to disk.
///
/// All fields are optional to support partial configs and layered merging
/// (`defaults → file → env → CLI`). Convert to the validated runtime type
/// with [`AppConfig::try_from`](super::AppConfig::try_from).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawAppConfig {
    pub auth: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_transcribe_model: Option<String>,
    pub transcribe_prompt: Option<String>,
    pub transcribe_language: Option<String>,
    pub duration: Option<String>,
    pub max_duration: Option<String>,
    pub clipboard: Option<bool>,
    pub keystroke: Option<bool>,
    pub notify: Option<bool>,
    pub audio_cue: Option<bool>,
    pub linux: Option<RawLinuxConfig>,
    pub windows: Option<RawWindowsConfig>,
}

impl RawAppConfig {
    /// Static default values, used as the bottom layer of the merge stack.
    pub fn defaults() -> Self {
        Self {
            auth: Some(AuthMode::default().to_string()),
            openai_api_key: None,
            openai_transcribe_model: Some(DEFAULT_OPENAI_TRANSCRIBE_MODEL.to_string()),
            transcribe_prompt: None,
            transcribe_language: None,
            duration: None,
            max_duration: None,
            clipboard: Some(false),
            keystroke: Some(false),
            notify: Some(false),
            audio_cue: Some(false),
            linux: Some(RawLinuxConfig {
                keystroke_tool: Some("enigo".to_string()),
                indicator: Some(false),
                indicator_position: Some("top-right".to_string()),
                paste: Some(false),
            }),
            windows: Some(RawWindowsConfig {
                indicator: Some(false),
                show_balloon: Some(false),
            }),
        }
    }

    /// Empty config (all `None`). Equivalent to `Self::default()`.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Merge `other` over `self`: any field set in `other` wins.
    ///
    /// Designed for layered configuration (`defaults → file → env → CLI`).
    pub fn merge(self, other: Self) -> Self {
        Self {
            auth: other.auth.or(self.auth),
            openai_api_key: other.openai_api_key.or(self.openai_api_key),
            openai_transcribe_model: other
                .openai_transcribe_model
                .or(self.openai_transcribe_model),
            transcribe_prompt: other.transcribe_prompt.or(self.transcribe_prompt),
            transcribe_language: other.transcribe_language.or(self.transcribe_language),
            duration: other.duration.or(self.duration),
            max_duration: other.max_duration.or(self.max_duration),
            clipboard: other.clipboard.or(self.clipboard),
            keystroke: other.keystroke.or(self.keystroke),
            notify: other.notify.or(self.notify),
            audio_cue: other.audio_cue.or(self.audio_cue),
            linux: merge_linux(self.linux, other.linux),
            windows: merge_windows(self.windows, other.windows),
        }
    }
}

fn merge_linux(
    base: Option<RawLinuxConfig>,
    other: Option<RawLinuxConfig>,
) -> Option<RawLinuxConfig> {
    match (base, other) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(b), Some(o)) => Some(RawLinuxConfig {
            keystroke_tool: o.keystroke_tool.or(b.keystroke_tool),
            indicator: o.indicator.or(b.indicator),
            indicator_position: o.indicator_position.or(b.indicator_position),
            paste: o.paste.or(b.paste),
        }),
    }
}

fn merge_windows(
    base: Option<RawWindowsConfig>,
    other: Option<RawWindowsConfig>,
) -> Option<RawWindowsConfig> {
    match (base, other) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(b), Some(o)) => Some(RawWindowsConfig {
            indicator: o.indicator.or(b.indicator),
            show_balloon: o.show_balloon.or(b.show_balloon),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_expected_values() {
        let config = RawAppConfig::defaults();
        assert_eq!(config.auth.as_deref(), Some("oauth"));
        assert_eq!(
            config.openai_transcribe_model.as_deref(),
            Some(DEFAULT_OPENAI_TRANSCRIBE_MODEL)
        );
        assert_eq!(config.clipboard, Some(false));
    }

    #[test]
    fn empty_has_all_none() {
        let config = RawAppConfig::empty();
        assert!(config.auth.is_none());
        assert!(config.openai_api_key.is_none());
        assert!(config.linux.is_none());
    }

    #[test]
    fn merge_other_takes_precedence() {
        let base = RawAppConfig {
            openai_api_key: Some("base".into()),
            duration: Some("10s".into()),
            auth: Some("oauth".into()),
            ..Default::default()
        };
        let other = RawAppConfig {
            openai_api_key: Some("other".into()),
            duration: None,
            auth: Some("api_key".into()),
            ..Default::default()
        };
        let merged = base.merge(other);
        assert_eq!(merged.openai_api_key.as_deref(), Some("other"));
        assert_eq!(merged.duration.as_deref(), Some("10s"));
        assert_eq!(merged.auth.as_deref(), Some("api_key"));
    }

    #[test]
    fn merge_preserves_base_when_other_is_none() {
        let base = RawAppConfig {
            openai_api_key: Some("k".into()),
            clipboard: Some(true),
            ..Default::default()
        };
        let merged = base.merge(RawAppConfig::empty());
        assert_eq!(merged.openai_api_key.as_deref(), Some("k"));
        assert_eq!(merged.clipboard, Some(true));
    }

    #[test]
    fn merge_linux_keystroke_tool() {
        let base = RawAppConfig {
            linux: Some(RawLinuxConfig {
                keystroke_tool: Some("enigo".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let other = RawAppConfig {
            linux: Some(RawLinuxConfig {
                keystroke_tool: Some("xdotool".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let merged = base.merge(other);
        assert_eq!(
            merged.linux.unwrap().keystroke_tool.as_deref(),
            Some("xdotool")
        );
    }

    #[test]
    fn merge_windows_indicator_field() {
        let base = RawAppConfig {
            windows: Some(RawWindowsConfig {
                indicator: Some(false),
                show_balloon: Some(false),
            }),
            ..Default::default()
        };
        let other = RawAppConfig {
            windows: Some(RawWindowsConfig {
                indicator: Some(true),
                show_balloon: None,
            }),
            ..Default::default()
        };
        let merged = base.merge(other);
        let w = merged.windows.unwrap();
        assert_eq!(w.indicator, Some(true));
        assert_eq!(w.show_balloon, Some(false));
    }
}
