//! Recording infrastructure module
//!
//! Provides cross-platform audio recording using cpal (primary) or FFmpeg (fallback).
//! Audio is encoded to FLAC format for lossless, Gemini-compatible output.

mod cpal_recorder;
mod ffmpeg;
mod flac_encoder;

pub use cpal_recorder::CpalRecorder;
pub use ffmpeg::FfmpegRecorder;
pub use flac_encoder::{encode_to_flac, TARGET_SAMPLE_RATE};

/// Create the default recorder for the current platform
///
/// Uses cpal-based recording (cross-platform) as the primary option.
/// FFmpeg can still be used as a fallback if needed.
pub fn create_recorder() -> CpalRecorder {
    CpalRecorder::new()
}
