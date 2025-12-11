//! Keystroke port interface

use async_trait::async_trait;
use thiserror::Error;

/// Keystroke errors
#[derive(Debug, Clone, Error)]
pub enum KeystrokeError {
    #[error("xdotool not found. Please install xdotool.")]
    XdotoolNotFound,

    #[error("Failed to type text: {0}")]
    TypeFailed(String),
}

/// Port for keystroke injection
#[async_trait]
pub trait Keystroke: Send + Sync {
    /// Type text into the currently focused window.
    ///
    /// # Arguments
    /// * `text` - The text to type
    ///
    /// # Returns
    /// Ok(()) on success, error otherwise
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError>;
}
