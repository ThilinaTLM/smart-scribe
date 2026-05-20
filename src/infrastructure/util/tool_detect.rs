//! Detect availability of external command-line tools.
//!
//! `keystroke::factory` and `smart_paste` both need to ask "is `xdotool` on
//! PATH?" and "is the ydotoold socket reachable?". Before this module each
//! callsite had its own copy; centralising lets a single fix (e.g. switching
//! from `which` to an explicit PATH walk) land in one place.
//!
//! `is_ydotool_socket_available` is Linux-only by definition; on non-Linux
//! targets a stub returning `false` is provided so call sites in cross-
//! platform modules (e.g. `smart_paste::create_smart_paste`) still compile
//! without scattering `#[cfg]` attributes. Those call sites are themselves
//! only reached from Linux-gated runtime paths.

use std::process::Stdio;

use tokio::process::Command;

/// Returns true if `tool` is on `PATH` (resolved via `which`).
///
/// Performs no other validation; the tool may still fail at runtime for
/// other reasons (permissions, missing runtime dependencies, etc.).
pub async fn is_command_available(tool: &str) -> bool {
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

/// Returns true if the ydotoold socket exists at the conventional locations
/// (`$XDG_RUNTIME_DIR/.ydotool_socket` or `/tmp/.ydotool_socket`).
#[cfg(target_os = "linux")]
pub fn is_ydotool_socket_available() -> bool {
    use std::env;
    use std::path::Path;

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

/// Non-Linux stub: ydotool only exists on Linux, so the socket never does.
/// Kept so cross-platform modules can import the symbol unconditionally.
#[cfg(not(target_os = "linux"))]
pub fn is_ydotool_socket_available() -> bool {
    false
}
