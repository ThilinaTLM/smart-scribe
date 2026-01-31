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

/// Create a gentle tone with fade in/out for a smoother sound
fn gentle_tone(freq: f32, duration_ms: u64, amplitude: f32) -> impl Source<Item = f32> + Send {
    let fade_ms = (duration_ms / 5).min(30); // 20% fade or max 30ms
    SineWave::new(freq)
        .take_duration(Duration::from_millis(duration_ms))
        .fade_in(Duration::from_millis(fade_ms))
        .amplify(amplitude)
}

/// Play a cue synchronously (called from spawn_blocking)
fn play_cue_sync(cue_type: AudioCueType) -> Result<(), AudioCueError> {
    // Get output stream
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| AudioCueError::DeviceNotAvailable(e.to_string()))?;

    let sink =
        Sink::try_new(&stream_handle).map_err(|e| AudioCueError::PlaybackFailed(e.to_string()))?;

    // Softer amplitude for pleasant sound
    const AMP: f32 = 0.3;

    match cue_type {
        AudioCueType::RecordingStart => {
            // Pleasant ascending chime: C5 -> E5 (major third)
            // 523Hz (C5) -> 659Hz (E5)
            let tone1 = gentle_tone(523.0, 80, AMP);
            let tone2 = gentle_tone(659.0, 120, AMP);
            sink.append(tone1);
            sink.append(tone2);
        }
        AudioCueType::RecordingStop => {
            // Pleasant descending chime: E5 -> C5 (major third down)
            let tone1 = gentle_tone(659.0, 80, AMP);
            let tone2 = gentle_tone(523.0, 120, AMP);
            sink.append(tone1);
            sink.append(tone2);
        }
        AudioCueType::RecordingCancel => {
            // Gentle double-tap: G4 twice
            let tone1 = gentle_tone(392.0, 60, AMP * 0.8);
            let silence =
                rodio::source::Zero::<f32>::new(1, 44100).take_duration(Duration::from_millis(40));
            let tone2 = gentle_tone(392.0, 60, AMP * 0.8);
            sink.append(tone1);
            sink.append(silence);
            sink.append(tone2);
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
