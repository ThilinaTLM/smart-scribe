//! Xdotool keystroke adapter for X11 support

use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::application::ports::{Keystroke, KeystrokeError};

/// Xdotool keystroke adapter for X11 keystroke injection
///
/// Uses xdotool which works on X11 systems.
pub struct XdotoolKeystroke;

impl XdotoolKeystroke {
    /// Create a new xdotool keystroke adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for XdotoolKeystroke {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Keystroke for XdotoolKeystroke {
    async fn type_text(&self, text: &str) -> Result<(), KeystrokeError> {
        let status = Command::new("xdotool")
            .args(["type", "--delay", "2", "--", text])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    KeystrokeError::BackendUnavailable {
                        tool: "xdotool".to_string(),
                        reason: "command not found; install xdotool for X11 support".to_string(),
                    }
                } else {
                    KeystrokeError::TypeFailed {
                        tool: "xdotool".to_string(),
                        reason: e.to_string(),
                    }
                }
            })?;

        if !status.success() {
            return Err(KeystrokeError::TypeFailed {
                tool: "xdotool".to_string(),
                reason: format!("exited with status: {}", status),
            });
        }

        Ok(())
    }
}
