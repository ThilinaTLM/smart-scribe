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
pub use clipboard::{create_clipboard, ArboardClipboard, WaylandClipboard};
pub use config::XdgConfigStore;
pub use keystroke::{
    create_keystroke, detect_keystroke_tool, EnigoKeystroke, KeystrokeTool, NoOpKeystroke,
    YdotoolKeystroke,
};
pub use notification::{create_notifier, NotifyRustNotifier, NotifySendNotifier};
pub use recording::{create_recorder, CpalRecorder, FfmpegRecorder};
pub use transcription::GeminiTranscriber;
