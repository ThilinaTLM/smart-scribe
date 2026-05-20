//! Declarative registry of recognised configuration keys.
//!
//! Each entry exposes four operations on the persisted [`RawAppConfig`]:
//! - **validate**: reject malformed values *before* loading the file.
//! - **set**: install the value into the right nested field.
//! - **get**: read the current value (used by `config get` and `config list`).
//! - **display**: render a value for output (identity for most keys, masks
//!   for `openai_api_key`).
//!
//! Adding a new key is a single entry in [`KEYS`]; the `config_cmd` handlers
//! iterate the registry rather than maintaining four parallel match blocks.

use crate::domain::config::{AuthMode, RawAppConfig, RawLinuxConfig, RawWindowsConfig};
use crate::domain::error::ConfigError;
use crate::domain::recording::Duration;

/// Default keystroke tool name.
pub const KEYSTROKE_TOOL_ENIGO: &str = "enigo";

/// Accepted auth-mode strings (for CLI error messages).
pub const VALID_AUTH_MODES: &[&str] = &["oauth", "api_key"];

/// Accepted keystroke-tool strings. `enigo` is the portable default; the
/// other backends are Linux-only at runtime but stay valid in the schema so a
/// portable config can target Linux from any host.
pub const VALID_KEYSTROKE_TOOLS: &[&str] = &["enigo", "auto", "ydotool", "xdotool", "wtype"];

/// Accepted indicator positions (Linux overlay).
const VALID_INDICATOR_POSITIONS: &[&str] = &[
    "top-right",
    "top-left",
    "top-center",
    "bottom-center",
    "bottom-right",
    "bottom-left",
];

/// One entry in the schema.
#[derive(Clone, Copy)]
pub struct ConfigKey {
    pub name: &'static str,
    pub validate: fn(&str) -> Result<(), ConfigError>,
    pub set: fn(&mut RawAppConfig, &str) -> Result<(), ConfigError>,
    pub get: fn(&RawAppConfig) -> Option<String>,
    /// Render a stored value for display. Identity for most keys; masks
    /// `openai_api_key`.
    pub display: fn(&str) -> String,
}

// Manual `Debug` so tests can use `Result::unwrap_err`. We only surface the
// key name; the function pointers carry no useful diagnostic info.
impl std::fmt::Debug for ConfigKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigKey")
            .field("name", &self.name)
            .finish()
    }
}

/// The full set of recognised keys, in stable display order.
pub const KEYS: &[ConfigKey] = &[
    ConfigKey {
        name: "auth",
        validate: validate_auth,
        set: |c, v| {
            // Normalise to the canonical form before persisting.
            let mode: AuthMode = v
                .parse()
                .map_err(|m: String| ConfigError::ValidationError {
                    key: "auth".into(),
                    message: m,
                })?;
            c.auth = Some(mode.to_string());
            Ok(())
        },
        get: |c| c.auth.clone(),
        display: identity,
    },
    ConfigKey {
        name: "openai_api_key",
        validate: |_| Ok(()),
        set: |c, v| {
            c.openai_api_key = Some(v.to_string());
            Ok(())
        },
        get: |c| c.openai_api_key.clone(),
        display: mask_api_key,
    },
    ConfigKey {
        name: "openai_transcribe_model",
        validate: |v| {
            if v.trim().is_empty() {
                Err(ConfigError::ValidationError {
                    key: "openai_transcribe_model".into(),
                    message: "Model name cannot be empty".into(),
                })
            } else {
                Ok(())
            }
        },
        set: |c, v| {
            c.openai_transcribe_model = Some(v.to_string());
            Ok(())
        },
        get: |c| c.openai_transcribe_model.clone(),
        display: identity,
    },
    ConfigKey {
        name: "transcribe_prompt",
        validate: |v| {
            if v.len() > 4096 {
                Err(ConfigError::ValidationError {
                    key: "transcribe_prompt".into(),
                    message: "Prompt is too long (max 4096 chars)".into(),
                })
            } else {
                Ok(())
            }
        },
        set: |c, v| {
            c.transcribe_prompt = Some(v.to_string());
            Ok(())
        },
        get: |c| c.transcribe_prompt.clone(),
        display: identity,
    },
    ConfigKey {
        name: "transcribe_language",
        validate: |v| {
            let t = v.trim();
            if !t.is_empty() && (t.len() > 8 || !t.chars().all(|c| c.is_ascii_alphabetic())) {
                Err(ConfigError::ValidationError {
                    key: "transcribe_language".into(),
                    message: "Language must be an ISO 639-1/639-3 code (e.g. en, es, fr)".into(),
                })
            } else {
                Ok(())
            }
        },
        set: |c, v| {
            c.transcribe_language = Some(v.to_string());
            Ok(())
        },
        get: |c| c.transcribe_language.clone(),
        display: identity,
    },
    ConfigKey {
        name: "duration",
        validate: validate_duration,
        set: |c, v| {
            c.duration = Some(v.to_string());
            Ok(())
        },
        get: |c| c.duration.clone(),
        display: identity,
    },
    ConfigKey {
        name: "max_duration",
        validate: validate_duration,
        set: |c, v| {
            c.max_duration = Some(v.to_string());
            Ok(())
        },
        get: |c| c.max_duration.clone(),
        display: identity,
    },
    ConfigKey {
        name: "clipboard",
        validate: validate_bool,
        set: |c, v| {
            c.clipboard = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| c.clipboard.map(|b| b.to_string()),
        display: identity,
    },
    ConfigKey {
        name: "keystroke",
        validate: validate_bool,
        set: |c, v| {
            c.keystroke = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| c.keystroke.map(|b| b.to_string()),
        display: identity,
    },
    ConfigKey {
        name: "notify",
        validate: validate_bool,
        set: |c, v| {
            c.notify = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| c.notify.map(|b| b.to_string()),
        display: identity,
    },
    ConfigKey {
        name: "audio_cue",
        validate: validate_bool,
        set: |c, v| {
            c.audio_cue = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| c.audio_cue.map(|b| b.to_string()),
        display: identity,
    },
    ConfigKey {
        name: "linux.keystroke_tool",
        validate: |v| {
            let lower = v.to_lowercase();
            if VALID_KEYSTROKE_TOOLS.contains(&lower.as_str()) {
                Ok(())
            } else {
                Err(ConfigError::ValidationError {
                    key: "linux.keystroke_tool".into(),
                    message: format!(
                        "Invalid value '{}'. Valid options: {}",
                        v,
                        VALID_KEYSTROKE_TOOLS.join(", ")
                    ),
                })
            }
        },
        set: |c, v| {
            linux_section(c).keystroke_tool = Some(v.to_string());
            Ok(())
        },
        get: |c| c.linux.as_ref().and_then(|l| l.keystroke_tool.clone()),
        display: identity,
    },
    ConfigKey {
        name: "linux.indicator",
        validate: validate_bool,
        set: |c, v| {
            linux_section(c).indicator = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| {
            c.linux
                .as_ref()
                .and_then(|l| l.indicator)
                .map(|b| b.to_string())
        },
        display: identity,
    },
    ConfigKey {
        name: "linux.indicator_position",
        validate: |v| {
            if VALID_INDICATOR_POSITIONS.contains(&v) {
                Ok(())
            } else {
                Err(ConfigError::ValidationError {
                    key: "linux.indicator_position".into(),
                    message: format!(
                        "Invalid value '{}'. Valid: {}",
                        v,
                        VALID_INDICATOR_POSITIONS.join(", ")
                    ),
                })
            }
        },
        set: |c, v| {
            linux_section(c).indicator_position = Some(v.to_string());
            Ok(())
        },
        get: |c| c.linux.as_ref().and_then(|l| l.indicator_position.clone()),
        display: identity,
    },
    ConfigKey {
        name: "linux.paste",
        validate: validate_bool,
        set: |c, v| {
            linux_section(c).paste = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| {
            c.linux
                .as_ref()
                .and_then(|l| l.paste)
                .map(|b| b.to_string())
        },
        display: identity,
    },
    ConfigKey {
        name: "windows.indicator",
        validate: validate_bool,
        set: |c, v| {
            windows_section(c).indicator = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| {
            c.windows
                .as_ref()
                .and_then(|w| w.indicator)
                .map(|b| b.to_string())
        },
        display: identity,
    },
    ConfigKey {
        name: "windows.show_balloon",
        validate: validate_bool,
        set: |c, v| {
            windows_section(c).show_balloon = Some(parse_bool(v)?);
            Ok(())
        },
        get: |c| {
            c.windows
                .as_ref()
                .and_then(|w| w.show_balloon)
                .map(|b| b.to_string())
        },
        display: identity,
    },
];

/// Look up a key by name.
pub fn find(name: &str) -> Option<&'static ConfigKey> {
    KEYS.iter().find(|k| k.name == name)
}

/// Names of all recognised keys.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    KEYS.iter().map(|k| k.name)
}

// ---- shared validators / helpers ------------------------------------------

fn linux_section(c: &mut RawAppConfig) -> &mut RawLinuxConfig {
    c.linux.get_or_insert_with(RawLinuxConfig::default)
}

fn windows_section(c: &mut RawAppConfig) -> &mut RawWindowsConfig {
    c.windows.get_or_insert_with(RawWindowsConfig::default)
}

fn validate_auth(value: &str) -> Result<(), ConfigError> {
    value
        .parse::<AuthMode>()
        .map(|_| ())
        .map_err(|m| ConfigError::ValidationError {
            key: "auth".into(),
            message: format!("{m}. Valid options: {}", VALID_AUTH_MODES.join(", ")),
        })
}

fn validate_duration(value: &str) -> Result<(), ConfigError> {
    value
        .parse::<Duration>()
        .map(|_| ())
        .map_err(|e| ConfigError::ValidationError {
            key: "duration".into(),
            message: e.to_string(),
        })
}

fn validate_bool(value: &str) -> Result<(), ConfigError> {
    parse_bool(value).map(|_| ())
}

fn parse_bool(value: &str) -> Result<bool, ConfigError> {
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(ConfigError::ValidationError {
            key: String::new(),
            message: "Value must be 'true' or 'false'".to_string(),
        }),
    }
}

fn identity(s: &str) -> String {
    s.to_string()
}

/// Mask an API key for display (first 4 + last 4 chars).
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_unique_and_complete() {
        let names: Vec<_> = KEYS.iter().map(|k| k.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate key in registry");
        // Sanity: stable set the rest of the codebase relies on.
        assert!(find("auth").is_some());
        assert!(find("linux.indicator_position").is_some());
        assert!(find("windows.show_balloon").is_some());
        assert!(find("bogus").is_none());
    }

    #[test]
    fn validate_duration_accepts_valid() {
        let entry = find("duration").unwrap();
        assert!((entry.validate)("30s").is_ok());
        assert!((entry.validate)("1m").is_ok());
        assert!((entry.validate)("2m30s").is_ok());
        assert!((entry.validate)("garbage").is_err());
    }

    #[test]
    fn validate_auth_rejects_invalid() {
        let entry = find("auth").unwrap();
        assert!((entry.validate)("oauth").is_ok());
        assert!((entry.validate)("api_key").is_ok());
        assert!((entry.validate)("cookies").is_err());
    }

    #[test]
    fn validate_keystroke_tool_accepts_all() {
        let entry = find("linux.keystroke_tool").unwrap();
        for tool in VALID_KEYSTROKE_TOOLS {
            assert!((entry.validate)(tool).is_ok(), "rejected {tool}");
        }
        assert!((entry.validate)("nonsense").is_err());
    }

    #[test]
    fn set_and_get_round_trip_top_level() {
        let entry = find("clipboard").unwrap();
        let mut cfg = RawAppConfig::empty();
        (entry.set)(&mut cfg, "true").unwrap();
        assert_eq!((entry.get)(&cfg).as_deref(), Some("true"));
    }

    #[test]
    fn set_and_get_round_trip_linux_nested() {
        let entry = find("linux.indicator_position").unwrap();
        let mut cfg = RawAppConfig::empty();
        (entry.set)(&mut cfg, "bottom-left").unwrap();
        assert_eq!((entry.get)(&cfg).as_deref(), Some("bottom-left"));
    }

    #[test]
    fn set_invalid_indicator_position_fails() {
        let entry = find("linux.indicator_position").unwrap();
        assert!((entry.validate)("nowhere").is_err());
    }

    #[test]
    fn mask_api_key_long() {
        assert_eq!(mask_api_key("abcdefghijklmnop"), "abcd...mnop");
    }

    #[test]
    fn mask_api_key_short() {
        assert_eq!(mask_api_key("short"), "*****");
    }
}
