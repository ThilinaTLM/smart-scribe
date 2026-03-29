//! Ydotool keystroke adapter for Wayland support

use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::application::ports::{Keystroke, KeystrokeError};

/// Ydotool keystroke adapter for Wayland keystroke injection
///
/// Requires ydotoold daemon to be running and user to be in the input group.
pub struct YdotoolKeystroke;

impl YdotoolKeystroke {
    /// Create a new ydotool keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for YdotoolKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for YdotoolKeystroke {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        let status = Command::new("ydotool")
            .args(["type", "--", text])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    KeystrokeError::YdotoolNotAvailable
                } else {
                    KeystrokeError::TypeFailed(e.to_string())
                }
            })?;

        if !status.success() {
            return Err(KeystrokeError::TypeFailed(format!(
                "ydotool exited with status: {}",
                status
            )));
        }

        Ok(())
    }
}
