//! CLI layer - Command-line interface
//!
//! Contains argument parsing, output formatting, signal handling,
//! and the main application runners.

pub mod args;
pub mod presenter;
pub mod signals;
pub mod pid_file;
pub mod app;
pub mod daemon_app;
pub mod config_cmd;

// Re-export commonly used types
pub use args::{Cli, Commands, ConfigAction, DaemonOptions, TranscribeOptions};
pub use presenter::Presenter;
pub use app::{run_oneshot, EXIT_SUCCESS, EXIT_ERROR, EXIT_USAGE_ERROR};
pub use daemon_app::run_daemon;
