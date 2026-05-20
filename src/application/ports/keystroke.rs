//! Keystroke port interface

use async_trait::async_trait;
use thiserror::Error;

/// Keystroke errors.
///
/// Tool names appear only in the `tool` field of [`KeystrokeError::
/// BackendUnavailable`] / [`KeystrokeError::TypeFailed`]; the application
/// layer matches on the variant, not on tool name strings.
#[derive(Debug, Clone, Error)]
pub enum KeystrokeError {
    /// No keystroke backend can be used on this system.
    #[error(
        "No keystroke tool available. Install ydotool (with ydotoold running), wtype, or xdotool."
    )]
    NoBackendAvailable,

    /// The requested keystroke backend is not installed or not reachable.
    #[error("Keystroke backend `{tool}` is not available: {reason}")]
    BackendUnavailable { tool: String, reason: String },

    /// The backend was reachable but typing the text failed.
    #[error("Failed to type text via `{tool}`: {reason}")]
    TypeFailed { tool: String, reason: String },
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

/// Blanket implementation for boxed keystroke types
#[async_trait]
impl Keystroke for Box<dyn Keystroke> {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        self.as_ref().type_text(text).await
    }
}
