//! Wayland clipboard adapter using wl-copy

use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::application::ports::{Clipboard, ClipboardError};

/// Wayland clipboard adapter using wl-copy
pub struct WaylandClipboard;

impl WaylandClipboard {
    /// Create a new Wayland clipboard adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for WaylandClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clipboard for WaylandClipboard {
    async fn copy(&self, text: &str) -> Result<(), ClipboardError> {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ClipboardError::WlCopyNotFound
                } else {
                    ClipboardError::CopyFailed(e.to_string())
                }
            })?;

        // Write text to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| ClipboardError::CopyFailed(e.to_string()))?;
        }

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .map_err(|e| ClipboardError::CopyFailed(e.to_string()))?;

        if !status.success() {
            return Err(ClipboardError::CopyFailed(format!(
                "wl-copy exited with status: {}",
                status
            )));
        }

        Ok(())
    }
}
