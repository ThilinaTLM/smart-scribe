//! Transcription infrastructure module.
//!
//! Exposes the two concrete adapters (OAuth and API key) plus a small
//! [`Transcriber`] enum that dispatches between them at runtime, the
//! [`create_transcriber`] factory the CLI uses as its only entry point, and
//! a shared response parser so both adapters speak the same error language.

mod chatgpt_oauth;
mod openai_api;

pub use chatgpt_oauth::ChatGptOAuthTranscriber;
pub use openai_api::OpenAiApiTranscriber;

use std::sync::OnceLock;

use async_trait::async_trait;

use crate::application::ports::{Transcriber as TranscriberPort, TranscriptionError};
use crate::domain::config::{AppConfig, AuthMode};
use crate::domain::transcription::AudioData;
use crate::infrastructure::auth::OAuthStore;

/// Process-wide shared `reqwest::Client`.
///
/// Connections and DNS resolutions are pooled across the two transcription
/// adapters and the OAuth refresh path (see [`Self::shared_client`]). The
/// alternative — each adapter holding its own client — leaves us with three
/// independent pools that each pay a TLS handshake on first use.
fn shared_client_cell() -> &'static reqwest::Client {
    static CELL: OnceLock<reqwest::Client> = OnceLock::new();
    CELL.get_or_init(reqwest::Client::new)
}

/// Public accessor for the shared client. Returned by clone so consumers
/// can call instance methods (`.post(...)` etc.) without borrow contention.
pub(crate) fn shared_client() -> reqwest::Client {
    shared_client_cell().clone()
}

/// Parse a transcription HTTP response into either the trimmed transcript
/// text or a typed [`TranscriptionError`].
///
/// Both the OpenAI API and the ChatGPT OAuth endpoint return the same
/// `{"text": "..."}` shape and map auth/quota the same way. Keeping the
/// parsing logic here keeps the adapters short and means a status-code
/// fix lands in one place.
pub(crate) async fn parse_transcription_response(
    response: reqwest::Response,
) -> Result<String, TranscriptionError> {
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

/// Runtime dispatch between the two transcription adapters.
///
/// Kept as an enum (rather than `Box<dyn TranscriberPort>`) so the use cases
/// retain static dispatch and so tests can `match` the variant directly.
pub enum Transcriber {
    Oauth(ChatGptOAuthTranscriber),
    ApiKey(OpenAiApiTranscriber),
}

#[async_trait]
impl TranscriberPort for Transcriber {
    async fn transcribe(&self, audio: &AudioData) -> Result<String, TranscriptionError> {
        match self {
            Self::Oauth(t) => t.transcribe(audio).await,
            Self::ApiKey(t) => t.transcribe(audio).await,
        }
    }
}

/// Build the right transcriber for a validated [`AppConfig`].
///
/// For OAuth we construct the transcriber even if no token is yet on disk —
/// the missing-token error is surfaced at the first transcribe call so that
/// `smart-scribe login` can still be used to populate it.
pub fn create_transcriber(config: &AppConfig) -> Result<Transcriber, String> {
    let model = config.openai_transcribe_model.clone();
    let prompt = config.transcribe_prompt_some().map(str::to_string);
    let language = config.transcribe_language_some().map(str::to_string);

    match config.auth {
        AuthMode::Oauth => {
            let store = OAuthStore::new()
                .map_err(|e| format!("Could not initialize OAuth token store: {e}"))?;
            Ok(Transcriber::Oauth(
                ChatGptOAuthTranscriber::new(store, model)
                    .with_prompt(prompt)
                    .with_language(language),
            ))
        }
        AuthMode::ApiKey => {
            let api_key = config.openai_api_key.as_ref().ok_or_else(|| {
                "Missing OpenAI API key. Set OPENAI_API_KEY or run \
                 'smart-scribe config set openai_api_key <key>'."
                    .to_string()
            })?;
            Ok(Transcriber::ApiKey(
                OpenAiApiTranscriber::new(api_key, model)
                    .with_prompt(prompt)
                    .with_language(language),
            ))
        }
    }
}
