//! PKCE OAuth login + refresh against `auth.openai.com`.
//!
//! We piggyback on the public Codex CLI OAuth client (`app_EMoamE...`). This is
//! the only registered client_id that allows non-OpenAI tools to redeem ChatGPT
//! subscription tokens today; arbitrary client ids are rejected with
//! `invalid_client`. The redirect URI is fixed to `http://localhost:1455/auth/callback`.

use std::time::Duration as StdDuration;

use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::timeout;

use super::error::OAuthError;
use super::oauth_token::{decode_jwt_exp_and_account_id, OAuthToken};

pub const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
pub const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
pub const SCOPES: &str = "openid profile email offline_access";
pub const CALLBACK_PORT: u16 = 1455;
pub const ORIGINATOR: &str = "smart-scribe";

/// Maximum time we wait for the user to complete the browser flow.
const LOGIN_TIMEOUT: StdDuration = StdDuration::from_secs(300);

/// Internal callback container.
struct Callback {
    code: String,
    state: String,
}

/// Build the authorize URL for a given PKCE challenge + state.
pub(crate) fn build_authorize_url(code_challenge: &str, state: &str) -> String {
    let qs = [
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("scope", SCOPES),
        ("response_type", "code"),
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("codex_cli_simplified_flow", "true"),
        ("originator", ORIGINATOR),
    ];
    let encoded: String = qs
        .iter()
        .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTH_ENDPOINT}?{encoded}")
}

/// Run the full PKCE browser flow. Opens the browser, listens on localhost:1455,
/// and returns the freshly-issued [`OAuthToken`].
pub async fn run_pkce_login() -> Result<OAuthToken, OAuthError> {
    let verifier = generate_verifier();
    let challenge = challenge_from_verifier(&verifier);
    let state = generate_state();
    let url = build_authorize_url(&challenge, &state);

    // Bind first so we don't open the browser before we're ready to receive.
    let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT))
        .await
        .map_err(|e| {
            OAuthError::Callback(format!(
                "could not bind localhost:{CALLBACK_PORT} (is `codex login` running?): {e}"
            ))
        })?;

    eprintln!("Opening browser for ChatGPT login...");
    eprintln!("If the browser does not open, visit:\n\n  {url}\n");
    if let Err(e) = open::that(&url) {
        eprintln!("(could not auto-open browser: {e})");
    }

    let (tx, rx) = oneshot::channel::<Result<Callback, OAuthError>>();
    let server_state = state.clone();
    tokio::spawn(async move {
        let result = await_callback(listener, &server_state).await;
        let _ = tx.send(result);
    });

    let callback = match timeout(LOGIN_TIMEOUT, rx).await {
        Ok(Ok(Ok(cb))) => cb,
        Ok(Ok(Err(e))) => return Err(e),
        Ok(Err(_)) => return Err(OAuthError::Callback("callback channel closed".into())),
        Err(_) => return Err(OAuthError::Timeout),
    };

    if callback.state != state {
        return Err(OAuthError::StateMismatch);
    }

    exchange_code(&callback.code, &verifier).await
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(code: &str, verifier: &str) -> Result<OAuthToken, OAuthError> {
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", verifier),
    ];
    post_token(&form).await
}

/// Refresh an access token using its refresh token.
pub async fn refresh(refresh_token: &str) -> Result<OAuthToken, OAuthError> {
    let form = [
        ("grant_type", "refresh_token"),
        ("client_id", CLIENT_ID),
        ("refresh_token", refresh_token),
        ("scope", SCOPES),
    ];
    post_token(&form).await
}

async fn post_token(form: &[(&str, &str)]) -> Result<OAuthToken, OAuthError> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(30))
        .build()?;

    let response = client.post(TOKEN_ENDPOINT).form(form).send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        // Try to parse as { error, error_description }
        if let Ok(err) = serde_json::from_str::<TokenError>(&body) {
            if err.error == "invalid_grant" {
                return Err(OAuthError::InvalidGrant);
            }
            let desc = err.error_description.unwrap_or_else(|| err.error.clone());
            return Err(OAuthError::Server(format!("{}: {}", err.error, desc)));
        }
        return Err(OAuthError::Server(format!(
            "HTTP {status}: {}",
            body.lines().next().unwrap_or("")
        )));
    }

    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| OAuthError::Server(format!("malformed token response: {e}")))?;

    let (exp, account_id) = decode_jwt_exp_and_account_id(&parsed.access_token)?;

    Ok(OAuthToken {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at_unix: exp,
        account_id,
    })
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

async fn await_callback(
    listener: TcpListener,
    expected_state: &str,
) -> Result<Callback, OAuthError> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|e| OAuthError::Callback(format!("accept: {e}")))?;

        // Read up to the end of the headers.
        let mut buf = vec![0u8; 4096];
        let mut total = 0usize;
        loop {
            let n = stream
                .read(&mut buf[total..])
                .await
                .map_err(|e| OAuthError::Callback(format!("read: {e}")))?;
            if n == 0 {
                break;
            }
            total += n;
            if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") || total == buf.len() {
                break;
            }
        }
        let request = String::from_utf8_lossy(&buf[..total]);
        let target = request.lines().next().and_then(|line| {
            // "GET /auth/callback?... HTTP/1.1"
            let mut parts = line.split_whitespace();
            parts.next()?;
            parts.next().map(|s| s.to_string())
        });

        let target = match target {
            Some(t) => t,
            None => {
                let _ = write_html(&mut stream, 400, "Bad Request").await;
                continue;
            }
        };

        if !target.starts_with("/auth/callback") {
            let _ = write_html(&mut stream, 404, "Not Found").await;
            continue;
        }

        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
        let mut code = None;
        let mut state = None;
        let mut error = None;
        for pair in query.split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            let v = percent_decode(v);
            match k {
                "code" => code = Some(v),
                "state" => state = Some(v),
                "error" => error = Some(v),
                _ => {}
            }
        }

        if let Some(err) = error {
            let _ = write_html(
                &mut stream,
                400,
                &format!("Authorization failed: {err}. You can close this tab."),
            )
            .await;
            return Err(OAuthError::AuthDenied);
        }

        match (code, state) {
            (Some(code), Some(state)) => {
                if state != expected_state {
                    let _ = write_html(&mut stream, 400, "State mismatch").await;
                    return Err(OAuthError::StateMismatch);
                }
                let _ = write_html(
                    &mut stream,
                    200,
                    "Authentication successful — you can close this tab and return to the terminal.",
                )
                .await;
                return Ok(Callback { code, state });
            }
            _ => {
                let _ = write_html(&mut stream, 400, "Missing code or state").await;
                continue;
            }
        }
    }
}

async fn write_html(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Status",
    };
    let html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>smart-scribe</title></head>\
         <body style=\"font-family:system-ui;text-align:center;padding:48px;\">\
         <h1>smart-scribe</h1><p>{body}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await.ok();
    Ok(())
}

fn generate_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn challenge_from_verifier(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Minimal RFC3986 percent-encoder for query string values.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                    16,
                ) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_is_sha256_of_verifier_b64url() {
        let verifier = "abcdef";
        let challenge = challenge_from_verifier(verifier);
        let expected =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(b"abcdef"));
        assert_eq!(challenge, expected);
    }

    #[test]
    fn build_authorize_url_contains_required_params() {
        let url = build_authorize_url("CHAL", "STATE");
        assert!(url.starts_with(AUTH_ENDPOINT));
        assert!(url.contains("client_id=app_EMoamEEZ73f0CkXaXp7hrann"));
        assert!(url.contains("code_challenge=CHAL"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=STATE"));
        assert!(url.contains("scope=openid%20profile%20email%20offline_access"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback"));
        assert!(url.contains("originator=smart-scribe"));
        assert!(url.contains("codex_cli_simplified_flow=true"));
    }

    #[test]
    fn verifier_length_is_43_chars() {
        // 32 random bytes encoded as base64url-nopad is 43 chars
        assert_eq!(generate_verifier().len(), 43);
    }

    #[test]
    fn state_length_is_32_hex_chars() {
        assert_eq!(generate_state().len(), 32);
    }

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("plain"), "plain");
    }
}
