//! Recording infrastructure module
//!
//! Provides cross-platform audio recording using cpal.
//! Audio is encoded to FLAC format for lossless, Gemini-compatible output.

mod cpal_recorder;
mod flac_encoder;

pub use cpal_recorder::CpalRecorder;
pub use flac_encoder::{encode_to_flac, TARGET_SAMPLE_RATE};

/// Create the default recorder for the current platform
pub fn create_recorder() -> CpalRecorder {
    CpalRecorder::new()
}
