//! Audio cue infrastructure adapters
//!
//! Provides audio feedback when recording starts, stops, or is cancelled.

mod noop;
mod rodio;

pub use noop::NoOpAudioCue;
pub use rodio::RodioAudioCue;

use crate::application::ports::AudioCue;

/// Create an audio cue adapter based on whether audio cues are enabled
pub fn create_audio_cue(enabled: bool) -> Box<dyn AudioCue> {
    if enabled {
        Box::new(RodioAudioCue::new())
    } else {
        Box::new(NoOpAudioCue::new())
    }
}
