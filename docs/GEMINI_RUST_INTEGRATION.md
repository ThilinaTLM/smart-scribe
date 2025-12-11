# Google Gemini API Integration for Rust

This document describes how to integrate Google Gemini API with Rust for audio transcription, based on SmartScribe's requirements.

---

## Table of Contents

1. [Overview](#1-overview)
2. [API Reference](#2-api-reference)
3. [Implementation Approaches](#3-implementation-approaches)
4. [Manual REST Implementation](#4-manual-rest-implementation)
5. [Using Existing Crates](#5-using-existing-crates)
6. [Error Handling](#6-error-handling)
7. [Testing](#7-testing)

---

## 1. Overview

### 1.1 Current TypeScript Implementation

SmartScribe currently uses these Gemini features:
- **Model:** `gemini-2.0-flash-lite`
- **Feature:** Audio transcription via inline base64 data
- **Config:** System instructions + disabled thinking (`thinkingBudget: 0`)

### 1.2 Rust Options

| Approach | Pros | Cons |
|----------|------|------|
| Manual REST (reqwest) | Full control, minimal dependencies | More code to maintain |
| `gemini-rust` crate | Feature-rich, maintained | May have unnecessary features |
| `reqwest` + serde | Lightweight, flexible | Build types yourself |

**Recommendation:** Manual REST with `reqwest` + `serde` for maximum control and minimal dependencies, matching our specific use case.

---

## 2. API Reference

### 2.1 Endpoint

```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
```

For SmartScribe:
```
POST https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent
```

### 2.2 Authentication

API key passed via query parameter:
```
?key={GEMINI_API_KEY}
```

Or via header:
```
x-goog-api-key: {GEMINI_API_KEY}
```

### 2.3 Request Structure

```json
{
  "contents": [
    {
      "role": "user",
      "parts": [
        {
          "inlineData": {
            "mimeType": "audio/ogg",
            "data": "<base64-encoded-audio>"
          }
        }
      ]
    }
  ],
  "systemInstruction": {
    "parts": [
      {
        "text": "<system-prompt>"
      }
    ]
  },
  "generationConfig": {
    "thinkingConfig": {
      "thinkingBudget": 0
    }
  }
}
```

### 2.4 Response Structure

```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "text": "<transcribed-text>"
          }
        ],
        "role": "model"
      },
      "finishReason": "STOP",
      "safetyRatings": [...]
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 1920,
    "candidatesTokenCount": 50,
    "totalTokenCount": 1970
  },
  "modelVersion": "gemini-2.0-flash-lite"
}
```

### 2.5 Constraints

| Constraint | Value |
|------------|-------|
| Max inline request size | 20 MB |
| Max audio length | 9.5 hours |
| Audio token rate | 32 tokens/second |
| Supported formats | WAV, MP3, AIFF, AAC, OGG, FLAC |

---

## 3. Implementation Approaches

### 3.1 Recommended: Manual REST with reqwest

**Dependencies (Cargo.toml):**
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"
thiserror = "2.0"
tokio = { version = "1", features = ["full"] }
```

### 3.2 Alternative: gemini-rust Crate

**Dependencies:**
```toml
[dependencies]
gemini-rust = "1.5"
tokio = { version = "1", features = ["full"] }
```

**Pros:**
- Ready-made client with builder pattern
- Supports all Gemini features
- Well-documented with examples

**Cons:**
- Larger dependency footprint
- May include features we don't need
- External dependency to track

---

## 4. Manual REST Implementation

### 4.1 Type Definitions

```rust
use serde::{Deserialize, Serialize};

/// Supported audio MIME types
#[derive(Debug, Clone, Copy, Serialize)]
pub enum AudioMimeType {
    #[serde(rename = "audio/ogg")]
    Ogg,
    #[serde(rename = "audio/mp3")]
    Mp3,
    #[serde(rename = "audio/mpeg")]
    Mpeg,
    #[serde(rename = "audio/wav")]
    Wav,
    #[serde(rename = "audio/flac")]
    Flac,
    #[serde(rename = "audio/aac")]
    Aac,
}

/// Inline data (base64-encoded audio)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    pub mime_type: AudioMimeType,
    pub data: String, // base64-encoded
}

/// A part of content (can be text or inline data)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<InlineData>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            inline_data: None,
        }
    }

    pub fn inline_audio(mime_type: AudioMimeType, base64_data: String) -> Self {
        Self {
            text: None,
            inline_data: Some(InlineData {
                mime_type,
                data: base64_data,
            }),
        }
    }
}

/// Content with role and parts
#[derive(Debug, Serialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

/// System instruction
#[derive(Debug, Serialize)]
pub struct SystemInstruction {
    pub parts: Vec<Part>,
}

/// Thinking configuration
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    pub thinking_budget: i32,
}

/// Generation configuration
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Request body for generateContent
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}
```

### 4.2 Response Types

```rust
/// Response part (text content)
#[derive(Debug, Deserialize)]
pub struct ResponsePart {
    pub text: Option<String>,
}

/// Response content
#[derive(Debug, Deserialize)]
pub struct ResponseContent {
    pub parts: Vec<ResponsePart>,
    pub role: String,
}

/// Candidate response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: ResponseContent,
    pub finish_reason: Option<String>,
}

/// Usage metadata
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
}

/// Error detail from API
#[derive(Debug, Deserialize)]
pub struct ApiErrorDetail {
    pub code: Option<i32>,
    pub message: Option<String>,
    pub status: Option<String>,
}

/// Error response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

/// Full response from generateContent
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub usage_metadata: Option<UsageMetadata>,
    pub model_version: Option<String>,
}
```

### 4.3 Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeminiError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({code}): {message}")]
    ApiError { code: i32, message: String },

    #[error("Empty response from Gemini")]
    EmptyResponse,

    #[error("No text in response")]
    NoTextContent,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Request too large (max 20MB for inline data)")]
    RequestTooLarge,
}
```

### 4.4 Client Implementation

```rust
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-2.0-flash-lite";

pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Create client with custom model
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Transcribe audio to text
    pub async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: AudioMimeType,
        system_prompt: &str,
    ) -> Result<String, GeminiError> {
        // Encode audio as base64
        let base64_audio = BASE64.encode(audio_data);

        // Build request
        let request = GenerateContentRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::inline_audio(mime_type, base64_audio)],
            }],
            system_instruction: Some(SystemInstruction {
                parts: vec![Part::text(system_prompt)],
            }),
            generation_config: Some(GenerationConfig {
                thinking_config: Some(ThinkingConfig {
                    thinking_budget: 0, // Disable thinking for speed
                }),
                max_output_tokens: None,
                temperature: None,
            }),
        };

        // Send request
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_BASE, self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        // Handle response status
        let status = response.status();
        if !status.is_success() {
            return self.handle_error_response(response).await;
        }

        // Parse response
        let response: GenerateContentResponse = response.json().await?;

        // Extract text
        self.extract_text(response)
    }

    /// Handle error response from API
    async fn handle_error_response(
        &self,
        response: reqwest::Response,
    ) -> Result<String, GeminiError> {
        let status = response.status();

        // Try to parse error response
        if let Ok(error_response) = response.json::<ApiErrorResponse>().await {
            let code = error_response.error.code.unwrap_or(status.as_u16() as i32);
            let message = error_response
                .error
                .message
                .unwrap_or_else(|| "Unknown error".to_string());

            // Map specific error codes
            return Err(match code {
                401 | 403 => GeminiError::InvalidApiKey,
                429 => GeminiError::RateLimitExceeded,
                413 => GeminiError::RequestTooLarge,
                _ => GeminiError::ApiError { code, message },
            });
        }

        Err(GeminiError::ApiError {
            code: status.as_u16() as i32,
            message: status.to_string(),
        })
    }

    /// Extract text from response
    fn extract_text(&self, response: GenerateContentResponse) -> Result<String, GeminiError> {
        let candidates = response.candidates.ok_or(GeminiError::EmptyResponse)?;

        let candidate = candidates.first().ok_or(GeminiError::EmptyResponse)?;

        let text = candidate
            .content
            .parts
            .iter()
            .filter_map(|part| part.text.as_ref())
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(GeminiError::NoTextContent);
        }

        Ok(text.trim().to_string())
    }
}
```

### 4.5 Usage Example

```rust
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key
    let api_key = std::env::var("GEMINI_API_KEY")?;

    // Create client
    let client = GeminiClient::new(api_key);

    // Read audio file
    let audio_data = fs::read("recording.ogg")?;

    // System prompt for transcription
    let system_prompt = r#"
You are a voice-to-text assistant that transcribes audio into grammatically
correct, context-aware text output.

Instructions:
- Remove filler words (um, ah, like, you know)
- Must have correct grammar and punctuation
- Do NOT transcribe stutters, false starts, or repeated words
- Output ONLY the final cleaned text
- Do NOT include meta-commentary or explanations

Domain Context: Software Engineering
Focus on programming terminology, variable naming conventions,
and technical jargon.
"#;

    // Transcribe
    let text = client
        .transcribe(&audio_data, AudioMimeType::Ogg, system_prompt)
        .await?;

    println!("{}", text);

    Ok(())
}
```

---

## 5. Using Existing Crates

### 5.1 gemini-rust Crate

If you prefer using the `gemini-rust` crate:

```rust
use gemini_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;

    // Create client
    let client = Gemini::builder()
        .api_key(api_key)
        .model("gemini-2.0-flash-lite")
        .build()?;

    // Read audio
    let audio_bytes = std::fs::read("recording.ogg")?;

    // Build content with audio
    let content = ContentBuilder::new()
        .add_blob(Blob {
            mime_type: "audio/ogg".to_string(),
            data: audio_bytes,
        })
        .build();

    // Generate with system instruction
    let response = client
        .generate()
        .system_instruction("Your transcription prompt here...")
        .content(content)
        .send()
        .await?;

    println!("{}", response.text()?);

    Ok(())
}
```

**Note:** Verify the exact API of `gemini-rust` as it may differ. The crate is actively maintained and the API may have changed.

### 5.2 Direct reqwest (Minimal)

For the most minimal approach without custom types:

```rust
use reqwest::Client;
use serde_json::{json, Value};
use base64::{Engine, engine::general_purpose::STANDARD};

async fn transcribe(
    api_key: &str,
    audio_data: &[u8],
    system_prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let base64_audio = STANDARD.encode(audio_data);

    let body = json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "inlineData": {
                    "mimeType": "audio/ogg",
                    "data": base64_audio
                }
            }]
        }],
        "systemInstruction": {
            "parts": [{"text": system_prompt}]
        },
        "generationConfig": {
            "thinkingConfig": {
                "thinkingBudget": 0
            }
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent?key={}",
        api_key
    );

    let response: Value = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let text = response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("No text in response")?;

    Ok(text.trim().to_string())
}
```

---

## 6. Error Handling

### 6.1 HTTP Status Codes

| Status | Meaning | Action |
|--------|---------|--------|
| 200 | Success | Parse response |
| 400 | Bad request | Check request format |
| 401 | Unauthorized | Invalid API key |
| 403 | Forbidden | API key lacks permissions |
| 404 | Not found | Invalid model name |
| 429 | Rate limited | Implement retry with backoff |
| 500 | Server error | Retry with backoff |
| 503 | Service unavailable | Retry with backoff |

### 6.2 Retry Strategy

```rust
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

async fn transcribe_with_retry(
    client: &GeminiClient,
    audio_data: &[u8],
    mime_type: AudioMimeType,
    system_prompt: &str,
) -> Result<String, GeminiError> {
    let mut last_error = None;
    let mut backoff = INITIAL_BACKOFF_MS;

    for attempt in 0..MAX_RETRIES {
        match client.transcribe(audio_data, mime_type, system_prompt).await {
            Ok(text) => return Ok(text),
            Err(e) => {
                // Only retry on transient errors
                if matches!(e, GeminiError::RateLimitExceeded | GeminiError::HttpError(_)) {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        sleep(Duration::from_millis(backoff)).await;
                        backoff *= 2; // Exponential backoff
                    }
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

### 6.3 Response Validation

```rust
impl GeminiClient {
    fn validate_response(&self, response: &GenerateContentResponse) -> Result<(), GeminiError> {
        // Check for candidates
        let candidates = response.candidates.as_ref().ok_or(GeminiError::EmptyResponse)?;

        if candidates.is_empty() {
            return Err(GeminiError::EmptyResponse);
        }

        // Check finish reason
        if let Some(reason) = &candidates[0].finish_reason {
            match reason.as_str() {
                "STOP" => Ok(()),
                "MAX_TOKENS" => Ok(()), // Still usable, but truncated
                "SAFETY" => Err(GeminiError::ApiError {
                    code: 400,
                    message: "Content blocked by safety filters".to_string(),
                }),
                "RECITATION" => Err(GeminiError::ApiError {
                    code: 400,
                    message: "Content blocked due to recitation".to_string(),
                }),
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}
```

---

## 7. Testing

### 7.1 Unit Tests with Mock Server

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path_regex};

    #[tokio::test]
    async fn test_transcribe_success() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Setup mock response
        Mock::given(method("POST"))
            .and(path_regex(r"/v1beta/models/.+:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": {
                        "parts": [{"text": "Hello world"}],
                        "role": "model"
                    },
                    "finishReason": "STOP"
                }]
            })))
            .mount(&mock_server)
            .await;

        // Create client pointing to mock
        let client = GeminiClient {
            client: reqwest::Client::new(),
            api_key: "test-key".to_string(),
            model: "gemini-2.0-flash-lite".to_string(),
            base_url: mock_server.uri(), // Would need to add this field
        };

        let result = client
            .transcribe(b"fake audio", AudioMimeType::Ogg, "test prompt")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn test_transcribe_empty_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": []
            })))
            .mount(&mock_server)
            .await;

        // Test should return EmptyResponse error
    }

    #[tokio::test]
    async fn test_transcribe_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "code": 401,
                    "message": "API key not valid",
                    "status": "UNAUTHENTICATED"
                }
            })))
            .mount(&mock_server)
            .await;

        // Test should return InvalidApiKey error
    }
}
```

### 7.2 Integration Test

```rust
#[tokio::test]
#[ignore] // Run manually with: cargo test -- --ignored
async fn test_real_transcription() {
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY must be set");

    let client = GeminiClient::new(api_key);

    // Create a simple test audio (or use a test file)
    let audio_data = std::fs::read("tests/fixtures/test_audio.ogg")
        .expect("Test audio file not found");

    let result = client
        .transcribe(&audio_data, AudioMimeType::Ogg, "Transcribe this audio.")
        .await;

    assert!(result.is_ok());
    let text = result.unwrap();
    assert!(!text.is_empty());
    println!("Transcription: {}", text);
}
```

---

## Appendix A: Complete Cargo.toml

```toml
[package]
name = "smart-scribe"
version = "0.1.0"
edition = "2021"

[dependencies]
# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Base64 encoding
base64 = "0.22"

# Error handling
thiserror = "2.0"

# Async runtime
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
# Mock server for testing
wiremock = "0.6"
```

---

## Appendix B: API Quirks and Notes

### B.1 Model Naming

- Use `gemini-2.0-flash-lite` (not `gemini-2.0-flash-lite-001`)
- Model names are case-sensitive

### B.2 Thinking Config

- `thinkingBudget: 0` disables thinking for faster responses
- Only works with Flash/Flash-Lite models
- Gemini 2.5 Pro cannot disable thinking
- Gemini 3 uses `thinkingLevel` instead

### B.3 Audio Processing

- Gemini downsamples to 16 kbps internally
- Multi-channel audio is mixed to mono
- 32 tokens per second of audio
- Our 16kHz mono OGG/Opus format is optimal

### B.4 Rate Limits

- Free tier: 15 RPM (requests per minute)
- Paid tier: Higher limits, check Google's pricing
- Implement exponential backoff for 429 responses

### B.5 Request Size

- Inline data limit: 20 MB total request size
- For larger files, use the Files API (upload first)
- Our typical 60s recording at 16kbps = ~120KB (well under limit)

---

## Appendix C: References

- [Gemini API Documentation](https://ai.google.dev/gemini-api/docs)
- [Audio Understanding Guide](https://ai.google.dev/gemini-api/docs/audio)
- [Generating Content API Reference](https://ai.google.dev/api/generate-content)
- [Thinking Mode Documentation](https://ai.google.dev/gemini-api/docs/thinking)
- [gemini-rust crate](https://crates.io/crates/gemini-rust)
- [Gemini Models Overview](https://ai.google.dev/gemini-api/docs/models)
