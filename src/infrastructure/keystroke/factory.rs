//! Keystroke tool factory with automatic detection

use std::env;
use std::fmt;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::application::ports::{Keystroke, KeystrokeError};

use super::wtype::WtypeKeystroke;
use super::xdotool::XdotoolKeystroke;
use super::ydotool::YdotoolKeystroke;

/// Available keystroke tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeystrokeTool {
    Ydotool,
    Wtype,
    Xdotool,
}

impl fmt::Display for KeystrokeTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeystrokeTool::Ydotool => write!(f, "ydotool"),
            KeystrokeTool::Wtype => write!(f, "wtype"),
            KeystrokeTool::Xdotool => write!(f, "xdotool"),
        }
    }
}

/// Check if ydotool is available (binary exists AND daemon socket exists)
async fn is_ydotool_available() -> bool {
    // Check if ydotool binary exists
    let binary_exists = Command::new("which")
        .arg("ydotool")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if !binary_exists {
        return false;
    }

    // Check if ydotoold socket exists
    // Try XDG_RUNTIME_DIR first, then /tmp
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

/// Check if a tool binary is available using `which`
async fn is_tool_available(tool: &str) -> bool {
    Command::new("which")
        .arg(tool)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect the best available keystroke tool
///
/// Priority: ydotool → wtype → xdotool
pub async fn detect_keystroke_tool() -> Option<KeystrokeTool> {
    // Check ydotool first (needs both binary and daemon)
    if is_ydotool_available().await {
        return Some(KeystrokeTool::Ydotool);
    }

    // Check wtype (Wayland-native)
    if is_tool_available("wtype").await {
        return Some(KeystrokeTool::Wtype);
    }

    // Check xdotool (X11 fallback)
    if is_tool_available("xdotool").await {
        return Some(KeystrokeTool::Xdotool);
    }

    None
}

/// Create a keystroke adapter using the best available tool
///
/// Returns the adapter and the detected tool, or an error if no tool is available.
pub async fn create_keystroke() -> Result<(Box<dyn Keystroke>, KeystrokeTool), KeystrokeError> {
    match detect_keystroke_tool().await {
        Some(KeystrokeTool::Ydotool) => Ok((
            Box::new(YdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Ydotool,
        )),
        Some(KeystrokeTool::Wtype) => Ok((
            Box::new(WtypeKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Wtype,
        )),
        Some(KeystrokeTool::Xdotool) => Ok((
            Box::new(XdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Xdotool,
        )),
        None => Err(KeystrokeError::NoToolAvailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_tool_display() {
        assert_eq!(KeystrokeTool::Ydotool.to_string(), "ydotool");
        assert_eq!(KeystrokeTool::Wtype.to_string(), "wtype");
        assert_eq!(KeystrokeTool::Xdotool.to_string(), "xdotool");
    }
}
