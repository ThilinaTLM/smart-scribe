//! Process exit codes used by every CLI entry point.
//!
//! Centralised so the CLI surface stays consistent and so we don't redeclare
//! the same constants across `app.rs`, `auth_cmd.rs`, etc.

/// Process completed successfully.
pub const SUCCESS: u8 = 0;

/// Recoverable runtime error (transcription failed, network down, etc.).
pub const ERROR: u8 = 1;

/// Invalid arguments / configuration. Matches the conventional `2` used by
/// `clap` for argument errors.
pub const USAGE_ERROR: u8 = 2;
