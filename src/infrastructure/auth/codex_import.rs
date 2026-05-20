//! Import OAuth credentials from an existing Codex CLI installation.
//!
//! We read `~/.codex/auth.json` (or `$CODEX_HOME/auth.json`), pull the refresh
//! token out, and call the OAuth refresh endpoint once. This gives us our own
//! fresh access+refresh token pair without ever touching Codex's file.

use std::path::PathBuf;

use serde::Deserialize;

use super::error::OAuthError;
use super::oauth_client::refresh;
use super::oauth_token::OAuthToken;

/// Try `$CODEX_HOME/auth.json` first, fall back to `~/.codex/auth.json`.
pub fn codex_auth_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        let p = PathBuf::from(home).join("auth.json");
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".codex").join("auth.json");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Read Codex's auth file, extract the refresh token, refresh once and return
/// the fresh credentials. The Codex file is never written.
pub async fn import_from_codex() -> Result<OAuthToken, OAuthError> {
    let path = codex_auth_path().ok_or(OAuthError::CodexAuthMissing)?;
    let bytes = std::fs::read(&path)
        .map_err(|e| OAuthError::CodexAuthMalformed(format!("read {}: {e}", path.display())))?;
    let parsed: CodexAuthFile = serde_json::from_slice(&bytes)
        .map_err(|e| OAuthError::CodexAuthMalformed(e.to_string()))?;
    let refresh_token = parsed
        .tokens
        .and_then(|t| t.refresh_token)
        .ok_or_else(|| OAuthError::CodexAuthMalformed("missing tokens.refresh_token".into()))?;

    refresh(&refresh_token).await
}

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexTokens>,
}

#[derive(Debug, Deserialize)]
struct CodexTokens {
    refresh_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn codex_auth_path_returns_some_when_codex_home_is_set() {
        let dir = tempfile::tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(&auth, "{}").unwrap();
        std::env::set_var("CODEX_HOME", dir.path());
        let result = codex_auth_path();
        std::env::remove_var("CODEX_HOME");
        assert_eq!(result.as_deref(), Some(auth.as_path()));
    }

    #[test]
    fn parses_minimal_codex_auth_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"{{"auth_mode":"chatgpt","tokens":{{"refresh_token":"rt_abc"}}}}"#
        )
        .unwrap();
        let bytes = std::fs::read(file.path()).unwrap();
        let parsed: CodexAuthFile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            parsed.tokens.unwrap().refresh_token.as_deref(),
            Some("rt_abc")
        );
    }
}
