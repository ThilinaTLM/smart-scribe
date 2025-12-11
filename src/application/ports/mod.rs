//! Port interfaces (traits) for external systems
//!
//! These traits define the boundaries between the application
//! and infrastructure layers.

pub mod clipboard;
pub mod config;
pub mod keystroke;
pub mod notifier;
pub mod recorder;
pub mod transcriber;

// Re-export common types
pub use clipboard::{Clipboard, ClipboardError};
pub use config::ConfigStore;
pub use keystroke::{Keystroke, KeystrokeError};
pub use notifier::{NotificationError, NotificationIcon, Notifier};
pub use recorder::{AudioRecorder, ProgressCallback, RecordingError, UnboundedRecorder};
pub use transcriber::{Transcriber, TranscriptionError};
