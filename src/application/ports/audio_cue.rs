//! Audio cue port for playing sound feedback
//!
//! Provides audible feedback when recording starts, stops, or is cancelled.

use async_trait::async_trait;
use thiserror::Error;

/// Types of audio cues that can be played
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCueType {
    /// High beep when recording starts (880Hz, 150ms)
    RecordingStart,
    /// Low beep when recording stops (440Hz, 150ms)
    RecordingStop,
    /// Double-beep when recording is cancelled (330Hz, 2×75ms + 50ms gap)
    RecordingCancel,
}

/// Errors that can occur during audio cue playback
#[derive(Error, Debug)]
pub enum AudioCueError {
    /// Failed to play the audio cue
    #[error("Playback failed: {0}")]
    PlaybackFailed(String),

    /// No audio output device available
    #[error("Audio device not available: {0}")]
    DeviceNotAvailable(String),
}

/// Port trait for audio cue playback
#[async_trait]
pub trait AudioCue: Send + Sync {
    /// Play an audio cue
    async fn play(&self, cue_type: AudioCueType) -> Result<(), AudioCueError>;
}
