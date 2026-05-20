//! Clipboard port interface

use async_trait::async_trait;
use thiserror::Error;

/// Clipboard errors.
///
/// The application layer treats these errors uniformly: the choice of backend
/// (X11 vs Wayland vs native API) is an infrastructure concern. Backend
/// adapters report which tool failed in [`BackendUnavailable::tool`].
#[derive(Debug, Clone, Error)]
pub enum ClipboardError {
    /// The requested clipboard backend (e.g. `wl-copy`) is not installed or
    /// not reachable. The application layer logs this as a warning and skips
    /// the clipboard step.
    #[error("Clipboard backend `{tool}` is not available: {reason}")]
    BackendUnavailable { tool: String, reason: String },

    /// The backend was reachable but the copy itself failed.
    #[error("Clipboard copy failed: {0}")]
    CopyFailed(String),
}

/// Port for clipboard operations
#[async_trait]
pub trait Clipboard: Send + Sync {
    /// Copy text to the system clipboard.
    ///
    /// # Arguments
    /// * `text` - The text to copy
    ///
    /// # Returns
    /// Ok(()) on success, error otherwise
    async fn copy(&self, text: &str) -> Result<(), ClipboardError>;
}

/// Blanket implementation for boxed clipboard types
#[async_trait]
impl Clipboard for Box<dyn Clipboard> {
    async fn copy(&self, text: &str) -> Result<(), ClipboardError> {
        self.as_ref().copy(text).await
    }
}
