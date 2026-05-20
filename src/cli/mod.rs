//! CLI layer - Command-line interface
//!
//! Contains argument parsing, output formatting, signal handling,
//! and the main application runners.

pub mod app;
pub mod args;
pub mod auth_cmd;
pub mod config_cmd;
pub mod config_schema;
pub mod daemon_app;
pub mod daemon_cmd;
pub mod exit_codes;
pub mod ipc;
pub mod output;
pub mod pid_file;
pub mod presenter;
pub mod runtime;
pub mod signals;

// Re-export commonly used types
pub use app::run_oneshot;
#[cfg(target_os = "linux")]
pub use args::IndicatorPosition;
pub use args::{
    AuthAction, Cli, Commands, ConfigAction, DaemonAction, DaemonOptions, OutputFormatArg,
    TranscribeOptions,
};
pub use daemon_app::run_daemon;
pub use daemon_cmd::handle_daemon_command;
pub use presenter::Presenter;
