//! Audio data value object

use std::fmt;

/// Supported audio MIME types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AudioMimeType {
    Mp3,
    Mpeg,
    Wav,
    Webm,
    Mp4,
    #[default]
    Flac,
}

impl AudioMimeType {
    /// Get the MIME type string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mp3",
            Self::Mpeg => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Webm => "audio/webm",
            Self::Mp4 => "audio/mp4",
            Self::Flac => "audio/flac",
        }
    }

    /// Get the file extension
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 | Self::Mpeg => "mp3",
            Self::Wav => "wav",
            Self::Webm => "webm",
            Self::Mp4 => "mp4",
            Self::Flac => "flac",
        }
    }
}

impl fmt::Display for AudioMimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Value object representing audio data ready for transcription.
/// Contains raw audio bytes and its MIME type.
#[derive(Debug, Clone)]
pub struct AudioData {
    data: Vec<u8>,
    mime_type: AudioMimeType,
    duration_ms: Option<u64>,
}

impl AudioData {
    /// Create AudioData from raw bytes
    pub fn new(data: Vec<u8>, mime_type: AudioMimeType) -> Self {
        Self {
            data,
            mime_type,
            duration_ms: None,
        }
    }

    /// Create AudioData from a byte slice
    pub fn from_bytes(data: &[u8], mime_type: AudioMimeType) -> Self {
        Self {
            data: data.to_vec(),
            mime_type,
            duration_ms: None,
        }
    }

    /// Set the duration in milliseconds
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Get the duration in milliseconds, if known
    pub fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }

    /// Get the raw audio data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Consume and return the raw audio data
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Get the MIME type
    pub fn mime_type(&self) -> AudioMimeType {
        self.mime_type
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_as_str() {
        assert_eq!(AudioMimeType::Flac.as_str(), "audio/flac");
        assert_eq!(AudioMimeType::Mp3.as_str(), "audio/mp3");
        assert_eq!(AudioMimeType::Wav.as_str(), "audio/wav");
    }

    #[test]
    fn mime_type_extension() {
        assert_eq!(AudioMimeType::Flac.extension(), "flac");
        assert_eq!(AudioMimeType::Mp3.extension(), "mp3");
        assert_eq!(AudioMimeType::Wav.extension(), "wav");
    }

    #[test]
    fn audio_data_size() {
        let data = AudioData::new(vec![0u8; 1024], AudioMimeType::Flac);
        assert_eq!(data.size_bytes(), 1024);
    }

    #[test]
    fn from_bytes() {
        let bytes = [1u8, 2, 3, 4];
        let data = AudioData::from_bytes(&bytes, AudioMimeType::Mp3);
        assert_eq!(data.data(), &[1, 2, 3, 4]);
        assert_eq!(data.mime_type(), AudioMimeType::Mp3);
    }

    #[test]
    fn default_mime_type_is_flac() {
        assert_eq!(AudioMimeType::default(), AudioMimeType::Flac);
    }
}
