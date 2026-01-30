//! Cross-platform clipboard adapter using arboard
//!
//! Works on Windows, macOS, and Linux (X11/Wayland).

use async_trait::async_trait;

use crate::application::ports::{Clipboard, ClipboardError};

/// Cross-platform clipboard adapter using arboard
pub struct ArboardClipboard;

impl ArboardClipboard {
    /// Create a new arboard clipboard adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArboardClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clipboard for ArboardClipboard {
    async fn copy(&self, text: &str) -> Result<(), ClipboardError> {
        let text = text.to_owned();

        // arboard operations are blocking, so run in spawn_blocking
        tokio::task::spawn_blocking(move || {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| ClipboardError::ClipboardUnavailable(e.to_string()))?;

            clipboard
                .set_text(&text)
                .map_err(|e| ClipboardError::CopyFailed(e.to_string()))
        })
        .await
        .map_err(|e| ClipboardError::CopyFailed(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_creates_successfully() {
        let _clipboard = ArboardClipboard::new();
    }

    #[test]
    fn clipboard_default_creates() {
        let _clipboard = ArboardClipboard::default();
    }
}
