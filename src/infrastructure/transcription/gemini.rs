//! Gemini API transcriber adapter

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::transcription::{AudioData, SystemPrompt};

/// Gemini API model to use
const DEFAULT_MODEL: &str = "gemini-2.0-flash-lite";

/// Gemini API base URL
const API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

// Request types for Gemini API

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    system_instruction: Option<SystemInstruction>,
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<TextPart>,
}

#[derive(Debug, Serialize)]
struct TextPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkingConfig {
    thinking_budget: i32,
}

// Response types for Gemini API

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
    status: Option<String>,
    code: Option<i32>,
}

/// Gemini API transcriber
pub struct GeminiTranscriber {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiTranscriber {
    /// Create a new Gemini transcriber with the given API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create a new Gemini transcriber with a custom model
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Build the API URL
    fn api_url(&self) -> String {
        format!(
            "{}/{}:generateContent?key={}",
            API_BASE_URL, self.model, self.api_key
        )
    }

    /// Build the request body
    fn build_request(&self, audio: &AudioData, prompt: &SystemPrompt) -> GenerateContentRequest {
        GenerateContentRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: None,
                    inline_data: Some(InlineData {
                        mime_type: audio.mime_type().to_string(),
                        data: audio.to_base64(),
                    }),
                }],
            }],
            system_instruction: Some(SystemInstruction {
                parts: vec![TextPart {
                    text: prompt.content().to_string(),
                }],
            }),
            generation_config: Some(GenerationConfig {
                thinking_config: Some(ThinkingConfig {
                    thinking_budget: 0, // Disable thinking for faster response
                }),
            }),
        }
    }

    /// Extract text from response
    fn extract_text(response: &GenerateContentResponse) -> Option<String> {
        let parts: Vec<&str> = response
            .candidates
            .as_ref()?
            .first()?
            .content
            .as_ref()?
            .parts
            .as_ref()?
            .iter()
            .filter_map(|p| p.text.as_deref())
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(""))
        }
    }
}

#[async_trait]
impl Transcriber for GeminiTranscriber {
    async fn transcribe(
        &self,
        audio: &AudioData,
        prompt: &SystemPrompt,
    ) -> Result<String, TranscriptionError> {
        let url = self.api_url();
        let body = self.build_request(audio, prompt);

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        let status = response.status();

        // Handle HTTP errors
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

        // Parse response
        let response: GenerateContentResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

        // Check for API error in response body
        if let Some(error) = response.error {
            return Err(TranscriptionError::ApiError(error.message));
        }

        // Extract text from response
        let text = Self::extract_text(&response).ok_or(TranscriptionError::EmptyResponse)?;

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
    fn build_request_has_correct_structure() {
        let transcriber = GeminiTranscriber::new("test-key");
        let audio = AudioData::new(vec![1, 2, 3], Default::default());
        let prompt = SystemPrompt::default();

        let request = transcriber.build_request(&audio, &prompt);

        assert_eq!(request.contents.len(), 1);
        assert_eq!(request.contents[0].role, "user");
        assert!(request.contents[0].parts[0].inline_data.is_some());
        assert!(request.system_instruction.is_some());
        assert!(request.generation_config.is_some());
    }

    #[test]
    fn api_url_contains_model_and_key() {
        let transcriber = GeminiTranscriber::new("test-api-key");
        let url = transcriber.api_url();

        assert!(url.contains("gemini-2.0-flash-lite"));
        assert!(url.contains("test-api-key"));
        assert!(url.contains("generateContent"));
    }

    #[test]
    fn custom_model() {
        let transcriber = GeminiTranscriber::with_model("key", "custom-model");
        let url = transcriber.api_url();

        assert!(url.contains("custom-model"));
    }

    #[test]
    fn extract_text_from_response() {
        let response = GenerateContentResponse {
            candidates: Some(vec![Candidate {
                content: Some(CandidateContent {
                    parts: Some(vec![ResponsePart {
                        text: Some("Hello world".to_string()),
                    }]),
                }),
            }]),
            error: None,
        };

        let text = GeminiTranscriber::extract_text(&response);
        assert_eq!(text, Some("Hello world".to_string()));
    }

    #[test]
    fn extract_text_empty_response() {
        let response = GenerateContentResponse {
            candidates: None,
            error: None,
        };

        let text = GeminiTranscriber::extract_text(&response);
        assert!(text.is_none());
    }
}
