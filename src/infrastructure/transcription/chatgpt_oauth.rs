//! ChatGPT transcription via OAuth bearer tokens (ChatGPT subscription).
//!
//! Uses the same `/backend-api/transcribe` endpoint that chatgpt.com itself
//! talks to. Authentication is the Bearer token issued by the Codex CLI OAuth
//! client (see `infrastructure::auth`). The endpoint requires the full set of
//! browser-fetch headers to pass Cloudflare; sending only `Authorization` and
//! a generic UA yields a 403 interstitial.

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::transcription::AudioData;
use crate::infrastructure::auth::{refresh, OAuthStore, OAuthToken};

const TRANSCRIBE_URL: &str = "https://chatgpt.com/backend-api/transcribe";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const REFRESH_LEAD_SECS: i64 = 60;

/// ChatGPT transcriber using a bearer token from the OAuth flow.
pub struct ChatGptOAuthTranscriber {
    store: OAuthStore,
    client: reqwest::Client,
    device_id: String,
    cached: Mutex<Option<OAuthToken>>,
}

impl ChatGptOAuthTranscriber {
    pub fn new(store: OAuthStore) -> Self {
        Self {
            store,
            client: reqwest::Client::new(),
            device_id: Uuid::new_v4().to_string(),
            cached: Mutex::new(None),
        }
    }

    /// Ensure we have a non-expired access token, refreshing and persisting as
    /// necessary. Returns the (possibly refreshed) token by value for use in a
    /// single request.
    async fn current_token(&self) -> Result<OAuthToken, TranscriptionError> {
        let mut guard = self.cached.lock().await;

        // Load from disk if not cached.
        if guard.is_none() {
            let loaded = self
                .store
                .load()
                .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;
            match loaded {
                Some(tok) => *guard = Some(tok),
                None => return Err(TranscriptionError::NotAuthenticated),
            }
        }

        // Refresh if near expiry.
        let needs_refresh = guard
            .as_ref()
            .map(|t| t.is_expired_within(REFRESH_LEAD_SECS))
            .unwrap_or(true);

        if needs_refresh {
            let refresh_token = guard
                .as_ref()
                .map(|t| t.refresh_token.clone())
                .ok_or(TranscriptionError::NotAuthenticated)?;
            let fresh = refresh(&refresh_token).await.map_err(|e| {
                use crate::infrastructure::auth::OAuthError;
                match e {
                    OAuthError::InvalidGrant => TranscriptionError::NotAuthenticated,
                    other => TranscriptionError::RequestFailed(other.to_string()),
                }
            })?;
            self.store
                .save(&fresh)
                .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;
            *guard = Some(fresh);
        }

        Ok(guard.as_ref().expect("token populated above").clone())
    }

    async fn invalidate_cache(&self) {
        let mut guard = self.cached.lock().await;
        *guard = None;
    }

    async fn do_transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        let token = self.current_token().await?;
        send_transcribe(&self.client, &token, &self.device_id, audio).await
    }
}

async fn send_transcribe(
    client: &reqwest::Client,
    token: &OAuthToken,
    device_id: &str,
    audio: &AudioData,
) -> Result<String, TranscriptionError> {
    let mime_str = audio.mime_type().as_str();
    let extension = audio.mime_type().extension();
    let filename = format!("whisper.{extension}");

    let duration_ms = audio.duration_ms().unwrap_or_else(|| {
        // Rough estimate: FLAC at 16kHz mono ~= 50KB/s
        let bytes = audio.size_bytes() as u64;
        (bytes * 1000) / 50_000
    });

    let file_part = reqwest::multipart::Part::bytes(audio.data().to_vec())
        .file_name(filename)
        .mime_str(mime_str)
        .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("duration_ms", duration_ms.to_string());

    let response = client
        .post(TRANSCRIBE_URL)
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Origin", "https://chatgpt.com")
        .header("Referer", "https://chatgpt.com/")
        .header("oai-language", "en-US")
        .header("oai-device-id", device_id)
        .header("chatgpt-account-id", &token.account_id)
        .header(
            "sec-ch-ua",
            "\"Chromium\";v=\"131\", \"Not(A:Brand\";v=\"24\"",
        )
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Linux\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .multipart(form)
        .send()
        .await
        .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(TranscriptionError::InvalidApiKey);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(TranscriptionError::RateLimited);
    }
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(TranscriptionError::ApiError(format!(
            "HTTP {status}: {error_text}"
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

    let text = body
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or(TranscriptionError::EmptyResponse)?;

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(TranscriptionError::EmptyResponse);
    }

    Ok(trimmed.to_string())
}

#[async_trait]
impl Transcriber for ChatGptOAuthTranscriber {
    async fn transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        match self.do_transcribe(audio).await {
            Ok(text) => Ok(text),
            Err(TranscriptionError::InvalidApiKey) => {
                // Drop the cache so the next attempt forces a refresh from disk.
                self.invalidate_cache().await;
                match self.do_transcribe(audio).await {
                    Ok(text) => Ok(text),
                    Err(TranscriptionError::InvalidApiKey) => Err(TranscriptionError::ApiError(
                        "OAuth token rejected. Run `smart-scribe login` again.".to_string(),
                    )),
                    Err(other) => Err(other),
                }
            }
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transcription::AudioMimeType;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::tempdir;

    fn make_store_with_token() -> (tempfile::TempDir, OAuthStore) {
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let token = OAuthToken {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            expires_at_unix: now + 3600,
            account_id: "acc-1".into(),
        };
        store.save(&token).unwrap();
        (dir, store)
    }

    #[tokio::test]
    async fn returns_not_authenticated_when_store_empty() {
        let dir = tempdir().unwrap();
        let store = OAuthStore::with_path(dir.path().join("oauth.json"));
        let t = ChatGptOAuthTranscriber::new(store);
        let audio = AudioData::new(vec![0u8; 8], AudioMimeType::Flac);
        let err = t.transcribe(&audio).await.unwrap_err();
        assert!(matches!(err, TranscriptionError::NotAuthenticated));
    }

    #[tokio::test]
    async fn current_token_loads_from_store_and_caches() {
        let (_dir, store) = make_store_with_token();
        let t = ChatGptOAuthTranscriber::new(store);
        let tok = t.current_token().await.unwrap();
        assert_eq!(tok.access_token, "access-1");
        // Subsequent call should still succeed from cache (no network).
        let tok2 = t.current_token().await.unwrap();
        assert_eq!(tok2.access_token, "access-1");
    }
}
