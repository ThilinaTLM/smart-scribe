//! `config` subcommand handlers.
//!
//! All per-key behaviour lives in [`super::config_schema`]; this module just
//! orchestrates the load → validate → mutate → save lifecycle.

use std::collections::BTreeMap;

use crate::application::ports::ConfigStore;
use crate::domain::error::ConfigError;

use super::args::ConfigAction;
use super::config_schema;
use super::presenter::Presenter;

/// Handle a `config <action>` invocation.
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
    let entry = lookup(key)?;
    (entry.validate)(value)?;

    let mut config = store.load().await?;
    (entry.set)(&mut config, value)?;
    store.save(&config).await?;

    let display_value = (entry.display)(value);

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
    let entry = lookup(key)?;
    let config = store.load().await?;
    let value = (entry.get)(&config).map(|v| (entry.display)(&v));

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
    let mut values: BTreeMap<String, Option<String>> = BTreeMap::new();
    for entry in config_schema::KEYS {
        values.insert(
            entry.name.to_string(),
            (entry.get)(&config).map(|v| (entry.display)(&v)),
        );
    }

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

fn lookup(key: &str) -> Result<&'static config_schema::ConfigKey, ConfigError> {
    config_schema::find(key).ok_or_else(|| ConfigError::ValidationError {
        key: key.to_string(),
        message: format!(
            "Unknown key. Valid keys: {}",
            config_schema::all_names().collect::<Vec<_>>().join(", ")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::RawAppConfig;

    fn validate(key: &str, value: &str) -> Result<(), ConfigError> {
        let entry = lookup(key)?;
        (entry.validate)(value)
    }

    #[test]
    fn validate_duration_valid() {
        assert!(validate("duration", "30s").is_ok());
        assert!(validate("duration", "1m").is_ok());
        assert!(validate("duration", "2m30s").is_ok());
    }

    #[test]
    fn validate_duration_invalid() {
        assert!(validate("duration", "invalid").is_err());
    }

    #[test]
    fn validate_auth_valid() {
        assert!(validate("auth", "oauth").is_ok());
        assert!(validate("auth", "api_key").is_ok());
        assert!(validate("auth", "API-Key").is_ok());
    }

    #[test]
    fn validate_auth_invalid() {
        assert!(validate("auth", "cookies").is_err());
    }

    #[test]
    fn validate_openai_transcribe_model_rejects_empty() {
        assert!(validate("openai_transcribe_model", "  ").is_err());
        assert!(validate("openai_transcribe_model", "whisper-1").is_ok());
    }

    #[test]
    fn validate_keystroke_tool_accepts_all_tools() {
        assert!(validate("linux.keystroke_tool", "enigo").is_ok());
        assert!(validate("linux.keystroke_tool", "auto").is_ok());
        assert!(validate("linux.keystroke_tool", "ydotool").is_ok());
        assert!(validate("linux.keystroke_tool", "xdotool").is_ok());
        assert!(validate("linux.keystroke_tool", "wtype").is_ok());
    }

    #[test]
    fn validate_keystroke_tool_invalid() {
        assert!(validate("linux.keystroke_tool", "invalid").is_err());
    }

    #[test]
    fn unknown_key_returns_validation_error() {
        let err = lookup("nope").unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError { .. }));
    }

    #[test]
    fn set_roundtrip_through_schema() {
        let entry = lookup("clipboard").unwrap();
        let mut cfg = RawAppConfig::empty();
        (entry.set)(&mut cfg, "true").unwrap();
        assert_eq!(cfg.clipboard, Some(true));
    }
}
