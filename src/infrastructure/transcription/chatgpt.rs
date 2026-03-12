//! ChatGPT transcriber adapter using the /backend-api/transcribe endpoint

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::transcription::{AudioData, SystemPrompt};

const SESSION_URL: &str = "https://chatgpt.com/api/auth/session";
const TRANSCRIBE_URL: &str = "https://chatgpt.com/backend-api/transcribe";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Deserialize)]
struct CookieEntry {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

struct ChatGptAuth {
    access_token: String,
    cookie_header: String,
    device_id: String,
}

/// ChatGPT transcriber using browser cookie authentication
pub struct ChatGptTranscriber {
    cookie_file: PathBuf,
    client: reqwest::Client,
    auth: tokio::sync::Mutex<Option<ChatGptAuth>>,
}

impl ChatGptTranscriber {
    /// Create a new ChatGPT transcriber with the given cookie file path
    pub fn new(cookie_file: PathBuf) -> Self {
        Self {
            cookie_file,
            client: reqwest::Client::new(),
            auth: tokio::sync::Mutex::new(None),
        }
    }

    /// Load cookies from the JSON file and build cookie header
    fn load_cookies(path: &Path) -> Result<(String, String), TranscriptionError> {
        let content = std::fs::read_to_string(path).map_err(|_| {
            TranscriptionError::RequestFailed(format!(
                "ChatGPT cookie file not found: {}",
                path.display()
            ))
        })?;

        let cookies: Vec<CookieEntry> = serde_json::from_str(&content).map_err(|e| {
            TranscriptionError::ParseError(format!("Invalid cookie file format: {}", e))
        })?;

        let cookie_header: String = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        // Extract oai-did (device ID) from cookies, or generate one
        let device_id = cookies
            .iter()
            .find(|c| c.name == "oai-did")
            .map(|c| c.value.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok((cookie_header, device_id))
    }

    /// Fetch access token from ChatGPT session endpoint
    async fn fetch_access_token(&self, cookie_header: &str) -> Result<String, TranscriptionError> {
        let response = self
            .client
            .get(SESSION_URL)
            .header("Cookie", cookie_header)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TranscriptionError::ApiError(format!(
                "ChatGPT session request failed with status {}",
                response.status()
            )));
        }

        let session: SessionResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

        session.access_token.ok_or_else(|| {
            TranscriptionError::ApiError(
                "ChatGPT session expired. Re-export cookies from browser.".to_string(),
            )
        })
    }

    /// Ensure we have valid auth, lazily initializing if needed
    async fn ensure_auth(&self) -> Result<(), TranscriptionError> {
        let mut guard = self.auth.lock().await;
        if guard.is_some() {
            return Ok(());
        }

        let (cookie_header, device_id) = Self::load_cookies(&self.cookie_file)?;
        let access_token = self.fetch_access_token(&cookie_header).await?;

        *guard = Some(ChatGptAuth {
            access_token,
            cookie_header,
            device_id,
        });

        Ok(())
    }

    /// Clear cached auth (e.g., on 401)
    async fn clear_auth(&self) {
        let mut guard = self.auth.lock().await;
        *guard = None;
    }

    /// Perform the transcription request
    async fn do_transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        let guard = self.auth.lock().await;
        let auth = guard
            .as_ref()
            .ok_or_else(|| TranscriptionError::RequestFailed("Auth not initialized".to_string()))?;

        let extension = audio.mime_type().extension();
        let mime_str = audio.mime_type().as_str();
        let filename = format!("whisper.{}", extension);

        // Compute duration_ms: use actual if available, otherwise estimate from byte size
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

        let response = self
            .client
            .post(TRANSCRIBE_URL)
            .header("Authorization", format!("Bearer {}", auth.access_token))
            .header("Cookie", &auth.cookie_header)
            .header("User-Agent", USER_AGENT)
            .header("oai-language", "en-US")
            .header("oai-device-id", &auth.device_id)
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
                "HTTP {}: {}",
                status, error_text
            )));
        }

        // Response is a JSON object with a "text" field
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
}

#[async_trait]
impl Transcriber for ChatGptTranscriber {
    async fn transcribe(
        &self,
        audio: &AudioData,
        _prompt: &SystemPrompt,
    ) -> Result<String, TranscriptionError> {
        // Ensure auth is initialized
        self.ensure_auth().await?;

        // Try transcription
        match self.do_transcribe(audio).await {
            Ok(text) => Ok(text),
            Err(TranscriptionError::InvalidApiKey) => {
                // Token may have expired, retry once with fresh auth
                self.clear_auth().await;
                self.ensure_auth().await?;
                self.do_transcribe(audio).await.map_err(|e| match e {
                    TranscriptionError::InvalidApiKey => TranscriptionError::ApiError(
                        "ChatGPT authentication failed. Re-export cookies from browser."
                            .to_string(),
                    ),
                    other => other,
                })
            }
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_cookies_parses_valid_json() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(
            file,
            r#"[{{"name":"session","value":"abc"}},{{"name":"oai-did","value":"device-123"}}]"#
        )
        .unwrap();

        let (header, device_id) = ChatGptTranscriber::load_cookies(file.path()).unwrap();
        assert!(header.contains("session=abc"));
        assert_eq!(device_id, "device-123");
    }

    #[test]
    fn load_cookies_generates_device_id_if_missing() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, r#"[{{"name":"session","value":"abc"}}]"#).unwrap();

        let (_, device_id) = ChatGptTranscriber::load_cookies(file.path()).unwrap();
        assert!(!device_id.is_empty());
    }

    #[test]
    fn load_cookies_error_on_missing_file() {
        let result = ChatGptTranscriber::load_cookies(Path::new("/nonexistent/cookies.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_cookies_error_on_invalid_json() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "not json").unwrap();

        let result = ChatGptTranscriber::load_cookies(file.path());
        assert!(result.is_err());
    }
}
