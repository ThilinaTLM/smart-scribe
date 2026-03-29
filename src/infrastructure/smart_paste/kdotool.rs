//! Smart paste implementation using kdotool, wl-copy, wl-paste, and ydotool
//!
//! Linux KDE Plasma Wayland only.

use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::application::ports::{SmartPaste, SmartPasteError};

/// ydotool keycodes (from linux/input-event-codes.h)
const KEY_LEFTCTRL: &str = "29";
const KEY_LEFTSHIFT: &str = "42";
const KEY_V: &str = "47";

/// Known terminal window class names (case-insensitive matching)
const TERMINAL_CLASSES: &[&str] = &[
    "konsole",
    "alacritty",
    "kitty",
    "foot",
    "org.wezfurlong.wezterm",
    "xterm",
    "ghostty",
    "terminator",
    "tilix",
    "gnome-terminal-server",
    "xfce4-terminal",
    "sakura",
    "st",
    "urxvt",
    "yakuake",
];

/// Smart paste using kdotool (KDE), wl-clipboard, and ydotool
pub struct KdotoolSmartPaste {
    captured_window_id: Mutex<Option<String>>,
}

impl KdotoolSmartPaste {
    pub fn new() -> Self {
        Self {
            captured_window_id: Mutex::new(None),
        }
    }
}

impl Default for KdotoolSmartPaste {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmartPaste for KdotoolSmartPaste {
    async fn capture_active_window(&self) -> Result<(), SmartPasteError> {
        let output = Command::new("kdotool")
            .arg("getactivewindow")
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SmartPasteError::KdotoolNotFound
                } else {
                    SmartPasteError::PasteFailed(format!("kdotool failed: {}", e))
                }
            })?;

        if !output.status.success() {
            return Err(SmartPasteError::PasteFailed(
                "kdotool getactivewindow failed".to_string(),
            ));
        }

        let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if window_id.is_empty() {
            return Err(SmartPasteError::PasteFailed(
                "No active window found".to_string(),
            ));
        }

        *self.captured_window_id.lock().await = Some(window_id);
        Ok(())
    }

    async fn paste(&self, text: &str) -> Result<(), SmartPasteError> {
        let window_id = {
            let guard = self.captured_window_id.lock().await;
            guard.clone().ok_or(SmartPasteError::NoWindowCaptured)?
        };

        // 1. Backup current clipboard (may fail if empty — that's fine)
        let backup = backup_clipboard().await;

        // 2. Set clipboard to transcription text
        set_clipboard(text).await?;

        // 3. Activate the original window
        activate_window(&window_id).await?;

        // Small delay to let window activation settle
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 4. Detect if window is a terminal
        let is_terminal = is_terminal_window(&window_id).await;

        // 5. Send paste shortcut via ydotool
        send_paste_key(is_terminal).await?;

        // 6. Wait for paste to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 7. Restore original clipboard
        if let Some(ref backup_text) = backup {
            let _ = set_clipboard(backup_text).await;
        }

        Ok(())
    }
}

/// Backup current clipboard contents using wl-paste
async fn backup_clipboard() -> Option<String> {
    let output = Command::new("wl-paste")
        .arg("--no-newline")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    } else {
        None
    }
}

/// Set clipboard contents using wl-copy
async fn set_clipboard(text: &str) -> Result<(), SmartPasteError> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SmartPasteError::WlCopyNotFound
            } else {
                SmartPasteError::PasteFailed(format!("wl-copy failed: {}", e))
            }
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await.map_err(|e| {
            SmartPasteError::PasteFailed(format!("Failed to write to wl-copy: {}", e))
        })?;
    }

    let status = child
        .wait()
        .await
        .map_err(|e| SmartPasteError::PasteFailed(format!("wl-copy failed: {}", e)))?;

    if !status.success() {
        return Err(SmartPasteError::PasteFailed(format!(
            "wl-copy exited with status: {}",
            status
        )));
    }

    Ok(())
}

/// Activate a window by its kdotool ID
async fn activate_window(window_id: &str) -> Result<(), SmartPasteError> {
    let status = Command::new("kdotool")
        .arg("windowactivate")
        .arg(window_id)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await
        .map_err(|e| SmartPasteError::WindowActivationFailed(e.to_string()))?;

    if !status.success() {
        return Err(SmartPasteError::WindowActivationFailed(format!(
            "kdotool windowactivate exited with status: {}",
            status
        )));
    }

    Ok(())
}

/// Check if a window is a terminal based on its class name
async fn is_terminal_window(window_id: &str) -> bool {
    let output = Command::new("kdotool")
        .arg("getwindowclassname")
        .arg(window_id)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let class = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            TERMINAL_CLASSES.iter().any(|tc| class.contains(tc))
        }
        _ => false, // If we can't determine, assume non-terminal
    }
}

/// Send paste key combination via ydotool
///
/// - Non-terminal: Ctrl+V (keycodes: 29:1 47:1 47:0 29:0)
/// - Terminal: Ctrl+Shift+V (keycodes: 29:1 42:1 47:1 47:0 42:0 29:0)
async fn send_paste_key(is_terminal: bool) -> Result<(), SmartPasteError> {
    let args = if is_terminal {
        // Ctrl+Shift+V: press Ctrl, press Shift, press V, release V, release Shift, release Ctrl
        vec![
            format!("{}:1", KEY_LEFTCTRL),
            format!("{}:1", KEY_LEFTSHIFT),
            format!("{}:1", KEY_V),
            format!("{}:0", KEY_V),
            format!("{}:0", KEY_LEFTSHIFT),
            format!("{}:0", KEY_LEFTCTRL),
        ]
    } else {
        // Ctrl+V: press Ctrl, press V, release V, release Ctrl
        vec![
            format!("{}:1", KEY_LEFTCTRL),
            format!("{}:1", KEY_V),
            format!("{}:0", KEY_V),
            format!("{}:0", KEY_LEFTCTRL),
        ]
    };

    let status = Command::new("ydotool")
        .arg("key")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SmartPasteError::YdotoolNotAvailable
            } else {
                SmartPasteError::PasteFailed(format!("ydotool failed: {}", e))
            }
        })?;

    if !status.success() {
        return Err(SmartPasteError::PasteFailed(format!(
            "ydotool key exited with status: {}",
            status
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_classes_are_lowercase() {
        for class in TERMINAL_CLASSES {
            assert_eq!(
                *class,
                class.to_lowercase(),
                "Terminal class should be lowercase: {}",
                class
            );
        }
    }

    #[test]
    fn creates_successfully() {
        let _paste = KdotoolSmartPaste::new();
    }
}
