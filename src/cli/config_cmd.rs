//! Config command handler

use std::collections::BTreeMap;

use crate::application::ports::ConfigStore;
use crate::domain::config::{AuthMode, LinuxConfig, WindowsConfig};
use crate::domain::error::ConfigError;

use super::args::{
    is_valid_config_key, ConfigAction, VALID_AUTH_MODES, VALID_CONFIG_KEYS, VALID_KEYSTROKE_TOOLS,
};
use super::presenter::Presenter;

/// Handle config subcommand
pub async fn handle_config_command<S: ConfigStore>(
    action: ConfigAction,
    store: &S,
    presenter: &Presenter,
) -> Result<(), ConfigError> {
    match action {
        ConfigAction::Init => handle_init(store, presenter).await,
        ConfigAction::Set { key, value } => handle_set(store, presenter, &key, &value).await,
        ConfigAction::Get { key } => handle_get(store, presenter, &key).await,
        ConfigAction::List => handle_list(store, presenter).await,
        ConfigAction::Path => handle_path(store, presenter),
    }
}

async fn handle_init<S: ConfigStore>(store: &S, presenter: &Presenter) -> Result<(), ConfigError> {
    store.init().await?;
    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "init",
            "path": store.path().to_string_lossy(),
        }));
    } else {
        presenter.success(&format!(
            "Config file created at: {}",
            store.path().display()
        ));
    }
    Ok(())
}

async fn handle_set<S: ConfigStore>(
    store: &S,
    presenter: &Presenter,
    key: &str,
    value: &str,
) -> Result<(), ConfigError> {
    // Validate key
    if !is_valid_config_key(key) {
        return Err(ConfigError::ValidationError {
            key: key.to_string(),
            message: format!("Unknown key. Valid keys: {}", VALID_CONFIG_KEYS.join(", ")),
        });
    }

    // Validate value based on key type
    validate_config_value(key, value)?;

    // Load existing config
    let mut config = store.load().await?;

    // Update the appropriate field
    match key {
        "auth" => {
            // Normalize to canonical form before persisting.
            let mode: AuthMode =
                value
                    .parse()
                    .map_err(|m: String| ConfigError::ValidationError {
                        key: key.to_string(),
                        message: m,
                    })?;
            config.auth = Some(mode.to_string());
        }
        "openai_api_key" => config.openai_api_key = Some(value.to_string()),
        "openai_transcribe_model" => config.openai_transcribe_model = Some(value.to_string()),
        "duration" => config.duration = Some(value.to_string()),
        "max_duration" => config.max_duration = Some(value.to_string()),
        "clipboard" => {
            config.clipboard =
                Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: "Value must be 'true' or 'false'".to_string(),
                })?)
        }
        "keystroke" => {
            config.keystroke =
                Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: "Value must be 'true' or 'false'".to_string(),
                })?)
        }
        "notify" => {
            config.notify = Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                key: key.to_string(),
                message: "Value must be 'true' or 'false'".to_string(),
            })?)
        }
        "audio_cue" => {
            config.audio_cue =
                Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: "Value must be 'true' or 'false'".to_string(),
                })?)
        }
        "linux.keystroke_tool" => {
            if config.linux.is_none() {
                config.linux = Some(LinuxConfig::default());
            }
            if let Some(ref mut linux) = config.linux {
                linux.keystroke_tool = Some(value.to_string());
            }
        }
        "linux.indicator" => {
            if config.linux.is_none() {
                config.linux = Some(LinuxConfig::default());
            }
            if let Some(ref mut linux) = config.linux {
                linux.indicator =
                    Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                        key: key.to_string(),
                        message: "Value must be 'true' or 'false'".to_string(),
                    })?);
            }
        }
        "linux.indicator_position" => {
            if config.linux.is_none() {
                config.linux = Some(LinuxConfig::default());
            }
            if let Some(ref mut linux) = config.linux {
                linux.indicator_position = Some(value.to_string());
            }
        }
        "linux.paste" => {
            if config.linux.is_none() {
                config.linux = Some(LinuxConfig::default());
            }
            if let Some(ref mut linux) = config.linux {
                linux.paste =
                    Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                        key: key.to_string(),
                        message: "Value must be 'true' or 'false'".to_string(),
                    })?);
            }
        }
        "windows.indicator" => {
            if config.windows.is_none() {
                config.windows = Some(WindowsConfig::default());
            }
            if let Some(ref mut windows) = config.windows {
                windows.indicator =
                    Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                        key: key.to_string(),
                        message: "Value must be 'true' or 'false'".to_string(),
                    })?);
            }
        }
        "windows.show_balloon" => {
            if config.windows.is_none() {
                config.windows = Some(WindowsConfig::default());
            }
            if let Some(ref mut windows) = config.windows {
                windows.show_balloon =
                    Some(parse_bool(value).map_err(|_| ConfigError::ValidationError {
                        key: key.to_string(),
                        message: "Value must be 'true' or 'false'".to_string(),
                    })?);
            }
        }
        _ => unreachable!(), // Already validated
    }

    // Save config
    store.save(&config).await?;
    let display_value = if key == "openai_api_key" {
        mask_api_key(value)
    } else {
        value.to_string()
    };
    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "set",
            "key": key,
            "value": display_value,
        }));
    } else {
        presenter.success(&format!("{} = {}", key, display_value));
    }

    Ok(())
}

async fn handle_get<S: ConfigStore>(
    store: &S,
    presenter: &Presenter,
    key: &str,
) -> Result<(), ConfigError> {
    // Validate key
    if !is_valid_config_key(key) {
        return Err(ConfigError::ValidationError {
            key: key.to_string(),
            message: format!("Unknown key. Valid keys: {}", VALID_CONFIG_KEYS.join(", ")),
        });
    }

    let config = store.load().await?;

    let value = match key {
        "auth" => config.auth,
        "openai_api_key" => config.openai_api_key.map(|s| mask_api_key(&s)),
        "openai_transcribe_model" => config.openai_transcribe_model,
        "duration" => config.duration,
        "max_duration" => config.max_duration,
        "clipboard" => config.clipboard.map(|b| b.to_string()),
        "keystroke" => config.keystroke.map(|b| b.to_string()),
        "notify" => config.notify.map(|b| b.to_string()),
        "audio_cue" => config.audio_cue.map(|b| b.to_string()),
        "linux.keystroke_tool" => config.linux.as_ref().and_then(|l| l.keystroke_tool.clone()),
        "linux.indicator" => config
            .linux
            .as_ref()
            .and_then(|l| l.indicator)
            .map(|b| b.to_string()),
        "linux.indicator_position" => config
            .linux
            .as_ref()
            .and_then(|l| l.indicator_position.clone()),
        "linux.paste" => config
            .linux
            .as_ref()
            .and_then(|l| l.paste)
            .map(|b| b.to_string()),
        "windows.indicator" => config
            .windows
            .as_ref()
            .and_then(|w| w.indicator)
            .map(|b| b.to_string()),
        "windows.show_balloon" => config
            .windows
            .as_ref()
            .and_then(|w| w.show_balloon)
            .map(|b| b.to_string()),
        _ => unreachable!(),
    };

    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "get",
            "key": key,
            "value": value,
        }));
    } else {
        match value {
            Some(v) => presenter.output(&v),
            None => presenter.output("(not set)"),
        }
    }

    Ok(())
}

async fn handle_list<S: ConfigStore>(store: &S, presenter: &Presenter) -> Result<(), ConfigError> {
    let config = store.load().await?;

    let values = BTreeMap::from([
        ("auth".to_string(), config.auth.clone()),
        (
            "openai_api_key".to_string(),
            config.openai_api_key.as_ref().map(|s| mask_api_key(s)),
        ),
        (
            "openai_transcribe_model".to_string(),
            config.openai_transcribe_model.clone(),
        ),
        ("duration".to_string(), config.duration.clone()),
        ("max_duration".to_string(), config.max_duration.clone()),
        (
            "clipboard".to_string(),
            config.clipboard.map(|b| b.to_string()),
        ),
        (
            "keystroke".to_string(),
            config.keystroke.map(|b| b.to_string()),
        ),
        ("notify".to_string(), config.notify.map(|b| b.to_string())),
        (
            "audio_cue".to_string(),
            config.audio_cue.map(|b| b.to_string()),
        ),
        (
            "linux.keystroke_tool".to_string(),
            config.linux.as_ref().and_then(|l| l.keystroke_tool.clone()),
        ),
        (
            "linux.indicator".to_string(),
            config
                .linux
                .as_ref()
                .and_then(|l| l.indicator)
                .map(|b| b.to_string()),
        ),
        (
            "linux.indicator_position".to_string(),
            config
                .linux
                .as_ref()
                .and_then(|l| l.indicator_position.clone()),
        ),
        (
            "linux.paste".to_string(),
            config
                .linux
                .as_ref()
                .and_then(|l| l.paste)
                .map(|b| b.to_string()),
        ),
        (
            "windows.indicator".to_string(),
            config
                .windows
                .as_ref()
                .and_then(|w| w.indicator)
                .map(|b| b.to_string()),
        ),
        (
            "windows.show_balloon".to_string(),
            config
                .windows
                .as_ref()
                .and_then(|w| w.show_balloon)
                .map(|b| b.to_string()),
        ),
    ]);

    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "list",
            "values": values,
        }));
    } else {
        for (key, value) in values {
            presenter.key_value(&key, value.as_deref().unwrap_or("(not set)"));
        }
    }

    Ok(())
}

fn handle_path<S: ConfigStore>(store: &S, presenter: &Presenter) -> Result<(), ConfigError> {
    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "path",
            "path": store.path().to_string_lossy(),
        }));
    } else {
        presenter.output(&store.path().to_string_lossy());
    }
    Ok(())
}

/// Validate a config value based on key type
fn validate_config_value(key: &str, value: &str) -> Result<(), ConfigError> {
    match key {
        "duration" | "max_duration" => {
            value
                .parse::<crate::domain::recording::Duration>()
                .map_err(|e| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: e.to_string(),
                })?;
        }
        "auth" => {
            value
                .parse::<AuthMode>()
                .map_err(|m| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: format!("{m}. Valid options: {}", VALID_AUTH_MODES.join(", ")),
                })?;
        }
        "openai_transcribe_model" => {
            if value.trim().is_empty() {
                return Err(ConfigError::ValidationError {
                    key: key.to_string(),
                    message: "Model name cannot be empty".to_string(),
                });
            }
        }
        "clipboard"
        | "keystroke"
        | "notify"
        | "audio_cue"
        | "linux.paste"
        | "windows.indicator"
        | "windows.show_balloon" => {
            parse_bool(value).map_err(|_| ConfigError::ValidationError {
                key: key.to_string(),
                message: "Value must be 'true' or 'false'".to_string(),
            })?;
        }
        "linux.keystroke_tool" => {
            let lower = value.to_lowercase();
            if !VALID_KEYSTROKE_TOOLS.contains(&lower.as_str()) {
                return Err(ConfigError::ValidationError {
                    key: key.to_string(),
                    message: format!(
                        "Invalid value '{}'. Valid options: {}",
                        value,
                        VALID_KEYSTROKE_TOOLS.join(", ")
                    ),
                });
            }
        }
        "linux.indicator" => {
            parse_bool(value).map_err(|_| ConfigError::ValidationError {
                key: key.to_string(),
                message: "Value must be 'true' or 'false'".to_string(),
            })?;
        }
        "linux.indicator_position" => {
            let valid = [
                "top-right",
                "top-left",
                "top-center",
                "bottom-center",
                "bottom-right",
                "bottom-left",
            ];
            if !valid.contains(&value) {
                return Err(ConfigError::ValidationError {
                    key: key.to_string(),
                    message: format!("Invalid value '{}'. Valid: {}", value, valid.join(", ")),
                });
            }
        }
        _ => {} // openai_api_key accepts any string
    }
    Ok(())
}

/// Parse a boolean value
fn parse_bool(value: &str) -> Result<bool, ()> {
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(()),
    }
}

/// Mask API key for display (show first 4 and last 4 chars)
pub(crate) fn mask_api_key(key: &str) -> String {
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
    fn parse_bool_values() {
        assert_eq!(parse_bool("true"), Ok(true));
        assert_eq!(parse_bool("false"), Ok(false));
        assert_eq!(parse_bool("yes"), Ok(true));
        assert_eq!(parse_bool("no"), Ok(false));
        assert_eq!(parse_bool("1"), Ok(true));
        assert_eq!(parse_bool("0"), Ok(false));
        assert!(parse_bool("invalid").is_err());
    }

    #[test]
    fn mask_api_key_long() {
        let masked = mask_api_key("abcdefghijklmnop");
        assert_eq!(masked, "abcd...mnop");
    }

    #[test]
    fn mask_api_key_short() {
        let masked = mask_api_key("short");
        assert_eq!(masked, "*****");
    }

    #[test]
    fn validate_duration_valid() {
        assert!(validate_config_value("duration", "30s").is_ok());
        assert!(validate_config_value("duration", "1m").is_ok());
        assert!(validate_config_value("duration", "2m30s").is_ok());
    }

    #[test]
    fn validate_duration_invalid() {
        assert!(validate_config_value("duration", "invalid").is_err());
    }

    #[test]
    fn validate_auth_valid() {
        assert!(validate_config_value("auth", "oauth").is_ok());
        assert!(validate_config_value("auth", "api_key").is_ok());
        assert!(validate_config_value("auth", "API-Key").is_ok());
    }

    #[test]
    fn validate_auth_invalid() {
        assert!(validate_config_value("auth", "cookies").is_err());
    }

    #[test]
    fn validate_openai_transcribe_model_rejects_empty() {
        assert!(validate_config_value("openai_transcribe_model", "  ").is_err());
        assert!(validate_config_value("openai_transcribe_model", "whisper-1").is_ok());
    }

    #[test]
    fn validate_keystroke_tool_accepts_all_tools_on_all_platforms() {
        assert!(validate_config_value("linux.keystroke_tool", "enigo").is_ok());
        assert!(validate_config_value("linux.keystroke_tool", "auto").is_ok());
        assert!(validate_config_value("linux.keystroke_tool", "ydotool").is_ok());
        assert!(validate_config_value("linux.keystroke_tool", "xdotool").is_ok());
        assert!(validate_config_value("linux.keystroke_tool", "wtype").is_ok());
    }

    #[test]
    fn validate_keystroke_tool_invalid() {
        assert!(validate_config_value("linux.keystroke_tool", "invalid").is_err());
    }
}
