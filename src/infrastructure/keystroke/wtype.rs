//! Wtype keystroke adapter for Wayland support

use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::application::ports::{Keystroke, KeystrokeError};

/// Wtype keystroke adapter for Wayland keystroke injection
///
/// Uses the wtype tool which is a Wayland-native text input tool.
pub struct WtypeKeystroke;

impl WtypeKeystroke {
    /// Create a new wtype keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for WtypeKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for WtypeKeystroke {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        let status = Command::new("wtype")
            .arg(text)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    KeystrokeError::WtypeNotFound
                } else {
                    KeystrokeError::TypeFailed(e.to_string())
                }
            })?;

        if !status.success() {
            return Err(KeystrokeError::TypeFailed(format!(
                "wtype exited with status: {}",
                status
            )));
        }

        Ok(())
    }
}
