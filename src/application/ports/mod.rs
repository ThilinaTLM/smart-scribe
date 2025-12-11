//! Port interfaces (traits) for external systems
//!
//! These traits define the boundaries between the application
//! and infrastructure layers.

pub mod recorder;
pub mod transcriber;
pub mod clipboard;
pub mod keystroke;
pub mod notifier;
pub mod config;

// Re-export common types
pub use recorder::{AudioRecorder, UnboundedRecorder, RecordingError, ProgressCallback};
pub use transcriber::{Transcriber, TranscriptionError};
pub use clipboard::{Clipboard, ClipboardError};
pub use keystroke::{Keystroke, KeystrokeError};
pub use notifier::{Notifier, NotificationError, NotificationIcon};
pub use config::ConfigStore;
