//! Clipboard port interface

use async_trait::async_trait;
use thiserror::Error;

/// Clipboard errors
#[derive(Debug, Clone, Error)]
pub enum ClipboardError {
    #[error("wl-copy not found. Please install wl-clipboard.")]
    WlCopyNotFound,

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
