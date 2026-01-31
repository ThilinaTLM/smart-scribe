//! Rodio-based audio cue adapter
//!
//! Generates and plays synthesized tones for audio feedback.

use std::time::Duration;

use async_trait::async_trait;
use rodio::source::{SineWave, Source};
use rodio::{OutputStream, Sink};

use crate::application::ports::{AudioCue, AudioCueError, AudioCueType};

/// Audio cue implementation using rodio
pub struct RodioAudioCue;

impl RodioAudioCue {
    /// Create a new rodio-based audio cue
    pub fn new() -> Self {
        Self
    }
}

impl Default for RodioAudioCue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioCue for RodioAudioCue {
    async fn play(&self, cue_type: AudioCueType) -> Result<(), AudioCueError> {
        // Run audio playback in blocking thread to avoid blocking the async runtime
        tokio::task::spawn_blocking(move || play_cue_sync(cue_type))
            .await
            .map_err(|e| AudioCueError::PlaybackFailed(format!("Task join error: {}", e)))?
    }
}

/// Play a cue synchronously (called from spawn_blocking)
fn play_cue_sync(cue_type: AudioCueType) -> Result<(), AudioCueError> {
    // Get output stream
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| AudioCueError::DeviceNotAvailable(e.to_string()))?;

    let sink =
        Sink::try_new(&stream_handle).map_err(|e| AudioCueError::PlaybackFailed(e.to_string()))?;

    // Amplitude (0.5 to avoid clipping)
    const AMPLITUDE: f32 = 0.5;

    match cue_type {
        AudioCueType::RecordingStart => {
            // High beep: 880Hz for 150ms
            let source = SineWave::new(880.0)
                .take_duration(Duration::from_millis(150))
                .amplify(AMPLITUDE);
            sink.append(source);
        }
        AudioCueType::RecordingStop => {
            // Low beep: 440Hz for 150ms
            let source = SineWave::new(440.0)
                .take_duration(Duration::from_millis(150))
                .amplify(AMPLITUDE);
            sink.append(source);
        }
        AudioCueType::RecordingCancel => {
            // Double-beep: 330Hz, 2×75ms with 50ms gap
            let beep1 = SineWave::new(330.0)
                .take_duration(Duration::from_millis(75))
                .amplify(AMPLITUDE);
            let silence =
                rodio::source::Zero::<f32>::new(1, 44100).take_duration(Duration::from_millis(50));
            let beep2 = SineWave::new(330.0)
                .take_duration(Duration::from_millis(75))
                .amplify(AMPLITUDE);

            sink.append(beep1);
            sink.append(silence);
            sink.append(beep2);
        }
    }

    // Wait for playback to complete
    sink.sleep_until_end();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require audio hardware and may not work in CI
    // They are marked as ignored by default

    #[tokio::test]
    #[ignore = "Requires audio hardware"]
    async fn can_play_start_cue() {
        let cue = RodioAudioCue::new();
        let result = cue.play(AudioCueType::RecordingStart).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires audio hardware"]
    async fn can_play_stop_cue() {
        let cue = RodioAudioCue::new();
        let result = cue.play(AudioCueType::RecordingStop).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires audio hardware"]
    async fn can_play_cancel_cue() {
        let cue = RodioAudioCue::new();
        let result = cue.play(AudioCueType::RecordingCancel).await;
        assert!(result.is_ok());
    }
}
