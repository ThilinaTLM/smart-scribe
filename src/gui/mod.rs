//! GUI module for recording indicator.
//!
//! Linux: Wayland layer-shell overlay (`gui::layer_shell`).
//! Windows: System tray icon (`gui::tray`).

#[cfg(target_os = "linux")]
pub mod layer_shell;
#[cfg(target_os = "linux")]
pub use layer_shell::run_indicator;

#[cfg(target_os = "windows")]
pub mod tray;
#[cfg(target_os = "windows")]
pub use tray::run_indicator;
