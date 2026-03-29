//! Smart paste port interface (Linux only)
//!
//! Captures the active window before recording, then pastes transcription
//! into that window via clipboard + paste shortcut.

use async_trait::async_trait;
use thiserror::Error;

/// Smart paste errors
#[derive(Debug, Clone, Error)]
pub enum SmartPasteError {
    #[error("kdotool not found. Install kdotool for KDE Plasma window management.")]
    KdotoolNotFound,

    #[error("wl-copy not found. Install wl-clipboard.")]
    WlCopyNotFound,

    #[error("wl-paste not found. Install wl-clipboard.")]
    WlPasteNotFound,

    #[error("ydotool not available. Install ydotool and ensure ydotoold is running.")]
    YdotoolNotAvailable,

    #[error("No active window captured")]
    NoWindowCaptured,

    #[error("Failed to activate window: {0}")]
    WindowActivationFailed(String),

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
