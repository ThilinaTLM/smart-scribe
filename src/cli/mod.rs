//! CLI layer - Command-line interface
//!
//! Contains argument parsing, output formatting, signal handling,
//! and the main application runners.

pub mod app;
pub mod args;
pub mod config_cmd;
pub mod daemon_app;
pub mod daemon_cmd;
pub mod ipc;
pub mod pid_file;
pub mod presenter;
pub mod signals;
pub mod socket;

// Re-export commonly used types
pub use app::{run_oneshot, EXIT_ERROR, EXIT_SUCCESS, EXIT_USAGE_ERROR};
pub use args::{Cli, Commands, ConfigAction, DaemonAction, DaemonOptions, TranscribeOptions};
pub use daemon_app::run_daemon;
pub use daemon_cmd::handle_daemon_command;
pub use presenter::Presenter;
