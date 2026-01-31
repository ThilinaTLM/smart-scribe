//! No-op audio cue adapter
//!
//! Used when audio cues are disabled.

use async_trait::async_trait;

use crate::application::ports::{AudioCue, AudioCueError, AudioCueType};

/// No-op audio cue that does nothing
pub struct NoOpAudioCue;

impl NoOpAudioCue {
    /// Create a new no-op audio cue
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpAudioCue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioCue for NoOpAudioCue {
    async fn play(&self, _cue_type: AudioCueType) -> Result<(), AudioCueError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_returns_ok() {
        let cue = NoOpAudioCue::new();
        assert!(cue.play(AudioCueType::RecordingStart).await.is_ok());
        assert!(cue.play(AudioCueType::RecordingStop).await.is_ok());
        assert!(cue.play(AudioCueType::RecordingCancel).await.is_ok());
    }
}
