//! Domain layer - Core business logic
//!
//! Contains value objects, entities, and domain errors.
//! This layer has no dependencies on external systems.

pub mod config;
pub mod daemon;
pub mod error;
pub mod recording;
pub mod transcription;

// Re-export common types
pub use config::AppConfig;
pub use daemon::{DaemonSession, DaemonState};
pub use error::*;
pub use recording::Duration;
pub use transcription::{AudioData, AudioMimeType, DomainId, SystemPrompt};
