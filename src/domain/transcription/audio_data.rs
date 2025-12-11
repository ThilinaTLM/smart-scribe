//! Audio data value object

use std::fmt;

/// Supported audio MIME types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioMimeType {
    Ogg,
    Mp3,
    Mpeg,
    Wav,
    Webm,
    Mp4,
}

impl AudioMimeType {
    /// Get the MIME type string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Ogg => "audio/ogg",
            Self::Mp3 => "audio/mp3",
            Self::Mpeg => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Webm => "audio/webm",
            Self::Mp4 => "audio/mp4",
        }
    }

    /// Get the file extension
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Ogg => "ogg",
            Self::Mp3 | Self::Mpeg => "mp3",
            Self::Wav => "wav",
            Self::Webm => "webm",
            Self::Mp4 => "mp4",
        }
    }
}

impl fmt::Display for AudioMimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for AudioMimeType {
    fn default() -> Self {
        Self::Ogg
    }
}

/// Value object representing audio data ready for transcription.
/// Contains raw audio bytes and its MIME type.
#[derive(Debug, Clone)]
pub struct AudioData {
    data: Vec<u8>,
    mime_type: AudioMimeType,
}

impl AudioData {
    /// Create AudioData from raw bytes
    pub fn new(data: Vec<u8>, mime_type: AudioMimeType) -> Self {
        Self { data, mime_type }
    }

    /// Create AudioData from a byte slice
    pub fn from_bytes(data: &[u8], mime_type: AudioMimeType) -> Self {
        Self {
            data: data.to_vec(),
            mime_type,
        }
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

    /// Get human-readable size
    pub fn human_readable_size(&self) -> String {
        let bytes = self.size_bytes();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    /// Encode the audio data as base64
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_as_str() {
        assert_eq!(AudioMimeType::Ogg.as_str(), "audio/ogg");
        assert_eq!(AudioMimeType::Mp3.as_str(), "audio/mp3");
        assert_eq!(AudioMimeType::Wav.as_str(), "audio/wav");
    }

    #[test]
    fn mime_type_extension() {
        assert_eq!(AudioMimeType::Ogg.extension(), "ogg");
        assert_eq!(AudioMimeType::Mp3.extension(), "mp3");
        assert_eq!(AudioMimeType::Wav.extension(), "wav");
    }

    #[test]
    fn audio_data_size() {
        let data = AudioData::new(vec![0u8; 1024], AudioMimeType::Ogg);
        assert_eq!(data.size_bytes(), 1024);
    }

    #[test]
    fn human_readable_size_bytes() {
        let data = AudioData::new(vec![0u8; 500], AudioMimeType::Ogg);
        assert_eq!(data.human_readable_size(), "500 B");
    }

    #[test]
    fn human_readable_size_kb() {
        let data = AudioData::new(vec![0u8; 2048], AudioMimeType::Ogg);
        assert_eq!(data.human_readable_size(), "2.0 KB");
    }

    #[test]
    fn human_readable_size_mb() {
        let data = AudioData::new(vec![0u8; 2 * 1024 * 1024], AudioMimeType::Ogg);
        assert_eq!(data.human_readable_size(), "2.0 MB");
    }

    #[test]
    fn to_base64() {
        let data = AudioData::new(vec![1, 2, 3, 4], AudioMimeType::Ogg);
        let b64 = data.to_base64();
        assert!(!b64.is_empty());
        // Verify it's valid base64 by decoding
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .unwrap();
        assert_eq!(decoded, vec![1, 2, 3, 4]);
    }

    #[test]
    fn from_bytes() {
        let bytes = [1u8, 2, 3, 4];
        let data = AudioData::from_bytes(&bytes, AudioMimeType::Mp3);
        assert_eq!(data.data(), &[1, 2, 3, 4]);
        assert_eq!(data.mime_type(), AudioMimeType::Mp3);
    }

    #[test]
    fn default_mime_type_is_ogg() {
        assert_eq!(AudioMimeType::default(), AudioMimeType::Ogg);
    }
}
