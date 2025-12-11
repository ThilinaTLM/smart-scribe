//! Application layer - Use cases and port interfaces
//!
//! Contains the core business operations and trait definitions
//! for external system interactions.

pub mod daemon;
pub mod ports;
pub mod transcribe;

// Re-export use cases
pub use daemon::{DaemonConfig, DaemonError, DaemonOutput, DaemonTranscriptionUseCase};
pub use transcribe::{
    TranscribeCallbacks, TranscribeError, TranscribeInput, TranscribeOutput,
    TranscribeRecordingUseCase,
};
