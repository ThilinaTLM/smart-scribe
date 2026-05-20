//! `login`, `logout`, and `auth status` subcommands.

use std::env;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::config::{AppConfig, AuthMode};
use crate::infrastructure::auth::{import_from_codex, run_pkce_login, OAuthStore};

use super::args::OutputFormatArg;
use super::presenter::Presenter;

const EXIT_OK: u8 = 0;
const EXIT_FAIL: u8 = 1;

/// Run the OAuth login flow and persist the token.
pub async fn run_login(from_codex: bool, output: OutputFormatArg) -> ExitCode {
    let presenter = Presenter::new(output);

    let store = match OAuthStore::new() {
        Ok(s) => s,
        Err(e) => {
            presenter.error(&format!("OAuth store init failed: {e}"));
            return ExitCode::from(EXIT_FAIL);
        }
    };

    let token_result = if from_codex {
        import_from_codex().await
    } else {
        run_pkce_login().await
    };

    let token = match token_result {
        Ok(t) => t,
        Err(e) => {
            presenter.error(&format!("Login failed: {e}"));
            return ExitCode::from(EXIT_FAIL);
        }
    };

    if let Err(e) = store.save(&token) {
        presenter.error(&format!("Could not save token: {e}"));
        return ExitCode::from(EXIT_FAIL);
    }

    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "login",
            "account_id": token.account_id,
            "expires_at_unix": token.expires_at_unix,
            "from_codex": from_codex,
        }));
    } else {
        presenter.success(&format!(
            "Logged in. Token stored at {}.",
            store.path().display()
        ));
        if from_codex {
            presenter.info(
                "Imported from Codex. Your existing `codex` install must be re-logged in \
                 because the refresh token has been rotated.",
            );
        }
    }

    ExitCode::from(EXIT_OK)
}

/// Delete the persisted OAuth token (idempotent).
pub async fn run_logout(output: OutputFormatArg) -> ExitCode {
    let presenter = Presenter::new(output);

    let store = match OAuthStore::new() {
        Ok(s) => s,
        Err(e) => {
            presenter.error(&format!("OAuth store init failed: {e}"));
            return ExitCode::from(EXIT_FAIL);
        }
    };

    if let Err(e) = store.delete() {
        presenter.error(&format!("Could not delete token: {e}"));
        return ExitCode::from(EXIT_FAIL);
    }

    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "logout",
        }));
    } else {
        presenter.success("Logged out. OAuth token removed.");
    }

    ExitCode::from(EXIT_OK)
}

/// Print the current authentication status.
pub async fn run_auth_status(config: &AppConfig, output: OutputFormatArg) -> ExitCode {
    let presenter = Presenter::new(output);

    let mode = config.auth_or_default();

    let store = OAuthStore::new().ok();
    let token = store.as_ref().and_then(|s| s.load().ok().flatten());
    let token_present = token.is_some();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let expires_in_secs = token.as_ref().map(|t| t.expires_at_unix - now);
    let account_id = token.as_ref().map(|t| t.account_id.clone());
    let openai_env = env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some();
    let openai_in_config = config
        .openai_api_key
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    if presenter.is_json() {
        presenter.output_json(&serde_json::json!({
            "ok": true,
            "action": "auth_status",
            "auth": mode.to_string(),
            "oauth_token_present": token_present,
            "oauth_expires_in_secs": expires_in_secs,
            "oauth_account_id": account_id,
            "openai_api_key_env": openai_env,
            "openai_api_key_config": openai_in_config,
            "openai_transcribe_model": config.openai_transcribe_model_or_default(),
        }));
        return ExitCode::from(EXIT_OK);
    }

    presenter.key_value("auth", mode.as_str());
    match mode {
        AuthMode::Oauth => {
            if let Some(tok) = token.as_ref() {
                presenter.key_value("oauth_token", "present");
                presenter.key_value("oauth_account_id", &tok.account_id);
                let expires_in = tok.expires_at_unix - now;
                if expires_in > 0 {
                    presenter.key_value("oauth_expires_in", &format_seconds(expires_in));
                } else {
                    presenter.key_value(
                        "oauth_expires_in",
                        &format!(
                            "expired {} ago (will refresh on next use)",
                            format_seconds(-expires_in)
                        ),
                    );
                }
            } else {
                presenter.key_value("oauth_token", "missing (run `smart-scribe login`)");
            }
        }
        AuthMode::ApiKey => {
            presenter.key_value(
                "openai_api_key",
                if openai_env {
                    "present (from OPENAI_API_KEY env)"
                } else if openai_in_config {
                    "present (from config file)"
                } else {
                    "missing"
                },
            );
            presenter.key_value(
                "openai_transcribe_model",
                config.openai_transcribe_model_or_default(),
            );
        }
    }

    ExitCode::from(EXIT_OK)
}

/// Build a one-line auth-status banner for the startup logs (one-shot and daemon).
///
/// For OAuth this peeks at the cached token to surface its remaining lifetime
/// without forcing a refresh; for the API-key path it surfaces the configured
/// model. Output goes to stderr alongside the other startup lines
/// (`Keystroke: using ...`, `Paste: using ...`).
pub fn describe_auth(config: &AppConfig) -> String {
    let model = config.openai_transcribe_model_or_default();
    match config.auth_or_default() {
        AuthMode::Oauth => {
            let store = match OAuthStore::new() {
                Ok(s) => s,
                Err(_) => {
                    return format!(
                        "Auth: ChatGPT subscription (model: {model}, token store unavailable)"
                    )
                }
            };
            match store.load().ok().flatten() {
                Some(tok) => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let expires_in = tok.expires_at_unix - now;
                    if expires_in > 0 {
                        format!(
                            "Auth: ChatGPT subscription (model: {}, token valid for {})",
                            model,
                            format_seconds(expires_in)
                        )
                    } else {
                        format!(
                            "Auth: ChatGPT subscription (model: {}, token expired {} ago, will refresh)",
                            model,
                            format_seconds(-expires_in)
                        )
                    }
                }
                None => format!(
                    "Auth: ChatGPT subscription (model: {model}, no token cached \u{2014} run `smart-scribe login`)"
                ),
            }
        }
        AuthMode::ApiKey => {
            format!("Auth: OpenAI API key (model: {model})")
        }
    }
}

fn format_seconds(secs: i64) -> String {
    let abs = secs.unsigned_abs();
    if abs >= 86_400 {
        format!("{}d {}h", abs / 86_400, (abs % 86_400) / 3_600)
    } else if abs >= 3_600 {
        format!("{}h {}m", abs / 3_600, (abs % 3_600) / 60)
    } else if abs >= 60 {
        format!("{}m {}s", abs / 60, abs % 60)
    } else {
        format!("{abs}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::config::AuthMode;

    #[test]
    fn describe_auth_api_key_mentions_model() {
        let mut cfg = AppConfig::empty();
        cfg.auth = Some(AuthMode::ApiKey.to_string());
        cfg.openai_transcribe_model = Some("whisper-1".into());
        let line = describe_auth(&cfg);
        assert!(line.starts_with("Auth: OpenAI API key"));
        assert!(line.contains("whisper-1"), "got: {line}");
    }

    #[test]
    fn describe_auth_oauth_default_mentions_subscription_and_model() {
        // No token will typically be present in the test env. The banner
        // should still mention the ChatGPT subscription path and selected model.
        let mut cfg = AppConfig::empty();
        cfg.openai_transcribe_model = Some("gpt-4o-transcribe".into());
        let line = describe_auth(&cfg);
        assert!(
            line.starts_with("Auth: ChatGPT subscription"),
            "got: {line}"
        );
        assert!(line.contains("gpt-4o-transcribe"), "got: {line}");
    }

    #[test]
    fn format_seconds_units() {
        assert_eq!(format_seconds(5), "5s");
        assert_eq!(format_seconds(125), "2m 5s");
        assert_eq!(format_seconds(3_600 + 130), "1h 2m");
        assert_eq!(format_seconds(2 * 86_400 + 3_600 * 5), "2d 5h");
    }
}
