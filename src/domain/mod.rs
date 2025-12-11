//! Domain layer - Core business logic
//!
//! Contains value objects, entities, and domain errors.
//! This layer has no dependencies on external systems.

pub mod recording;
pub mod transcription;
pub mod config;
pub mod daemon;
pub mod error;

// Re-export common types
pub use error::*;
pub use recording::Duration;
pub use transcription::{AudioData, AudioMimeType, DomainId, SystemPrompt};
pub use config::AppConfig;
pub use daemon::{DaemonSession, DaemonState};
