//! FLAC encoder for Gemini API compatibility
//!
//! FLAC provides lossless compression, giving Gemini the highest
//! quality audio input while still being compressed (~40% of WAV size).
//!
//! Settings:
//! - 16kHz sample rate (speech-optimized)
//! - Mono channel
//! - 16-bit samples

use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::config;
use flacenc::error::Verify;
use flacenc::source::MemSource;

/// Target sample rate for speech-optimized encoding
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Bits per sample (16-bit audio)
const BITS_PER_SAMPLE: usize = 16;

/// Number of channels (mono)
const CHANNELS: usize = 1;

/// Encode PCM samples to FLAC format
///
/// Input: mono i16 samples at 16kHz
/// Output: FLAC bytes
pub fn encode_to_flac(pcm_samples: &[i16]) -> Result<Vec<u8>, EncodingError> {
    // Convert i16 to i32 (flacenc uses i32 internally)
    let samples_i32: Vec<i32> = pcm_samples.iter().map(|&s| s as i32).collect();

    // Create encoder config
    let config = config::Encoder::default()
        .into_verified()
        .map_err(|(_, e)| EncodingError::Config(format!("{:?}", e)))?;

    // Create memory source from samples
    let source = MemSource::from_samples(
        &samples_i32,
        CHANNELS,
        BITS_PER_SAMPLE,
        TARGET_SAMPLE_RATE as usize,
    );

    // Encode
    let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| EncodingError::Encode(format!("{:?}", e)))?;

    // Write to bytes
    let mut sink = ByteSink::new();
    flac_stream
        .write(&mut sink)
        .map_err(|e| EncodingError::Write(e.to_string()))?;

    Ok(sink.into_inner())
}

/// FLAC encoding errors
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("FLAC config error: {0}")]
    Config(String),

    #[error("FLAC encoding failed: {0}")]
    Encode(String),

    #[error("FLAC write failed: {0}")]
    Write(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_silence() {
        // 1 second of silence at 16kHz
        let silence = vec![0i16; TARGET_SAMPLE_RATE as usize];
        let result = encode_to_flac(&silence);
        assert!(result.is_ok());

        let flac_data = result.unwrap();
        // Should have valid FLAC data with header
        assert!(flac_data.len() > 50);
        // FLAC magic number: "fLaC"
        assert_eq!(&flac_data[0..4], b"fLaC");
    }

    #[test]
    fn encode_short_audio() {
        // 100ms of silence (1600 samples at 16kHz)
        let silence = vec![0i16; 1600];
        let result = encode_to_flac(&silence);
        assert!(result.is_ok());
    }

    #[test]
    fn encode_with_signal() {
        // Generate a simple sine wave (440Hz)
        let samples: Vec<i16> = (0..TARGET_SAMPLE_RATE as usize)
            .map(|i| {
                let t = i as f32 / TARGET_SAMPLE_RATE as f32;
                (f32::sin(2.0 * std::f32::consts::PI * 440.0 * t) * 16000.0) as i16
            })
            .collect();

        let result = encode_to_flac(&samples);
        assert!(result.is_ok());

        let flac_data = result.unwrap();
        // FLAC should compress the data
        assert!(flac_data.len() < samples.len() * 2); // Less than raw PCM size
    }

    #[test]
    fn target_sample_rate_is_16khz() {
        assert_eq!(TARGET_SAMPLE_RATE, 16000);
    }
}
