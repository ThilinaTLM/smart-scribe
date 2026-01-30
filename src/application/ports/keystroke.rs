//! Keystroke port interface

use async_trait::async_trait;
use thiserror::Error;

/// Keystroke errors
#[derive(Debug, Clone, Error)]
pub enum KeystrokeError {
    #[error(
        "No keystroke tool available. Install ydotool (with ydotoold running), wtype, or xdotool."
    )]
    NoToolAvailable,

    #[error("ydotool not available. Ensure ydotool is installed and ydotoold daemon is running.")]
    YdotoolNotAvailable,

    #[error("wtype not found. Please install wtype for Wayland keystroke support.")]
    WtypeNotFound,

    #[error("xdotool not found. Please install xdotool for X11 keystroke support.")]
    XdotoolNotFound,

    #[error("{0} not found. Please install the tool.")]
    ToolNotFound(String),

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

/// Blanket implementation for boxed keystroke types
#[async_trait]
impl Keystroke for Box<dyn Keystroke> {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        self.as_ref().type_text(text).await
    }
}
