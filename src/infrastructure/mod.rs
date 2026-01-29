//! Infrastructure layer - Adapter implementations
//!
//! Contains concrete implementations of the port interfaces,
//! integrating with external systems like FFmpeg, Gemini API, etc.

pub mod clipboard;
pub mod config;
pub mod keystroke;
pub mod notification;
pub mod recording;
pub mod transcription;

// Re-export adapters
pub use clipboard::WaylandClipboard;
pub use config::XdgConfigStore;
pub use keystroke::{create_keystroke, KeystrokeTool, NoOpKeystroke, YdotoolKeystroke};
pub use notification::NotifySendNotifier;
pub use recording::FfmpegRecorder;
pub use transcription::GeminiTranscriber;
