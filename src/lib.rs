//! SmartScribe - AI-powered audio transcription CLI
//!
//! This crate records audio from the microphone and transcribes it using OpenAI:
//! either a ChatGPT subscription via OAuth (`auth = "oauth"`) or the OpenAI
//! `/v1/audio/transcriptions` REST API with an API key (`auth = "api_key"`).
//!
//! # Architecture
//!
//! The crate follows hexagonal (ports & adapters) architecture:
//!
//! - **Domain**: Core business logic, value objects, entities, and errors
//! - **Application**: Use cases and port interfaces (traits)
//! - **Infrastructure**: Adapter implementations (cpal, OpenAI/ChatGPT, clipboard, etc.)
//! - **CLI**: Command-line interface, argument parsing, and signal handling
//! - **GUI**: Recording indicator — Wayland layer-shell overlay on Linux,
//!   system tray icon on Windows.

pub mod application;
pub mod cli;
pub mod domain;
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub mod gui;
pub mod infrastructure;
