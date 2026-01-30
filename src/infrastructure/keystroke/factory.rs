//! Keystroke tool factory with automatic detection

use std::fmt;

#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Stdio;

#[cfg(target_os = "linux")]
use tokio::process::Command;

use crate::application::ports::{Keystroke, KeystrokeError};

use super::enigo::EnigoKeystroke;
#[cfg(target_os = "linux")]
use super::wtype::WtypeKeystroke;
#[cfg(target_os = "linux")]
use super::xdotool::XdotoolKeystroke;
#[cfg(target_os = "linux")]
use super::ydotool::YdotoolKeystroke;

/// Available keystroke tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeystrokeTool {
    /// Cross-platform enigo library
    Enigo,
    /// Linux: ydotool (requires ydotoold daemon)
    Ydotool,
    /// Linux: wtype (Wayland native)
    Wtype,
    /// Linux: xdotool (X11)
    Xdotool,
}

impl fmt::Display for KeystrokeTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeystrokeTool::Enigo => write!(f, "enigo"),
            KeystrokeTool::Ydotool => write!(f, "ydotool"),
            KeystrokeTool::Wtype => write!(f, "wtype"),
            KeystrokeTool::Xdotool => write!(f, "xdotool"),
        }
    }
}

/// Check if ydotool is available (binary exists AND daemon socket exists)
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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
/// On Windows/macOS: Always uses Enigo
/// On Linux: Priority is ydotool → wtype → xdotool → Enigo
pub async fn detect_keystroke_tool() -> Option<KeystrokeTool> {
    // On non-Linux platforms, use Enigo
    #[cfg(not(target_os = "linux"))]
    {
        return Some(KeystrokeTool::Enigo);
    }

    // On Linux, try native tools first, then fall back to Enigo
    #[cfg(target_os = "linux")]
    {
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

        // Fall back to Enigo on Linux if no native tools available
        Some(KeystrokeTool::Enigo)
    }
}

/// Create a keystroke adapter using the best available tool
///
/// Returns the adapter and the detected tool, or an error if no tool is available.
pub async fn create_keystroke() -> Result<(Box<dyn Keystroke>, KeystrokeTool), KeystrokeError> {
    match detect_keystroke_tool().await {
        Some(KeystrokeTool::Enigo) => Ok((
            Box::new(EnigoKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Enigo,
        )),
        #[cfg(target_os = "linux")]
        Some(KeystrokeTool::Ydotool) => Ok((
            Box::new(YdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Ydotool,
        )),
        #[cfg(target_os = "linux")]
        Some(KeystrokeTool::Wtype) => Ok((
            Box::new(WtypeKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Wtype,
        )),
        #[cfg(target_os = "linux")]
        Some(KeystrokeTool::Xdotool) => Ok((
            Box::new(XdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Xdotool,
        )),
        #[cfg(not(target_os = "linux"))]
        Some(_) => Ok((
            Box::new(EnigoKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Enigo,
        )),
        None => Err(KeystrokeError::NoToolAvailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_tool_display() {
        assert_eq!(KeystrokeTool::Enigo.to_string(), "enigo");
        assert_eq!(KeystrokeTool::Ydotool.to_string(), "ydotool");
        assert_eq!(KeystrokeTool::Wtype.to_string(), "wtype");
        assert_eq!(KeystrokeTool::Xdotool.to_string(), "xdotool");
    }
}
