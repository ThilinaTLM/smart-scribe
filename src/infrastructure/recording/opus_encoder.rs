//! Opus encoder utility matching FFmpeg's voip-optimized settings
//!
//! Target settings to match FFmpeg:
//! - Sample rate: 16kHz (-ar 16000)
//! - Channels: Mono (-ac 1)
//! - Codec: Opus (-c:a libopus)
//! - Bitrate: 16kbps (-b:a 16k)
//! - Application: VOIP (-application voip)
//!
//! Result: ~2KB/second of audio (very efficient for Gemini API)

use ogg::writing::PacketWriteEndInfo;

/// Target sample rate for speech-optimized encoding
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Opus frame size in samples (20ms at 16kHz)
pub const FRAME_SIZE: usize = 320;

/// Target bitrate in bits per second
const TARGET_BITRATE: i32 = 16000;

/// Opus encoder matching FFmpeg's voip-optimized settings
pub struct OpusEncoder {
    encoder: opus::Encoder,
    serial: u32,
    granule_pos: u64,
}

impl OpusEncoder {
    /// Create encoder with FFmpeg-equivalent settings:
    /// - 16kHz sample rate
    /// - Mono
    /// - VOIP application (optimized for speech)
    /// - 16kbps target bitrate
    pub fn new() -> Result<Self, opus::Error> {
        let mut encoder = opus::Encoder::new(
            TARGET_SAMPLE_RATE,
            opus::Channels::Mono,
            opus::Application::Voip,
        )?;

        // Set bitrate to 16kbps (matches -b:a 16k)
        encoder.set_bitrate(opus::Bitrate::Bits(TARGET_BITRATE))?;

        // Additional optimizations for speech
        encoder.set_vbr(true)?; // Variable bitrate for better quality/size
        encoder.set_inband_fec(true)?; // Forward error correction for robustness

        // Generate a random serial number for the Ogg stream
        let serial = rand_serial();

        Ok(Self {
            encoder,
            serial,
            granule_pos: 0,
        })
    }

    /// Encode PCM samples to Opus in OGG container format
    ///
    /// Input should be mono i16 samples at 16kHz sample rate.
    /// Returns the complete OGG file as bytes.
    pub fn encode_to_ogg(&mut self, pcm_samples: &[i16]) -> Result<Vec<u8>, EncodingError> {
        let mut ogg_data = Vec::new();

        // Create OGG writer
        let mut packet_writer =
            ogg::writing::PacketWriter::new(std::io::Cursor::new(&mut ogg_data));

        // Write Opus header packets
        self.write_opus_header(&mut packet_writer)?;

        // Encode audio frames
        let mut frame_num = 0;
        for chunk in pcm_samples.chunks(FRAME_SIZE) {
            // Pad last frame if needed
            let frame = if chunk.len() < FRAME_SIZE {
                let mut padded = vec![0i16; FRAME_SIZE];
                padded[..chunk.len()].copy_from_slice(chunk);
                padded
            } else {
                chunk.to_vec()
            };

            // Encode the frame
            let mut opus_packet = vec![0u8; 4000]; // Max Opus packet size
            let len = self
                .encoder
                .encode(&frame, &mut opus_packet)
                .map_err(|e| EncodingError::OpusEncode(e.to_string()))?;
            opus_packet.truncate(len);

            // Update granule position (samples so far)
            self.granule_pos += FRAME_SIZE as u64;
            frame_num += 1;

            // Determine if this is the last packet
            let is_last = (frame_num * FRAME_SIZE) >= pcm_samples.len();
            let end_info = if is_last {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::NormalPacket
            };

            packet_writer
                .write_packet(opus_packet, self.serial, end_info, self.granule_pos)
                .map_err(|e| EncodingError::OggWrite(e.to_string()))?;
        }

        // Get the data out of the cursor
        drop(packet_writer);

        Ok(ogg_data)
    }

    /// Write Opus identification and comment headers
    fn write_opus_header<W: std::io::Write>(
        &self,
        writer: &mut ogg::writing::PacketWriter<W>,
    ) -> Result<(), EncodingError> {
        // Opus identification header (required by spec)
        let mut id_header = Vec::with_capacity(19);
        id_header.extend_from_slice(b"OpusHead"); // Magic signature
        id_header.push(1); // Version
        id_header.push(1); // Channel count (mono)
        id_header.extend_from_slice(&0u16.to_le_bytes()); // Pre-skip
        id_header.extend_from_slice(&TARGET_SAMPLE_RATE.to_le_bytes()); // Original sample rate
        id_header.extend_from_slice(&0i16.to_le_bytes()); // Output gain
        id_header.push(0); // Channel mapping family

        writer
            .write_packet(id_header, self.serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| EncodingError::OggWrite(e.to_string()))?;

        // Opus comment header (required by spec)
        let mut comment_header = Vec::new();
        comment_header.extend_from_slice(b"OpusTags"); // Magic signature
        let vendor = b"smart-scribe";
        comment_header.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment_header.extend_from_slice(vendor);
        comment_header.extend_from_slice(&0u32.to_le_bytes()); // No user comments

        writer
            .write_packet(comment_header, self.serial, PacketWriteEndInfo::EndPage, 0)
            .map_err(|e| EncodingError::OggWrite(e.to_string()))?;

        Ok(())
    }
}

impl Default for OpusEncoder {
    fn default() -> Self {
        Self::new().expect("Failed to create Opus encoder")
    }
}

/// Generate a pseudo-random serial number for the Ogg stream
fn rand_serial() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Mix time components for randomness
    (duration.as_secs() as u32) ^ duration.subsec_nanos()
}

/// Encoding errors
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("Opus encoding failed: {0}")]
    OpusEncode(String),

    #[error("Failed to write OGG packet: {0}")]
    OggWrite(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_creates_successfully() {
        let encoder = OpusEncoder::new();
        assert!(encoder.is_ok());
    }

    #[test]
    fn encode_silence() {
        let mut encoder = OpusEncoder::new().unwrap();
        // 1 second of silence at 16kHz
        let silence = vec![0i16; TARGET_SAMPLE_RATE as usize];
        let result = encoder.encode_to_ogg(&silence);
        assert!(result.is_ok());

        let ogg_data = result.unwrap();
        // Should have valid OGG data with Opus headers
        assert!(ogg_data.len() > 50); // At minimum, headers + some data
        assert!(ogg_data.starts_with(b"OggS")); // OGG magic number
    }

    #[test]
    fn encode_short_audio() {
        let mut encoder = OpusEncoder::new().unwrap();
        // 100ms of silence (less than one frame)
        let silence = vec![0i16; 1600];
        let result = encoder.encode_to_ogg(&silence);
        assert!(result.is_ok());
    }

    #[test]
    fn frame_size_is_20ms() {
        // At 16kHz, 20ms = 320 samples
        assert_eq!(FRAME_SIZE, 320);
        assert_eq!(FRAME_SIZE as f32 / TARGET_SAMPLE_RATE as f32 * 1000.0, 20.0);
    }
}
