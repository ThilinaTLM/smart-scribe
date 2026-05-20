//! OpenAI `/v1/audio/transcriptions` adapter (API-key authentication).
//!
//! Used when `auth = "api_key"`. Sends multipart form with the FLAC payload
//! and the chosen Whisper model; expects `{"text": "..."}` back.

use async_trait::async_trait;

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::transcription::AudioData;

use super::{parse_transcription_response, shared_client};

const TRANSCRIBE_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

pub struct OpenAiApiTranscriber {
    api_key: String,
    model: String,
    prompt: Option<String>,
    language: Option<String>,
    client: reqwest::Client,
}

impl OpenAiApiTranscriber {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            prompt: None,
            language: None,
            client: shared_client(),
        }
    }

    /// Builder: attach a transcription prompt (OpenAI's documented accuracy lever).
    pub fn with_prompt(mut self, prompt: Option<String>) -> Self {
        self.prompt = prompt.filter(|s| !s.trim().is_empty());
        self
    }

    /// Builder: attach an ISO 639-1 language hint.
    pub fn with_language(mut self, language: Option<String>) -> Self {
        self.language = language.filter(|s| !s.trim().is_empty());
        self
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

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "json");
        if let Some(prompt) = &self.prompt {
            form = form.text("prompt", prompt.clone());
        }
        if let Some(language) = &self.language {
            form = form.text("language", language.clone());
        }

        let response = self
            .client
            .post(TRANSCRIBE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        parse_transcription_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_with_model_and_key() {
        let t = OpenAiApiTranscriber::new("sk-test", "gpt-4o-transcribe");
        assert_eq!(t.api_key, "sk-test");
        assert_eq!(t.model, "gpt-4o-transcribe");
        assert!(t.prompt.is_none());
        assert!(t.language.is_none());
    }

    #[test]
    fn with_prompt_trims_and_drops_empty() {
        let t = OpenAiApiTranscriber::new("k", "m")
            .with_prompt(Some("   ".into()))
            .with_language(Some("".into()));
        assert!(t.prompt.is_none());
        assert!(t.language.is_none());

        let t = OpenAiApiTranscriber::new("k", "m")
            .with_prompt(Some("Rust, OAuth".into()))
            .with_language(Some("en".into()));
        assert_eq!(t.prompt.as_deref(), Some("Rust, OAuth"));
        assert_eq!(t.language.as_deref(), Some("en"));
    }
}
