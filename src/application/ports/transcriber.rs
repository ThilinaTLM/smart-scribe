//! Transcription port interface

use async_trait::async_trait;
use thiserror::Error;

use crate::domain::transcription::AudioData;

/// Transcription errors
#[derive(Debug, Clone, Error)]
pub enum TranscriptionError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Not authenticated. Run `smart-scribe login` or set `OPENAI_API_KEY`.")]
    NotAuthenticated,

    #[error("Rate limit exceeded. Please try again later.")]
    RateLimited,

    #[error("Empty audio response")]
    EmptyResponse,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Failed to parse API response: {0}")]
    ParseError(String),

    #[error("API error: {0}")]
    ApiError(String),
}

/// Port for audio transcription
#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Transcribe audio data to text.
    ///
    /// # Arguments
    /// * `audio` - The audio data to transcribe
    ///
    /// # Returns
    /// The transcribed text or an error
    async fn transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError>;
}
