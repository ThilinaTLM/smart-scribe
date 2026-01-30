//! Keystroke tool factory with automatic detection

use std::fmt;
use std::str::FromStr;

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

/// User preference for keystroke tool selection.
///
/// - All platforms support `Enigo` (the default).
/// - Linux additionally supports `Auto`, `Ydotool`, `Xdotool`, and `Wtype`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeystrokeToolPreference {
    /// Use cross-platform enigo library (default on all platforms)
    #[default]
    Enigo,
    /// Auto-detect best native tool (Linux only)
    #[cfg(target_os = "linux")]
    Auto,
    /// Use ydotool (Linux only, requires ydotoold daemon)
    #[cfg(target_os = "linux")]
    Ydotool,
    /// Use xdotool (Linux only, X11)
    #[cfg(target_os = "linux")]
    Xdotool,
    /// Use wtype (Linux only, Wayland native)
    #[cfg(target_os = "linux")]
    Wtype,
}

impl fmt::Display for KeystrokeToolPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeystrokeToolPreference::Enigo => write!(f, "enigo"),
            #[cfg(target_os = "linux")]
            KeystrokeToolPreference::Auto => write!(f, "auto"),
            #[cfg(target_os = "linux")]
            KeystrokeToolPreference::Ydotool => write!(f, "ydotool"),
            #[cfg(target_os = "linux")]
            KeystrokeToolPreference::Xdotool => write!(f, "xdotool"),
            #[cfg(target_os = "linux")]
            KeystrokeToolPreference::Wtype => write!(f, "wtype"),
        }
    }
}

/// Error type for parsing keystroke tool preference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseKeystrokeToolError {
    pub value: String,
    pub valid_options: &'static str,
}

impl fmt::Display for ParseKeystrokeToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid keystroke tool '{}'. Valid options: {}",
            self.value, self.valid_options
        )
    }
}

impl std::error::Error for ParseKeystrokeToolError {}

impl FromStr for KeystrokeToolPreference {
    type Err = ParseKeystrokeToolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "enigo" => Ok(KeystrokeToolPreference::Enigo),
            #[cfg(target_os = "linux")]
            "auto" => Ok(KeystrokeToolPreference::Auto),
            #[cfg(target_os = "linux")]
            "ydotool" => Ok(KeystrokeToolPreference::Ydotool),
            #[cfg(target_os = "linux")]
            "xdotool" => Ok(KeystrokeToolPreference::Xdotool),
            #[cfg(target_os = "linux")]
            "wtype" => Ok(KeystrokeToolPreference::Wtype),
            _ => Err(ParseKeystrokeToolError {
                value: s.to_string(),
                #[cfg(target_os = "linux")]
                valid_options: "enigo, auto, ydotool, xdotool, wtype",
                #[cfg(not(target_os = "linux"))]
                valid_options: "enigo",
            }),
        }
    }
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

/// Create a keystroke adapter using the specified preference.
///
/// Returns the adapter and the detected tool, or an error if no tool is available.
///
/// On non-Linux platforms, always uses Enigo regardless of preference.
pub async fn create_keystroke(
    preference: KeystrokeToolPreference,
) -> Result<(Box<dyn Keystroke>, KeystrokeTool), KeystrokeError> {
    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux, always use Enigo
        let _ = preference;
        Ok((
            Box::new(EnigoKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Enigo,
        ))
    }

    #[cfg(target_os = "linux")]
    {
        match preference {
            KeystrokeToolPreference::Enigo => Ok((
                Box::new(EnigoKeystroke::new()) as Box<dyn Keystroke>,
                KeystrokeTool::Enigo,
            )),
            KeystrokeToolPreference::Auto => {
                // Auto-detect best available tool
                match detect_keystroke_tool().await {
                    Some(tool) => create_specific_tool(tool),
                    None => Err(KeystrokeError::NoToolAvailable),
                }
            }
            KeystrokeToolPreference::Ydotool => {
                if is_ydotool_available().await {
                    Ok((
                        Box::new(YdotoolKeystroke::new()) as Box<dyn Keystroke>,
                        KeystrokeTool::Ydotool,
                    ))
                } else {
                    Err(KeystrokeError::ToolNotFound("ydotool".to_string()))
                }
            }
            KeystrokeToolPreference::Xdotool => {
                if is_tool_available("xdotool").await {
                    Ok((
                        Box::new(XdotoolKeystroke::new()) as Box<dyn Keystroke>,
                        KeystrokeTool::Xdotool,
                    ))
                } else {
                    Err(KeystrokeError::ToolNotFound("xdotool".to_string()))
                }
            }
            KeystrokeToolPreference::Wtype => {
                if is_tool_available("wtype").await {
                    Ok((
                        Box::new(WtypeKeystroke::new()) as Box<dyn Keystroke>,
                        KeystrokeTool::Wtype,
                    ))
                } else {
                    Err(KeystrokeError::ToolNotFound("wtype".to_string()))
                }
            }
        }
    }
}

/// Create a specific keystroke tool adapter
#[cfg(target_os = "linux")]
fn create_specific_tool(
    tool: KeystrokeTool,
) -> Result<(Box<dyn Keystroke>, KeystrokeTool), KeystrokeError> {
    match tool {
        KeystrokeTool::Enigo => Ok((
            Box::new(EnigoKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Enigo,
        )),
        KeystrokeTool::Ydotool => Ok((
            Box::new(YdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Ydotool,
        )),
        KeystrokeTool::Wtype => Ok((
            Box::new(WtypeKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Wtype,
        )),
        KeystrokeTool::Xdotool => Ok((
            Box::new(XdotoolKeystroke::new()) as Box<dyn Keystroke>,
            KeystrokeTool::Xdotool,
        )),
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

    #[test]
    fn keystroke_tool_preference_display() {
        assert_eq!(KeystrokeToolPreference::Enigo.to_string(), "enigo");
        #[cfg(target_os = "linux")]
        {
            assert_eq!(KeystrokeToolPreference::Auto.to_string(), "auto");
            assert_eq!(KeystrokeToolPreference::Ydotool.to_string(), "ydotool");
            assert_eq!(KeystrokeToolPreference::Xdotool.to_string(), "xdotool");
            assert_eq!(KeystrokeToolPreference::Wtype.to_string(), "wtype");
        }
    }

    #[test]
    fn keystroke_tool_preference_from_str() {
        assert_eq!(
            "enigo".parse::<KeystrokeToolPreference>().unwrap(),
            KeystrokeToolPreference::Enigo
        );
        assert_eq!(
            "ENIGO".parse::<KeystrokeToolPreference>().unwrap(),
            KeystrokeToolPreference::Enigo
        );
        #[cfg(target_os = "linux")]
        {
            assert_eq!(
                "auto".parse::<KeystrokeToolPreference>().unwrap(),
                KeystrokeToolPreference::Auto
            );
            assert_eq!(
                "ydotool".parse::<KeystrokeToolPreference>().unwrap(),
                KeystrokeToolPreference::Ydotool
            );
            assert_eq!(
                "xdotool".parse::<KeystrokeToolPreference>().unwrap(),
                KeystrokeToolPreference::Xdotool
            );
            assert_eq!(
                "wtype".parse::<KeystrokeToolPreference>().unwrap(),
                KeystrokeToolPreference::Wtype
            );
        }
    }

    #[test]
    fn keystroke_tool_preference_from_str_invalid() {
        let err = "invalid".parse::<KeystrokeToolPreference>().unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn keystroke_tool_preference_default() {
        assert_eq!(
            KeystrokeToolPreference::default(),
            KeystrokeToolPreference::Enigo
        );
    }
}
