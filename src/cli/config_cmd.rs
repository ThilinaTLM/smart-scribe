//! Config command handler

use crate::application::ports::ConfigStore;
use crate::domain::error::ConfigError;
use crate::domain::transcription::DomainId;

use super::args::{is_valid_config_key, ConfigAction, VALID_CONFIG_KEYS};
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
    presenter.success(&format!(
        "Config file created at: {}",
        store.path().display()
    ));
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
        "api_key" => config.api_key = Some(value.to_string()),
        "duration" => config.duration = Some(value.to_string()),
        "max_duration" => config.max_duration = Some(value.to_string()),
        "domain" => config.domain = Some(value.to_string()),
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
        _ => unreachable!(), // Already validated
    }

    // Save config
    store.save(&config).await?;
    presenter.success(&format!("{} = {}", key, value));

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
        "api_key" => config.api_key.map(|s| mask_api_key(&s)),
        "duration" => config.duration,
        "max_duration" => config.max_duration,
        "domain" => config.domain,
        "clipboard" => config.clipboard.map(|b| b.to_string()),
        "keystroke" => config.keystroke.map(|b| b.to_string()),
        "notify" => config.notify.map(|b| b.to_string()),
        _ => unreachable!(),
    };

    match value {
        Some(v) => presenter.output(&v),
        None => presenter.output("(not set)"),
    }

    Ok(())
}

async fn handle_list<S: ConfigStore>(store: &S, presenter: &Presenter) -> Result<(), ConfigError> {
    let config = store.load().await?;

    presenter.key_value(
        "api_key",
        &config
            .api_key
            .map(|s| mask_api_key(&s))
            .unwrap_or_else(|| "(not set)".to_string()),
    );
    presenter.key_value(
        "duration",
        config.duration.as_deref().unwrap_or("(not set)"),
    );
    presenter.key_value(
        "max_duration",
        config.max_duration.as_deref().unwrap_or("(not set)"),
    );
    presenter.key_value("domain", config.domain.as_deref().unwrap_or("(not set)"));
    presenter.key_value(
        "clipboard",
        &config
            .clipboard
            .map(|b| b.to_string())
            .unwrap_or_else(|| "(not set)".to_string()),
    );
    presenter.key_value(
        "keystroke",
        &config
            .keystroke
            .map(|b| b.to_string())
            .unwrap_or_else(|| "(not set)".to_string()),
    );
    presenter.key_value(
        "notify",
        &config
            .notify
            .map(|b| b.to_string())
            .unwrap_or_else(|| "(not set)".to_string()),
    );

    Ok(())
}

fn handle_path<S: ConfigStore>(store: &S, presenter: &Presenter) -> Result<(), ConfigError> {
    presenter.output(&store.path().to_string_lossy());
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
        "domain" => {
            value
                .parse::<DomainId>()
                .map_err(|e| ConfigError::ValidationError {
                    key: key.to_string(),
                    message: e.to_string(),
                })?;
        }
        "clipboard" | "keystroke" | "notify" => {
            parse_bool(value).map_err(|_| ConfigError::ValidationError {
                key: key.to_string(),
                message: "Value must be 'true' or 'false'".to_string(),
            })?;
        }
        _ => {} // api_key accepts any string
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
fn mask_api_key(key: &str) -> String {
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
    fn validate_domain_valid() {
        assert!(validate_config_value("domain", "dev").is_ok());
        assert!(validate_config_value("domain", "general").is_ok());
    }

    #[test]
    fn validate_domain_invalid() {
        assert!(validate_config_value("domain", "invalid").is_err());
    }
}
