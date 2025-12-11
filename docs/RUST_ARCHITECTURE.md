# SmartScribe Rust Architecture Plan

This document outlines the architectural design for implementing SmartScribe in Rust, maintaining the hexagonal architecture pattern while leveraging Rust's strengths.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Technology Stack](#2-technology-stack)
3. [Project Structure](#3-project-structure)
4. [Domain Layer](#4-domain-layer)
5. [Application Layer](#5-application-layer)
6. [Infrastructure Layer](#6-infrastructure-layer)
7. [CLI Layer](#7-cli-layer)
8. [Error Handling Strategy](#8-error-handling-strategy)
9. [Async Architecture](#9-async-architecture)
10. [Configuration System](#10-configuration-system)
11. [Signal Handling](#11-signal-handling)
12. [Testing Strategy](#12-testing-strategy)
13. [Build and Distribution](#13-build-and-distribution)
14. [Migration Checklist](#14-migration-checklist)

---

## 1. Overview

### 1.1 Design Principles

| Principle | Implementation |
|-----------|----------------|
| Hexagonal Architecture | Ports (traits) + Adapters (implementations) |
| Type Safety | Leverage Rust's type system for domain validation |
| Zero-Cost Abstractions | Traits with static dispatch where possible |
| Explicit Error Handling | `Result<T, E>` throughout, no panics |
| Minimal Dependencies | Prefer std library, add deps only when necessary |

### 1.2 Key Rust Advantages

- **Compile-time guarantees** for state machine transitions
- **Ownership model** ensures resource cleanup (temp files, processes)
- **Enums with data** for rich error types
- **Pattern matching** for exhaustive handling
- **No garbage collector** - predictable performance

---

## 2. Technology Stack

### 2.1 Core Dependencies

| Purpose | Crate | Version | Justification |
|---------|-------|---------|---------------|
| Async Runtime | `tokio` | 1.x | Industry standard, required by reqwest |
| HTTP Client | `reqwest` | 0.12 | Mature, async, JSON support |
| CLI Parser | `clap` | 4.x | Derive macros, subcommands, excellent UX |
| Serialization | `serde` + `serde_json` | 1.x | De facto standard |
| TOML Config | `toml` | 0.8 | Config file parsing |
| Error Handling | `thiserror` | 2.x | Derive Error trait |
| Base64 | `base64` | 0.22 | Audio encoding |
| Colored Output | `colored` | 2.x | Terminal colors |
| Spinner | `indicatif` | 0.17 | Progress bars and spinners |

### 2.2 Optional/Conditional Dependencies

| Purpose | Crate | When |
|---------|-------|------|
| Signal Handling | `tokio` (signal feature) | Unix signals |
| Directory Paths | `dirs` | XDG config paths |
| Environment | `dotenvy` | .env file support |

### 2.3 Cargo.toml

```toml
[package]
name = "smart-scribe"
version = "2.0.0"
edition = "2021"
authors = ["Your Name"]
description = "AI-powered audio transcription CLI"
license = "MIT"
repository = "https://github.com/..."
keywords = ["transcription", "audio", "gemini", "cli"]
categories = ["command-line-utilities", "multimedia::audio"]

[dependencies]
# Async
tokio = { version = "1", features = ["full", "signal"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Utilities
base64 = "0.22"
thiserror = "2"
colored = "2"
indicatif = "0.17"
dirs = "5"
dotenvy = "0.15"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
assert_cmd = "2"
predicates = "3"

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"

[[bin]]
name = "smart-scribe"
path = "src/main.rs"
```

---

## 3. Project Structure

```
smart-scribe/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                 # Entry point
│   ├── lib.rs                  # Library root (re-exports)
│   │
│   ├── domain/                 # Core business logic
│   │   ├── mod.rs
│   │   ├── recording/
│   │   │   ├── mod.rs
│   │   │   └── duration.rs     # Duration value object
│   │   ├── transcription/
│   │   │   ├── mod.rs
│   │   │   ├── audio_data.rs   # AudioData value object
│   │   │   ├── domain_preset.rs # DomainPreset enum
│   │   │   └── system_prompt.rs # SystemPrompt builder
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── app_config.rs   # AppConfig value object
│   │   ├── daemon/
│   │   │   ├── mod.rs
│   │   │   └── session.rs      # DaemonSession state machine
│   │   └── error.rs            # Domain errors
│   │
│   ├── application/            # Use cases and ports
│   │   ├── mod.rs
│   │   ├── ports/
│   │   │   ├── mod.rs
│   │   │   ├── recorder.rs     # AudioRecorder trait
│   │   │   ├── transcriber.rs  # Transcriber trait
│   │   │   ├── clipboard.rs    # Clipboard trait
│   │   │   ├── keystroke.rs    # Keystroke trait
│   │   │   ├── notifier.rs     # Notifier trait
│   │   │   └── config.rs       # ConfigStore trait
│   │   ├── transcribe.rs       # TranscribeRecordingUseCase
│   │   └── daemon.rs           # DaemonTranscriptionUseCase
│   │
│   ├── infrastructure/         # External adapters
│   │   ├── mod.rs
│   │   ├── recording/
│   │   │   ├── mod.rs
│   │   │   └── ffmpeg.rs       # FFmpegRecorder
│   │   ├── transcription/
│   │   │   ├── mod.rs
│   │   │   └── gemini.rs       # GeminiTranscriber
│   │   ├── clipboard/
│   │   │   ├── mod.rs
│   │   │   └── wayland.rs      # WaylandClipboard
│   │   ├── keystroke/
│   │   │   ├── mod.rs
│   │   │   └── xdotool.rs      # XdotoolKeystroke
│   │   ├── notification/
│   │   │   ├── mod.rs
│   │   │   └── notify_send.rs  # NotifySendNotifier
│   │   └── config/
│   │       ├── mod.rs
│   │       └── xdg.rs          # XdgConfigStore
│   │
│   └── cli/                    # CLI interface
│       ├── mod.rs
│       ├── args.rs             # Clap argument definitions
│       ├── app.rs              # Main application runner
│       ├── daemon_app.rs       # Daemon mode runner
│       ├── config_cmd.rs       # Config subcommand handler
│       ├── presenter.rs        # Output formatting
│       ├── signals.rs          # Signal handlers
│       └── pid_file.rs         # PID file management
│
├── tests/                      # Integration tests
│   ├── cli_tests.rs
│   ├── recording_tests.rs
│   └── fixtures/
│       └── test_audio.ogg
│
└── docs/
    ├── REQUIREMENTS.md
    ├── GEMINI_RUST_INTEGRATION.md
    └── RUST_ARCHITECTURE.md
```

---

## 4. Domain Layer

### 4.1 Duration Value Object

```rust
// src/domain/recording/duration.rs

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid duration format '{0}'. Expected format: 30s, 1m, 2m30s")]
pub struct DurationParseError(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    milliseconds: u64,
}

impl Duration {
    pub const fn from_seconds(seconds: u64) -> Self {
        Self {
            milliseconds: seconds * 1000,
        }
    }

    pub const fn from_milliseconds(milliseconds: u64) -> Self {
        Self { milliseconds }
    }

    pub const fn as_seconds(&self) -> u64 {
        self.milliseconds / 1000
    }

    pub const fn as_milliseconds(&self) -> u64 {
        self.milliseconds
    }

    pub fn as_std(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.milliseconds)
    }
}

impl FromStr for Duration {
    type Err = DurationParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        // Regex-free parsing: "2m30s", "30s", "1m"
        let mut minutes = 0u64;
        let mut seconds = 0u64;
        let mut current_num = String::new();

        for c in s.chars() {
            match c {
                '0'..='9' => current_num.push(c),
                'm' => {
                    minutes = current_num
                        .parse()
                        .map_err(|_| DurationParseError(s.clone()))?;
                    current_num.clear();
                }
                's' => {
                    seconds = current_num
                        .parse()
                        .map_err(|_| DurationParseError(s.clone()))?;
                    current_num.clear();
                }
                _ => return Err(DurationParseError(s)),
            }
        }

        let total_seconds = minutes * 60 + seconds;
        if total_seconds == 0 {
            return Err(DurationParseError(s));
        }

        Ok(Self::from_seconds(total_seconds))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_secs = self.as_seconds();
        let mins = total_secs / 60;
        let secs = total_secs % 60;

        match (mins, secs) {
            (0, s) => write!(f, "{}s", s),
            (m, 0) => write!(f, "{}m", m),
            (m, s) => write!(f, "{}m{}s", m, s),
        }
    }
}

// Default durations as constants
impl Duration {
    pub const DEFAULT: Self = Self::from_seconds(10);
    pub const DEFAULT_MAX: Self = Self::from_seconds(60);
}
```

### 4.2 Domain Preset

```rust
// src/domain/transcription/domain_preset.rs

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Unknown domain '{0}'. Valid options: general, dev, medical, legal, finance")]
pub struct InvalidDomainError(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainId {
    #[default]
    General,
    Dev,
    Medical,
    Legal,
    Finance,
}

impl DomainId {
    pub const ALL: &'static [DomainId] = &[
        DomainId::General,
        DomainId::Dev,
        DomainId::Medical,
        DomainId::Legal,
        DomainId::Finance,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "General Conversation",
            Self::Dev => "Software Engineering",
            Self::Medical => "Medical / Healthcare",
            Self::Legal => "Legal",
            Self::Finance => "Finance",
        }
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            Self::General => "Standard grammar correction and clarity.",
            Self::Dev => "Focus on programming terminology, variable naming conventions, \
                          camelCase, snake_case, technical jargon, API names, and code-related terms.",
            Self::Medical => "Ensure accurate spelling of medical conditions, medications, \
                              procedures, anatomy terms, and healthcare terminology.",
            Self::Legal => "Maintain formal tone, ensure accurate legal terminology, \
                            case citations, and proper legal document formatting.",
            Self::Finance => "Focus on financial markets, acronyms (ETF, ROI, CAGR), \
                              currency formatting, and precise numerical transcription.",
        }
    }
}

impl FromStr for DomainId {
    type Err = InvalidDomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "general" => Ok(Self::General),
            "dev" => Ok(Self::Dev),
            "medical" => Ok(Self::Medical),
            "legal" => Ok(Self::Legal),
            "finance" => Ok(Self::Finance),
            _ => Err(InvalidDomainError(s.to_string())),
        }
    }
}

impl fmt::Display for DomainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::General => "general",
            Self::Dev => "dev",
            Self::Medical => "medical",
            Self::Legal => "legal",
            Self::Finance => "finance",
        };
        write!(f, "{}", s)
    }
}
```

### 4.3 System Prompt

```rust
// src/domain/transcription/system_prompt.rs

use super::domain_preset::DomainId;

const BASE_INSTRUCTION: &str = r#"You are a voice-to-text assistant that transcribes audio into grammatically correct, context-aware text output.

Instructions:
- Remove filler words (um, ah, like, you know)
- Must have correct grammar and punctuation
- Do NOT transcribe stutters, false starts, or repeated words
- Output ONLY the final cleaned text
- Do NOT include meta-commentary or explanations"#;

#[derive(Debug, Clone)]
pub struct SystemPrompt {
    content: String,
}

impl SystemPrompt {
    pub fn build(domain: DomainId) -> Self {
        let content = format!(
            "{}\n\nDomain Context: {}\n{}",
            BASE_INSTRUCTION,
            domain.label(),
            domain.prompt()
        );
        Self { content }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Default for SystemPrompt {
    fn default() -> Self {
        Self::build(DomainId::default())
    }
}
```

### 4.4 Audio Data

```rust
// src/domain/transcription/audio_data.rs

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioMimeType {
    Ogg,
    Mp3,
    Mpeg,
    Wav,
    Flac,
    Aac,
}

impl AudioMimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ogg => "audio/ogg",
            Self::Mp3 => "audio/mp3",
            Self::Mpeg => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Flac => "audio/flac",
            Self::Aac => "audio/aac",
        }
    }
}

impl fmt::Display for AudioMimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct AudioData {
    data: Vec<u8>,
    mime_type: AudioMimeType,
}

impl AudioData {
    pub fn new(data: Vec<u8>, mime_type: AudioMimeType) -> Self {
        Self { data, mime_type }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn mime_type(&self) -> AudioMimeType {
        self.mime_type
    }

    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn human_readable_size(&self) -> String {
        let bytes = self.size_bytes();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}
```

### 4.5 Daemon Session State Machine

```rust
// src/domain/daemon/session.rs

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    Idle,
    Recording,
    Processing,
}

impl std::fmt::Display for DaemonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "IDLE"),
            Self::Recording => write!(f, "RECORDING"),
            Self::Processing => write!(f, "PROCESSING"),
        }
    }
}

#[derive(Debug, Error)]
#[error("Cannot {action} while in {state} state")]
pub struct InvalidStateTransition {
    pub state: DaemonState,
    pub action: &'static str,
}

/// Type-state pattern for compile-time state machine verification
pub struct DaemonSession<S> {
    _state: std::marker::PhantomData<S>,
}

// State markers
pub struct Idle;
pub struct Recording;
pub struct Processing;

impl DaemonSession<Idle> {
    pub fn new() -> Self {
        Self {
            _state: std::marker::PhantomData,
        }
    }

    pub fn start_recording(self) -> DaemonSession<Recording> {
        DaemonSession {
            _state: std::marker::PhantomData,
        }
    }
}

impl DaemonSession<Recording> {
    pub fn stop_recording(self) -> DaemonSession<Processing> {
        DaemonSession {
            _state: std::marker::PhantomData,
        }
    }

    pub fn cancel(self) -> DaemonSession<Idle> {
        DaemonSession {
            _state: std::marker::PhantomData,
        }
    }
}

impl DaemonSession<Processing> {
    pub fn complete(self) -> DaemonSession<Idle> {
        DaemonSession {
            _state: std::marker::PhantomData,
        }
    }
}

impl Default for DaemonSession<Idle> {
    fn default() -> Self {
        Self::new()
    }
}

// Runtime state machine (for dynamic state management)
#[derive(Debug)]
pub struct RuntimeDaemonSession {
    state: DaemonState,
}

impl RuntimeDaemonSession {
    pub fn new() -> Self {
        Self {
            state: DaemonState::Idle,
        }
    }

    pub fn state(&self) -> DaemonState {
        self.state
    }

    pub fn is_idle(&self) -> bool {
        self.state == DaemonState::Idle
    }

    pub fn is_recording(&self) -> bool {
        self.state == DaemonState::Recording
    }

    pub fn is_processing(&self) -> bool {
        self.state == DaemonState::Processing
    }

    pub fn start_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Idle {
            return Err(InvalidStateTransition {
                state: self.state,
                action: "start recording",
            });
        }
        self.state = DaemonState::Recording;
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Recording {
            return Err(InvalidStateTransition {
                state: self.state,
                action: "stop recording",
            });
        }
        self.state = DaemonState::Processing;
        Ok(())
    }

    pub fn cancel_recording(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Recording {
            return Err(InvalidStateTransition {
                state: self.state,
                action: "cancel recording",
            });
        }
        self.state = DaemonState::Idle;
        Ok(())
    }

    pub fn complete_processing(&mut self) -> Result<(), InvalidStateTransition> {
        if self.state != DaemonState::Processing {
            return Err(InvalidStateTransition {
                state: self.state,
                action: "complete processing",
            });
        }
        self.state = DaemonState::Idle;
        Ok(())
    }
}

impl Default for RuntimeDaemonSession {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4.6 App Config

```rust
// src/domain/config/app_config.rs

use crate::domain::recording::Duration;
use crate::domain::transcription::DomainId;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub api_key: Option<String>,
    pub duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub domain: Option<DomainId>,
    pub clipboard: Option<bool>,
    pub keystroke: Option<bool>,
    pub notify: Option<bool>,
}

impl AppConfig {
    pub fn defaults() -> Self {
        Self {
            api_key: None,
            duration: Some(Duration::DEFAULT),
            max_duration: Some(Duration::DEFAULT_MAX),
            domain: Some(DomainId::default()),
            clipboard: Some(false),
            keystroke: Some(false),
            notify: Some(false),
        }
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(self, other: Self) -> Self {
        Self {
            api_key: other.api_key.or(self.api_key),
            duration: other.duration.or(self.duration),
            max_duration: other.max_duration.or(self.max_duration),
            domain: other.domain.or(self.domain),
            clipboard: other.clipboard.or(self.clipboard),
            keystroke: other.keystroke.or(self.keystroke),
            notify: other.notify.or(self.notify),
        }
    }

    /// Get duration with fallback to default
    pub fn duration_or_default(&self) -> Duration {
        self.duration.unwrap_or(Duration::DEFAULT)
    }

    /// Get max_duration with fallback to default
    pub fn max_duration_or_default(&self) -> Duration {
        self.max_duration.unwrap_or(Duration::DEFAULT_MAX)
    }

    /// Get domain with fallback to default
    pub fn domain_or_default(&self) -> DomainId {
        self.domain.unwrap_or_default()
    }
}
```

---

## 5. Application Layer

### 5.1 Port Traits

```rust
// src/application/ports/recorder.rs

use crate::domain::{AudioData, Duration};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("Failed to start recording: {0}")]
    StartFailed(String),

    #[error("Recording interrupted: {0}")]
    Interrupted(String),

    #[error("Failed to read audio file: {0}")]
    ReadFailed(String),

    #[error("FFmpeg not found. Install with: sudo pacman -S ffmpeg")]
    FfmpegNotFound,

    #[error("Already recording")]
    AlreadyRecording,
}

pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Port for bounded (fixed duration) recording
#[async_trait]
pub trait AudioRecorder: Send + Sync {
    async fn record(
        &self,
        duration: Duration,
        on_progress: Option<ProgressCallback>,
    ) -> Result<AudioData, RecordingError>;

    async fn stop(&self) -> Result<Option<AudioData>, RecordingError>;
}

/// Port for unbounded (daemon) recording
#[async_trait]
pub trait UnboundedRecorder: Send + Sync {
    fn start_recording(
        &self,
        max_duration: Duration,
        on_progress: Option<Box<dyn Fn(u64) + Send + Sync>>,
        on_max_reached: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Result<(), RecordingError>;

    async fn stop_and_finalize(&self) -> Result<AudioData, RecordingError>;

    async fn cancel(&self) -> Result<(), RecordingError>;

    fn is_recording(&self) -> bool;
}
```

```rust
// src/application/ports/transcriber.rs

use crate::domain::{AudioData, SystemPrompt};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Empty response from API")]
    EmptyResponse,

    #[error("Content blocked by safety filters")]
    ContentBlocked,
}

#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcribe(
        &self,
        audio: &AudioData,
        prompt: &SystemPrompt,
    ) -> Result<String, TranscriptionError>;
}
```

```rust
// src/application/ports/clipboard.rs

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("Clipboard operation failed: {0}")]
    Failed(String),

    #[error("wl-copy not found. Install with: sudo pacman -S wl-clipboard")]
    NotFound,
}

#[async_trait]
pub trait Clipboard: Send + Sync {
    async fn copy(&self, text: &str) -> Result<(), ClipboardError>;
}
```

```rust
// src/application/ports/keystroke.rs

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeystrokeError {
    #[error("Keystroke injection failed: {0}")]
    Failed(String),

    #[error("xdotool not found. Install with: sudo pacman -S xdotool")]
    NotFound,
}

#[async_trait]
pub trait Keystroke: Send + Sync {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError>;
}
```

```rust
// src/application/ports/notifier.rs

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Notification failed: {0}")]
    Failed(String),

    #[error("notify-send not found. Install with: sudo pacman -S libnotify")]
    NotFound,
}

pub enum NotificationIcon {
    Microphone,
    Processing,
    Clipboard,
    Keyboard,
    Success,
    Error,
    Info,
    Warning,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(
        &self,
        title: &str,
        message: &str,
        icon: Option<NotificationIcon>,
    ) -> Result<(), NotificationError>;
}
```

### 5.2 Transcribe Use Case

```rust
// src/application/transcribe.rs

use crate::application::ports::*;
use crate::domain::*;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranscribeError {
    #[error("Recording failed: {0}")]
    Recording(#[from] RecordingError),

    #[error("Transcription failed: {0}")]
    Transcription(#[from] TranscriptionError),
}

pub struct TranscribeRecordingUseCase {
    recorder: Arc<dyn AudioRecorder>,
    transcriber: Arc<dyn Transcriber>,
    clipboard: Option<Arc<dyn Clipboard>>,
    keystroke: Option<Arc<dyn Keystroke>>,
}

pub struct TranscribeInput {
    pub duration: Duration,
    pub domain: DomainId,
    pub enable_clipboard: bool,
    pub enable_keystroke: bool,
}

pub struct TranscribeOutput {
    pub text: String,
    pub clipboard_copied: bool,
    pub keystroke_sent: bool,
}

pub struct TranscribeCallbacks {
    pub on_recording_start: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_recording_progress: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    pub on_recording_complete: Option<Box<dyn Fn(&str) + Send + Sync>>,
    pub on_transcription_start: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_transcription_complete: Option<Box<dyn Fn(&str) + Send + Sync>>,
    pub on_clipboard_copy: Option<Box<dyn Fn(bool) + Send + Sync>>,
    pub on_keystroke_send: Option<Box<dyn Fn(bool) + Send + Sync>>,
}

impl TranscribeRecordingUseCase {
    pub fn new(
        recorder: Arc<dyn AudioRecorder>,
        transcriber: Arc<dyn Transcriber>,
        clipboard: Option<Arc<dyn Clipboard>>,
        keystroke: Option<Arc<dyn Keystroke>>,
    ) -> Self {
        Self {
            recorder,
            transcriber,
            clipboard,
            keystroke,
        }
    }

    pub async fn execute(
        &self,
        input: TranscribeInput,
        callbacks: TranscribeCallbacks,
    ) -> Result<TranscribeOutput, TranscribeError> {
        // 1. Start recording
        if let Some(cb) = &callbacks.on_recording_start {
            cb();
        }

        let audio = self
            .recorder
            .record(input.duration, callbacks.on_recording_progress.map(|cb| {
                Box::new(move |elapsed, total| cb(elapsed, total)) as ProgressCallback
            }))
            .await?;

        if let Some(cb) = &callbacks.on_recording_complete {
            cb(&audio.human_readable_size());
        }

        // 2. Transcribe
        if let Some(cb) = &callbacks.on_transcription_start {
            cb();
        }

        let prompt = SystemPrompt::build(input.domain);
        let text = self.transcriber.transcribe(&audio, &prompt).await?;

        if let Some(cb) = &callbacks.on_transcription_complete {
            cb(&text);
        }

        // 3. Output actions (non-fatal)
        let mut clipboard_copied = false;
        let mut keystroke_sent = false;

        if input.enable_clipboard {
            if let Some(clipboard) = &self.clipboard {
                clipboard_copied = clipboard.copy(&text).await.is_ok();
            }
            if let Some(cb) = &callbacks.on_clipboard_copy {
                cb(clipboard_copied);
            }
        }

        if input.enable_keystroke {
            if let Some(keystroke) = &self.keystroke {
                keystroke_sent = keystroke.type_text(&text).await.is_ok();
            }
            if let Some(cb) = &callbacks.on_keystroke_send {
                cb(keystroke_sent);
            }
        }

        Ok(TranscribeOutput {
            text,
            clipboard_copied,
            keystroke_sent,
        })
    }

    pub async fn stop_early(&self) -> Result<Option<AudioData>, RecordingError> {
        self.recorder.stop().await
    }
}
```

---

## 6. Infrastructure Layer

### 6.1 FFmpeg Recorder

```rust
// src/infrastructure/recording/ffmpeg.rs

use crate::application::ports::{AudioRecorder, RecordingError, UnboundedRecorder};
use crate::domain::{AudioData, AudioMimeType, Duration};
use async_trait::async_trait;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration as TokioDuration};

pub struct FfmpegRecorder {
    process: Arc<Mutex<Option<Child>>>,
    output_path: Arc<Mutex<Option<String>>>,
    should_stop: Arc<AtomicBool>,
}

impl FfmpegRecorder {
    pub fn new() -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            output_path: Arc::new(Mutex::new(None)),
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    fn generate_output_path() -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("/tmp/smartscribe-{}.ogg", timestamp)
    }

    async fn spawn_ffmpeg(&self, duration_flag: Option<&str>) -> Result<Child, RecordingError> {
        let output_path = Self::generate_output_path();
        *self.output_path.lock().await = Some(output_path.clone());

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-f", "pulse", "-i", "default"]);

        if let Some(duration) = duration_flag {
            cmd.args(["-t", duration]);
        }

        cmd.args([
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "libopus",
            "-b:a", "16k",
            "-application", "voip",
            "-y",
            &output_path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

        cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RecordingError::FfmpegNotFound
            } else {
                RecordingError::StartFailed(e.to_string())
            }
        })
    }

    async fn read_audio_file(&self) -> Result<AudioData, RecordingError> {
        let path = self.output_path.lock().await;
        let path = path.as_ref().ok_or_else(|| {
            RecordingError::ReadFailed("No output path set".to_string())
        })?;

        let data = tokio::fs::read(path)
            .await
            .map_err(|e| RecordingError::ReadFailed(e.to_string()))?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(path).await;

        Ok(AudioData::new(data, AudioMimeType::Ogg))
    }
}

#[async_trait]
impl AudioRecorder for FfmpegRecorder {
    async fn record(
        &self,
        duration: Duration,
        on_progress: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> Result<AudioData, RecordingError> {
        self.should_stop.store(false, Ordering::SeqCst);

        let duration_str = duration.as_seconds().to_string();
        let mut child = self.spawn_ffmpeg(Some(&duration_str)).await?;
        *self.process.lock().await = Some(child);

        // Progress reporting
        if let Some(callback) = on_progress {
            let total = duration.as_seconds();
            let should_stop = self.should_stop.clone();
            let mut ticker = interval(TokioDuration::from_millis(100));
            let start = std::time::Instant::now();

            tokio::spawn(async move {
                loop {
                    ticker.tick().await;
                    if should_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let elapsed = start.elapsed().as_secs();
                    callback(elapsed.min(total), total);
                    if elapsed >= total {
                        break;
                    }
                }
            });
        }

        // Wait for completion
        let mut process = self.process.lock().await;
        if let Some(ref mut child) = *process {
            let status = child.wait().await.map_err(|e| {
                RecordingError::Interrupted(e.to_string())
            })?;

            self.should_stop.store(true, Ordering::SeqCst);

            if !status.success() && !self.should_stop.load(Ordering::SeqCst) {
                return Err(RecordingError::Interrupted(
                    format!("FFmpeg exited with code: {:?}", status.code())
                ));
            }
        }
        *process = None;

        self.read_audio_file().await
    }

    async fn stop(&self) -> Result<Option<AudioData>, RecordingError> {
        self.should_stop.store(true, Ordering::SeqCst);

        let mut process = self.process.lock().await;
        if let Some(ref mut child) = *process {
            // Send SIGINT for graceful stop
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                if let Some(pid) = child.id() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGINT);
                }
            }

            let _ = child.wait().await;
        }
        *process = None;

        // Try to read whatever was recorded
        match self.read_audio_file().await {
            Ok(audio) => Ok(Some(audio)),
            Err(_) => Ok(None),
        }
    }
}

impl Default for FfmpegRecorder {
    fn default() -> Self {
        Self::new()
    }
}
```

### 6.2 Gemini Transcriber

```rust
// src/infrastructure/transcription/gemini.rs

use crate::application::ports::{Transcriber, TranscriptionError};
use crate::domain::{AudioData, SystemPrompt};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const MODEL: &str = "gemini-2.0-flash-lite";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_data: Option<InlineData>,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkingConfig {
    thinking_budget: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    thinking_config: ThinkingConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    contents: Vec<Content>,
    system_instruction: SystemInstruction,
    generation_config: GenerationConfig,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
    code: Option<i32>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

pub struct GeminiTranscriber {
    client: Client,
    api_key: String,
}

impl GeminiTranscriber {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
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
        let base64_audio = STANDARD.encode(audio.data());

        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: None,
                    inline_data: Some(InlineData {
                        mime_type: audio.mime_type().to_string(),
                        data: base64_audio,
                    }),
                }],
            }],
            system_instruction: SystemInstruction {
                parts: vec![Part {
                    text: Some(prompt.content().to_string()),
                    inline_data: None,
                }],
            },
            generation_config: GenerationConfig {
                thinking_config: ThinkingConfig { thinking_budget: 0 },
            },
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            API_BASE, MODEL, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            if let Ok(error) = response.json::<ErrorResponse>().await {
                let code = error.error.code.unwrap_or(status.as_u16() as i32);
                let message = error.error.message.unwrap_or_default();

                return Err(match code {
                    401 | 403 => TranscriptionError::InvalidApiKey,
                    429 => TranscriptionError::RateLimitExceeded,
                    _ => TranscriptionError::RequestFailed(message),
                });
            }
            return Err(TranscriptionError::RequestFailed(status.to_string()));
        }

        let response: GenerateResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::RequestFailed(e.to_string()))?;

        let candidates = response.candidates.ok_or(TranscriptionError::EmptyResponse)?;
        let candidate = candidates.first().ok_or(TranscriptionError::EmptyResponse)?;

        let text: String = candidate
            .content
            .parts
            .iter()
            .filter_map(|p| p.text.as_ref())
            .cloned()
            .collect();

        if text.is_empty() {
            return Err(TranscriptionError::EmptyResponse);
        }

        Ok(text.trim().to_string())
    }
}
```

---

## 7. CLI Layer

### 7.1 Argument Definitions

```rust
// src/cli/args.rs

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "smart-scribe")]
#[command(author, version, about = "AI-powered audio transcription")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub transcribe: TranscribeArgs,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Create default config file
    Init,
    /// Set a config value
    Set {
        key: String,
        value: String,
    },
    /// Get a config value
    Get {
        key: String,
    },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

#[derive(Args)]
pub struct TranscribeArgs {
    /// Recording duration (e.g., 10s, 1m, 2m30s)
    #[arg(short, long, value_name = "TIME")]
    pub duration: Option<String>,

    /// Domain preset for transcription context
    #[arg(short = 'D', long, value_enum)]
    pub domain: Option<DomainArg>,

    /// Copy result to clipboard
    #[arg(short, long)]
    pub clipboard: bool,

    /// Type result into focused window
    #[arg(short, long)]
    pub keystroke: bool,

    /// Show desktop notifications
    #[arg(short, long)]
    pub notify: bool,

    /// Run as daemon (signal-controlled)
    #[arg(long, conflicts_with = "duration")]
    pub daemon: bool,

    /// Max recording duration for daemon mode
    #[arg(long, value_name = "TIME")]
    pub max_duration: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum DomainArg {
    General,
    Dev,
    Medical,
    Legal,
    Finance,
}

impl From<DomainArg> for crate::domain::DomainId {
    fn from(arg: DomainArg) -> Self {
        match arg {
            DomainArg::General => Self::General,
            DomainArg::Dev => Self::Dev,
            DomainArg::Medical => Self::Medical,
            DomainArg::Legal => Self::Legal,
            DomainArg::Finance => Self::Finance,
        }
    }
}
```

### 7.2 Main Entry Point

```rust
// src/main.rs

use clap::Parser;
use smart_scribe::cli::{App, Cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let app = App::new();
    let exit_code = app.run(cli).await;

    std::process::exit(exit_code);
}
```

### 7.3 Presenter

```rust
// src/cli/presenter.rs

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

pub struct Presenter {
    spinner: Option<ProgressBar>,
}

impl Presenter {
    pub fn new() -> Self {
        Self { spinner: None }
    }

    pub fn start_spinner(&mut self, message: &str) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        self.spinner = Some(pb);
    }

    pub fn update_spinner(&mut self, message: &str) {
        if let Some(ref pb) = self.spinner {
            pb.set_message(message.to_string());
        }
    }

    pub fn spinner_success(&mut self, message: &str) {
        if let Some(pb) = self.spinner.take() {
            pb.finish_with_message(format!("{} {}", "✓".green(), message));
        }
    }

    pub fn spinner_fail(&mut self, message: &str) {
        if let Some(pb) = self.spinner.take() {
            pb.finish_with_message(format!("{} {}", "✗".red(), message));
        }
    }

    pub fn stop_spinner(&mut self) {
        if let Some(pb) = self.spinner.take() {
            pb.finish_and_clear();
        }
    }

    pub fn info(&self, message: &str) {
        eprintln!("{} {}", "ℹ".blue(), message);
    }

    pub fn success(&self, message: &str) {
        eprintln!("{} {}", "✓".green(), message);
    }

    pub fn warn(&self, message: &str) {
        eprintln!("{} {}", "⚠".yellow(), message);
    }

    pub fn error(&self, message: &str) {
        eprintln!("{} {}", "✗".red(), message);
    }

    pub fn output(&self, text: &str) {
        println!("{}", text);
    }

    pub fn format_progress(&self, elapsed: u64, total: u64) -> String {
        let progress = (elapsed as f64 / total as f64).min(1.0);
        let filled = (progress * 20.0) as usize;
        let empty = 20 - filled;

        let bar = format!(
            "{}{}",
            "█".repeat(filled).green(),
            "░".repeat(empty).dimmed()
        );

        let remaining = total.saturating_sub(elapsed);
        format!(
            "Recording: [{}] {:.1}s remaining",
            bar,
            remaining as f64
        )
    }

    pub fn format_duration(seconds: u64) -> String {
        let mins = seconds / 60;
        let secs = seconds % 60;
        match (mins, secs) {
            (0, s) => format!("{}s", s),
            (m, 0) => format!("{}m", m),
            (m, s) => format!("{}m{}s", m, s),
        }
    }
}

impl Default for Presenter {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 8. Error Handling Strategy

### 8.1 Error Hierarchy

```rust
// src/domain/error.rs

use thiserror::Error;

/// Top-level application error
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Recording error: {0}")]
    Recording(#[from] crate::application::ports::RecordingError),

    #[error("Transcription error: {0}")]
    Transcription(#[from] crate::application::ports::TranscriptionError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Missing API key. Set GEMINI_API_KEY or use 'smart-scribe config set api_key <KEY>'")]
    MissingApiKey,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error("Failed to write config: {0}")]
    WriteError(String),

    #[error("Invalid config key: {0}")]
    InvalidKey(String),

    #[error("Invalid value for {key}: {message}")]
    ValidationError { key: String, message: String },
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Recording(_) | Self::Transcription(_) => 1,
            Self::Config(_) | Self::MissingApiKey => 1,
        }
    }
}
```

### 8.2 Result Type Alias

```rust
// src/lib.rs

pub type Result<T> = std::result::Result<T, AppError>;
```

---

## 9. Async Architecture

### 9.1 Tokio Runtime Configuration

```rust
// src/main.rs

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // ...
}
```

### 9.2 Cancellation Pattern

```rust
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub struct CancellableTask {
    cancel_token: CancellationToken,
}

impl CancellableTask {
    pub fn new() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
        }
    }

    pub async fn run<F, T>(&self, task: F) -> Option<T>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::select! {
            result = task => Some(result),
            _ = self.cancel_token.cancelled() => None,
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}
```

---

## 10. Configuration System

### 10.1 Config Loading

```rust
// src/infrastructure/config/xdg.rs

use crate::domain::{AppConfig, ConfigError, DomainId, Duration};
use std::path::PathBuf;

pub struct XdgConfigStore {
    path: PathBuf,
}

impl XdgConfigStore {
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("smart-scribe")
            .join("config.toml");

        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub async fn load(&self) -> Result<AppConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }

        let content = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::NotFound(e.to_string()))?;

        self.parse_toml(&content)
    }

    fn parse_toml(&self, content: &str) -> Result<AppConfig, ConfigError> {
        let table: toml::Table = content
            .parse()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(AppConfig {
            api_key: table.get("api_key").and_then(|v| v.as_str()).map(String::from),
            duration: table
                .get("duration")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            max_duration: table
                .get("max_duration")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            domain: table
                .get("domain")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            clipboard: table.get("clipboard").and_then(|v| v.as_bool()),
            keystroke: table.get("keystroke").and_then(|v| v.as_bool()),
            notify: table.get("notify").and_then(|v| v.as_bool()),
        })
    }

    pub async fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        // Ensure directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ConfigError::WriteError(e.to_string()))?;
        }

        let mut table = toml::Table::new();

        if let Some(ref key) = config.api_key {
            table.insert("api_key".to_string(), toml::Value::String(key.clone()));
        }
        if let Some(duration) = config.duration {
            table.insert("duration".to_string(), toml::Value::String(duration.to_string()));
        }
        if let Some(max_duration) = config.max_duration {
            table.insert("max_duration".to_string(), toml::Value::String(max_duration.to_string()));
        }
        if let Some(domain) = config.domain {
            table.insert("domain".to_string(), toml::Value::String(domain.to_string()));
        }
        if let Some(clipboard) = config.clipboard {
            table.insert("clipboard".to_string(), toml::Value::Boolean(clipboard));
        }
        if let Some(keystroke) = config.keystroke {
            table.insert("keystroke".to_string(), toml::Value::Boolean(keystroke));
        }
        if let Some(notify) = config.notify {
            table.insert("notify".to_string(), toml::Value::Boolean(notify));
        }

        let content = toml::to_string_pretty(&table)
            .map_err(|e| ConfigError::WriteError(e.to_string()))?;

        tokio::fs::write(&self.path, content)
            .await
            .map_err(|e| ConfigError::WriteError(e.to_string()))
    }
}
```

### 10.2 Config Service

```rust
// src/infrastructure/config/service.rs

use crate::domain::AppConfig;
use super::xdg::XdgConfigStore;

pub struct ConfigService {
    store: XdgConfigStore,
}

impl ConfigService {
    pub fn new() -> Self {
        Self {
            store: XdgConfigStore::new(),
        }
    }

    pub async fn load_merged(&self) -> AppConfig {
        // 1. Start with defaults
        let mut config = AppConfig::defaults();

        // 2. Merge file config
        if let Ok(file_config) = self.store.load().await {
            config = config.merge(file_config);
        }

        // 3. Merge environment
        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            config.api_key = Some(api_key);
        }

        config
    }
}
```

---

## 11. Signal Handling

### 11.1 One-Shot Signal Handler

```rust
// src/cli/signals.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};

pub struct SignalHandler {
    shutdown: Arc<AtomicBool>,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    pub async fn wait_for_shutdown(&self) {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }

        self.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn trigger_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}
```

### 11.2 Daemon Signal Handler

```rust
// src/cli/daemon_signals.rs

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum DaemonSignal {
    Toggle,   // SIGUSR1
    Cancel,   // SIGUSR2
    Shutdown, // SIGINT/SIGTERM
}

pub struct DaemonSignalHandler {
    rx: mpsc::Receiver<DaemonSignal>,
}

impl DaemonSignalHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);

        // SIGUSR1 - Toggle
        let tx1 = tx.clone();
        tokio::spawn(async move {
            let mut sig = signal(SignalKind::user_defined1()).unwrap();
            loop {
                sig.recv().await;
                let _ = tx1.send(DaemonSignal::Toggle).await;
            }
        });

        // SIGUSR2 - Cancel
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let mut sig = signal(SignalKind::user_defined2()).unwrap();
            loop {
                sig.recv().await;
                let _ = tx2.send(DaemonSignal::Cancel).await;
            }
        });

        // SIGINT - Shutdown
        let tx3 = tx.clone();
        tokio::spawn(async move {
            let mut sig = signal(SignalKind::interrupt()).unwrap();
            sig.recv().await;
            let _ = tx3.send(DaemonSignal::Shutdown).await;
        });

        // SIGTERM - Shutdown
        let tx4 = tx;
        tokio::spawn(async move {
            let mut sig = signal(SignalKind::terminate()).unwrap();
            sig.recv().await;
            let _ = tx4.send(DaemonSignal::Shutdown).await;
        });

        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<DaemonSignal> {
        self.rx.recv().await
    }
}
```

---

## 12. Testing Strategy

### 12.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_parse() {
        assert_eq!(Duration::from_str("30s").unwrap().as_seconds(), 30);
        assert_eq!(Duration::from_str("1m").unwrap().as_seconds(), 60);
        assert_eq!(Duration::from_str("2m30s").unwrap().as_seconds(), 150);
        assert!(Duration::from_str("invalid").is_err());
        assert!(Duration::from_str("0s").is_err());
    }

    #[test]
    fn test_duration_display() {
        assert_eq!(Duration::from_seconds(30).to_string(), "30s");
        assert_eq!(Duration::from_seconds(60).to_string(), "1m");
        assert_eq!(Duration::from_seconds(90).to_string(), "1m30s");
    }

    #[test]
    fn test_domain_preset() {
        assert_eq!(DomainId::from_str("dev").unwrap(), DomainId::Dev);
        assert!(DomainId::from_str("unknown").is_err());
    }

    #[test]
    fn test_config_merge() {
        let base = AppConfig::defaults();
        let override_config = AppConfig {
            domain: Some(DomainId::Dev),
            clipboard: Some(true),
            ..Default::default()
        };

        let merged = base.merge(override_config);
        assert_eq!(merged.domain, Some(DomainId::Dev));
        assert_eq!(merged.clipboard, Some(true));
        assert_eq!(merged.duration, Some(Duration::DEFAULT));
    }
}
```

### 12.2 Integration Tests

```rust
// tests/cli_tests.rs

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    Command::cargo_bin("smart-scribe")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI-powered audio transcription"));
}

#[test]
fn test_version() {
    Command::cargo_bin("smart-scribe")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn test_config_path() {
    Command::cargo_bin("smart-scribe")
        .unwrap()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_invalid_duration() {
    Command::cargo_bin("smart-scribe")
        .unwrap()
        .args(["-d", "invalid"])
        .assert()
        .failure();
}

#[test]
fn test_daemon_duration_conflict() {
    Command::cargo_bin("smart-scribe")
        .unwrap()
        .args(["--daemon", "-d", "30s"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
```

### 12.3 Mock Adapters

```rust
// src/infrastructure/mock.rs (for testing)

use crate::application::ports::*;
use crate::domain::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct MockRecorder {
    pub recorded_duration: Arc<Mutex<Option<Duration>>>,
    pub response: Result<AudioData, RecordingError>,
}

#[async_trait]
impl AudioRecorder for MockRecorder {
    async fn record(
        &self,
        duration: Duration,
        _on_progress: Option<ProgressCallback>,
    ) -> Result<AudioData, RecordingError> {
        *self.recorded_duration.lock().unwrap() = Some(duration);
        self.response.clone()
    }

    async fn stop(&self) -> Result<Option<AudioData>, RecordingError> {
        Ok(None)
    }
}

pub struct MockTranscriber {
    pub response: Result<String, TranscriptionError>,
}

#[async_trait]
impl Transcriber for MockTranscriber {
    async fn transcribe(
        &self,
        _audio: &AudioData,
        _prompt: &SystemPrompt,
    ) -> Result<String, TranscriptionError> {
        self.response.clone()
    }
}
```

---

## 13. Build and Distribution

### 13.1 Release Build

```bash
# Optimized release build
cargo build --release

# Binary location
./target/release/smart-scribe
```

### 13.2 Cross-Compilation

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "x86_64-linux-gnu-gcc"

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

### 13.3 GitHub Actions

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: smart-scribe-linux-x86_64
          path: target/release/smart-scribe
```

---

## 14. Migration Checklist

### Phase 1: Core Domain
- [ ] Duration value object with parsing
- [ ] DomainId enum with presets
- [ ] SystemPrompt builder
- [ ] AudioData value object
- [ ] AppConfig with merge
- [ ] DaemonSession state machine
- [ ] Domain error types

### Phase 2: Application Layer
- [ ] Port traits (AudioRecorder, Transcriber, etc.)
- [ ] TranscribeRecordingUseCase
- [ ] DaemonTranscriptionUseCase

### Phase 3: Infrastructure
- [ ] FFmpegRecorder adapter
- [ ] GeminiTranscriber adapter
- [ ] WaylandClipboard adapter
- [ ] XdotoolKeystroke adapter
- [ ] NotifySendNotifier adapter
- [ ] XdgConfigStore adapter

### Phase 4: CLI
- [ ] Clap argument definitions
- [ ] Main app runner
- [ ] Daemon app runner
- [ ] Config command handler
- [ ] Presenter (output formatting)
- [ ] Signal handlers
- [ ] PID file management

### Phase 5: Testing
- [ ] Unit tests for domain
- [ ] Integration tests for CLI
- [ ] Mock adapters for testing

### Phase 6: Polish
- [ ] Error messages refinement
- [ ] Documentation (README)
- [ ] Release workflow
- [ ] Performance optimization

---

## Appendix: Quick Reference

### Key Dependencies Summary

```toml
tokio = { version = "1", features = ["full", "signal"] }
reqwest = { version = "0.12", features = ["json"] }
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
base64 = "0.22"
thiserror = "2"
colored = "2"
indicatif = "0.17"
dirs = "5"
dotenvy = "0.15"
```

### Module Re-exports (lib.rs)

```rust
pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod cli;

pub use domain::*;
pub use application::*;
```
