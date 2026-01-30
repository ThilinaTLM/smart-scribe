//! GUI module for recording indicator (Linux only)
//!
//! Uses Wayland layer-shell for proper overlay behavior on Linux.

pub mod layer_shell;

pub use layer_shell::run_indicator;
