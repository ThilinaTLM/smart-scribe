//! Cross-platform keystroke adapter using enigo
//!
//! Works on Windows, macOS, and Linux (X11/Wayland).

use async_trait::async_trait;

use crate::application::ports::{Keystroke, KeystrokeError};

/// Cross-platform keystroke adapter using enigo
pub struct EnigoKeystroke;

impl EnigoKeystroke {
    /// Create a new enigo keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnigoKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for EnigoKeystroke {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        let text = text.to_owned();

        // enigo operations are blocking, so run in spawn_blocking
        tokio::task::spawn_blocking(move || {
            use enigo::{Enigo, Keyboard, Settings};

            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
                KeystrokeError::TypeFailed(format!("Failed to create enigo: {}", e))
            })?;

            enigo
                .text(&text)
                .map_err(|e| KeystrokeError::TypeFailed(format!("Failed to type text: {}", e)))
        })
        .await
        .map_err(|e| KeystrokeError::TypeFailed(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_creates_successfully() {
        let _keystroke = EnigoKeystroke::new();
    }

    #[test]
    fn keystroke_default_creates() {
        let _keystroke = EnigoKeystroke::default();
    }
}
