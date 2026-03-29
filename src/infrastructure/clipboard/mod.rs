//! Clipboard infrastructure module
//!
//! Provides cross-platform clipboard support using arboard (primary)
//! or wl-copy as the preferred option on Wayland.

#[cfg(target_os = "linux")]
use std::process::Stdio;

mod arboard;
mod wayland;

pub use arboard::ArboardClipboard;
pub use wayland::WaylandClipboard;

use crate::application::ports::Clipboard;

/// Check if running under a Wayland session
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Check if wl-copy is available
#[cfg(target_os = "linux")]
async fn is_wl_copy_available() -> bool {
    tokio::process::Command::new("which")
        .arg("wl-copy")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Create the default clipboard adapter for the current platform.
///
/// On Wayland: prefers wl-copy (persists clipboard after process exits).
/// Otherwise: uses arboard (cross-platform).
///
/// Returns the adapter and a label describing which backend was selected.
pub async fn create_clipboard() -> (Box<dyn Clipboard>, &'static str) {
    #[cfg(target_os = "linux")]
    {
        if is_wayland() && is_wl_copy_available().await {
            return (Box::new(WaylandClipboard::new()), "wl-copy");
        }
    }
    (Box::new(ArboardClipboard::new()), "arboard")
}
