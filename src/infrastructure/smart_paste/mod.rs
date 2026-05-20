//! Smart paste infrastructure module
//!
//! Provides clipboard-based paste for Linux KDE Plasma Wayland,
//! using kdotool, wl-clipboard, and ydotool.

mod kdotool;
mod noop;

pub use kdotool::KdotoolSmartPaste;
pub use noop::NoOpSmartPaste;

use crate::application::ports::{SmartPaste, SmartPasteError};
use crate::infrastructure::util::tool_detect::{is_command_available, is_ydotool_socket_available};

/// Create a smart paste adapter, checking that all required tools are available.
///
/// Required tools: kdotool, wl-copy, wl-paste, ydotool (binary + daemon)
///
/// Returns an error describing which tool is missing.
pub async fn create_smart_paste() -> Result<Box<dyn SmartPaste>, SmartPasteError> {
    if !is_command_available("kdotool").await {
        return Err(SmartPasteError::BackendUnavailable {
            tool: "kdotool".to_string(),
            reason: "command not found; install kdotool for KDE Plasma".to_string(),
        });
    }

    if !is_command_available("wl-copy").await {
        return Err(SmartPasteError::BackendUnavailable {
            tool: "wl-copy".to_string(),
            reason: "command not found; install wl-clipboard".to_string(),
        });
    }

    if !is_command_available("wl-paste").await {
        return Err(SmartPasteError::BackendUnavailable {
            tool: "wl-paste".to_string(),
            reason: "command not found; install wl-clipboard".to_string(),
        });
    }

    if !is_command_available("ydotool").await || !is_ydotool_socket_available() {
        return Err(SmartPasteError::BackendUnavailable {
            tool: "ydotool".to_string(),
            reason: "command not found or ydotoold socket missing".to_string(),
        });
    }

    Ok(Box::new(KdotoolSmartPaste::new()))
}
