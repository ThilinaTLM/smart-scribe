//! Recording infrastructure module
//!
//! Provides cross-platform audio recording using cpal (primary) or FFmpeg (fallback).

mod cpal_recorder;
mod ffmpeg;
mod opus_encoder;

pub use cpal_recorder::CpalRecorder;
pub use ffmpeg::FfmpegRecorder;
pub use opus_encoder::{OpusEncoder, FRAME_SIZE, TARGET_SAMPLE_RATE};

/// Create the default recorder for the current platform
///
/// Uses cpal-based recording (cross-platform) as the primary option.
/// FFmpeg can still be used as a fallback if needed.
pub fn create_recorder() -> CpalRecorder {
    CpalRecorder::new()
}
