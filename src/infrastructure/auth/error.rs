//! Errors raised by the OAuth subsystem

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Network error talking to OpenAI: {0}")]
    Network(String),

    #[error("OpenAI auth server returned an error: {0}")]
    Server(String),

    #[error("Refresh token rejected (expired or revoked). Run `smart-scribe login` again.")]
    InvalidGrant,

    #[error("Login timed out waiting for browser callback")]
    Timeout,

    #[error("Failed to open browser. Visit the URL printed above to continue.")]
    BrowserOpen,

    #[error("Failed to parse JWT: {0}")]
    JwtParse(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Could not locate Codex auth file at ~/.codex/auth.json")]
    CodexAuthMissing,

    #[error("Codex auth file is malformed: {0}")]
    CodexAuthMalformed(String),

    #[error("Could not resolve config directory for token storage")]
    ConfigDirMissing,

    #[error("Local callback server failed: {0}")]
    Callback(String),

    #[error("OAuth state mismatch — possible CSRF attempt")]
    StateMismatch,

    #[error("Authorization was denied or cancelled in the browser")]
    AuthDenied,
}

impl From<std::io::Error> for OAuthError {
    fn from(err: std::io::Error) -> Self {
        OAuthError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for OAuthError {
    fn from(err: reqwest::Error) -> Self {
        OAuthError::Network(err.to_string())
    }
}
