//! Recording port interfaces

use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

use crate::domain::recording::Duration;
use crate::domain::transcription::AudioData;

/// Recording errors
#[derive(Debug, Clone, Error)]
pub enum RecordingError {
    #[error("Failed to start recording: {0}")]
    StartFailed(String),

    #[error("Recording failed: {0}")]
    RecordingFailed(String),

    #[error("Failed to read audio file: {0}")]
    ReadFailed(String),

    #[error("Recording was cancelled")]
    Cancelled,

    #[error("No audio device available")]
    NoAudioDevice,
}

/// Progress callback type for reporting recording progress.
/// Parameters: (elapsed_ms, total_ms)
pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// Port for bounded audio recording (fixed duration)
#[async_trait]
pub trait AudioRecorder: Send + Sync {
    /// Record audio for a fixed duration.
    ///
    /// # Arguments
    /// * `duration` - How long to record
    /// * `on_progress` - Optional callback for progress updates
    ///
    /// # Returns
    /// The recorded audio data or an error
    async fn record(
        &self,
        duration: Duration,
        on_progress: Option<ProgressCallback>,
    ) -> Result<AudioData, RecordingError>;
}

/// Port for unbounded audio recording (signal-controlled, daemon mode)
#[async_trait]
pub trait UnboundedRecorder: Send + Sync {
    /// Start an unbounded recording session.
    ///
    /// # Returns
    /// A recording handle that can be used to stop/cancel
    async fn start(&self) -> Result<(), RecordingError>;

    /// Stop the recording and return the audio data.
    ///
    /// # Returns
    /// The recorded audio data or an error
    async fn stop(&self) -> Result<AudioData, RecordingError>;

    /// Cancel the recording without returning data.
    async fn cancel(&self) -> Result<(), RecordingError>;

    /// Check if currently recording
    fn is_recording(&self) -> bool;

    /// Get elapsed recording time in milliseconds
    fn elapsed_ms(&self) -> u64;
}
