//! Infrastructure layer - Adapter implementations
//!
//! Contains concrete implementations of the port interfaces,
//! integrating with external systems like FFmpeg, Gemini API, etc.

pub mod recording;
pub mod transcription;
pub mod clipboard;
pub mod keystroke;
pub mod notification;
pub mod config;

// Re-export adapters
pub use recording::FfmpegRecorder;
pub use transcription::GeminiTranscriber;
pub use clipboard::WaylandClipboard;
pub use keystroke::XdotoolKeystroke;
pub use notification::NotifySendNotifier;
pub use config::XdgConfigStore;
