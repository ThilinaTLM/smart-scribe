//! Smart paste infrastructure module
//!
//! Provides clipboard-based paste for Linux KDE Plasma Wayland,
//! using kdotool, wl-clipboard, and ydotool.

mod kdotool;
mod noop;

pub use kdotool::KdotoolSmartPaste;
pub use noop::NoOpSmartPaste;

use std::env;
use std::path::Path;
use std::process::Stdio;

use crate::application::ports::{SmartPaste, SmartPasteError};

/// Check if a tool binary is available using `which`
async fn is_tool_available(tool: &str) -> bool {
    tokio::process::Command::new("which")
        .arg(tool)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if ydotool daemon socket exists
fn is_ydotool_socket_available() -> bool {
    let socket_paths = [
        env::var("XDG_RUNTIME_DIR")
            .map(|dir| format!("{}/.ydotool_socket", dir))
            .ok(),
        Some("/tmp/.ydotool_socket".to_string()),
    ];

    for path in socket_paths.into_iter().flatten() {
        if Path::new(&path).exists() {
            return true;
        }
    }

    false
}

/// Create a smart paste adapter, checking that all required tools are available.
///
/// Required tools: kdotool, wl-copy, wl-paste, ydotool (binary + daemon)
///
/// Returns an error describing which tool is missing.
pub async fn create_smart_paste() -> Result<Box<dyn SmartPaste>, SmartPasteError> {
    if !is_tool_available("kdotool").await {
        return Err(SmartPasteError::KdotoolNotFound);
    }

    if !is_tool_available("wl-copy").await {
        return Err(SmartPasteError::WlCopyNotFound);
    }

    if !is_tool_available("wl-paste").await {
        return Err(SmartPasteError::WlPasteNotFound);
    }

    if !is_tool_available("ydotool").await || !is_ydotool_socket_available() {
        return Err(SmartPasteError::YdotoolNotAvailable);
    }

    Ok(Box::new(KdotoolSmartPaste::new()))
}
