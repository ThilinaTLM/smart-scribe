//! Application layer - Use cases and port interfaces
//!
//! Contains the core business operations and trait definitions
//! for external system interactions.

pub mod ports;
pub mod transcribe;
pub mod daemon;

// Re-export use cases
pub use transcribe::{
    TranscribeRecordingUseCase, TranscribeInput, TranscribeOutput,
    TranscribeCallbacks, TranscribeError,
};
pub use daemon::{
    DaemonTranscriptionUseCase, DaemonConfig, DaemonOutput, DaemonError,
};
