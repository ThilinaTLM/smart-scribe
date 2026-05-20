//! Platform-specific runtime configuration.
//!
//! The flat structure (Linux + Windows fields side-by-side) is deliberate:
//! `config.toml` carries `[linux]` and `[windows]` sections on every OS so
//! profiles roam between machines, and CLI flag surfaces expect a flat field
//! set. Whether `linux_paste` is honoured on the current platform is decided
//! at the CLI/infrastructure layer (it's a no-op elsewhere).

/// Runtime platform configuration. Concrete values; no Options.
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// Preferred keystroke backend (`enigo`, `xdotool`, `wtype`, `ydotool`,
    /// or `auto`). Default `enigo`.
    pub keystroke_tool: String,
    /// Show the platform-native recording indicator (Wayland overlay on
    /// Linux, system tray on Windows).
    pub indicator: bool,
    /// Indicator anchor for the Linux overlay (`top-right`,
    /// `bottom-left`, …). Ignored on other platforms.
    pub indicator_position: String,
    /// Smart paste (capture-then-paste) on Linux KDE Wayland.
    /// `false` and ignored on non-Linux.
    pub linux_paste: bool,
    /// Show Windows balloon notifications. `false` and ignored on
    /// non-Windows.
    pub windows_show_balloon: bool,
}

impl PlatformConfig {
    /// Static defaults.
    pub fn defaults() -> Self {
        Self {
            keystroke_tool: "enigo".to_string(),
            indicator: false,
            indicator_position: "top-right".to_string(),
            linux_paste: false,
            windows_show_balloon: false,
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self::defaults()
    }
}
