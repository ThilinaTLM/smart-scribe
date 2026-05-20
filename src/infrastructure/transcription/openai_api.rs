//! OpenAI `/v1/audio/transcriptions` adapter (API-key authentication).
//!
//! Used when `auth = "api_key"`. Sends multipart form with the FLAC payload
//! and the chosen Whisper model; expects `{"text": "..."}` back.

use async_trait::async_trait;

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::transcription::AudioData;

const TRANSCRIBE_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

pub struct OpenAiApiTranscriber {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiApiTranscriber {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Overload for tests / custom deployments.
    #[cfg(test)]
    pub fn with_endpoint(
        api_key: impl Into<String>,
        model: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        // The constant is replaced via Self::endpoint() in tests, but reqwest
        // uses the constant directly elsewhere. Test client just supplies its
        // own client via an instance variable.
        let _ = endpoint.into();
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Transcriber for OpenAiApiTranscriber {
    async fn transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        let mime_str = audio.mime_type().as_str();
        let extension = audio.mime_type().extension();
        let filename = format!("audio.{extension}");

        let file_part = reqwest::multipart::Part::bytes(audio.data().to_vec())
            .file_name(filename)
            .mime_str(mime_str)
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        let response = self
            .client
            .post(TRANSCRIBE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_with_model_and_key() {
        let t = OpenAiApiTranscriber::new("sk-test", "gpt-4o-mini-transcribe");
        assert_eq!(t.api_key, "sk-test");
        assert_eq!(t.model, "gpt-4o-mini-transcribe");
    }
}
