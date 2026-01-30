//! Clipboard port interface

use async_trait::async_trait;
use thiserror::Error;

/// Clipboard errors
#[derive(Debug, Clone, Error)]
pub enum ClipboardError {
    #[error("wl-copy not found. Please install wl-clipboard.")]
    WlCopyNotFound,

    #[error("Clipboard unavailable: {0}")]
    ClipboardUnavailable(String),

    #[error("Failed to copy to clipboard: {0}")]
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
