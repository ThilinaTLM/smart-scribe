//! Smart paste port interface (Linux only)
//!
//! Captures the active window before recording, then pastes transcription
//! into that window via clipboard + paste shortcut.

use async_trait::async_trait;
use thiserror::Error;

/// Smart paste errors.
///
/// Tool names appear only in the `tool` field of variants; the application
/// layer matches on the variant, not on tool name strings.
#[derive(Debug, Clone, Error)]
pub enum SmartPasteError {
    /// A required backend tool (kdotool, wl-copy, ydotool, …) is not
    /// installed or not reachable.
    #[error("Smart-paste backend `{tool}` is not available: {reason}")]
    BackendUnavailable { tool: String, reason: String },

    /// No window was captured before recording started.
    #[error("No active window captured")]
    NoWindowCaptured,

    /// Re-activating the captured window failed.
    #[error("Failed to activate window: {0}")]
    WindowActivationFailed(String),

    /// The paste sequence itself failed.
    #[error("Paste failed: {0}")]
    PasteFailed(String),
}

/// Port for smart paste operations (capture window + paste via clipboard)
#[async_trait]
pub trait SmartPaste: Send + Sync {
    /// Capture the currently focused window before recording starts.
    async fn capture_active_window(&self) -> Result<(), SmartPasteError>;

    /// Paste text into the previously captured window.
    ///
    /// Flow: backup clipboard → set clipboard → activate window → paste → restore clipboard.
    async fn paste(&self, text: &str) -> Result<(), SmartPasteError>;
}

/// Blanket implementation for boxed smart paste types
#[async_trait]
impl SmartPaste for Box<dyn SmartPaste> {
    async fn capture_active_window(&self) -> Result<(), SmartPasteError> {
        self.as_ref().capture_active_window().await
    }

    async fn paste(&self, text: &str) -> Result<(), SmartPasteError> {
        self.as_ref().paste(text).await
    }
}
