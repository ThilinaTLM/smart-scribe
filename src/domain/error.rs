//! Domain error types

use thiserror::Error;

/// Error when parsing a duration string
#[derive(Debug, Clone, Error)]
#[error("Invalid duration format: \"{input}\". Expected format: <number>s, <number>m, or <number>m<number>s (e.g., 30s, 1m, 2m30s)")]
pub struct DurationParseError {
    pub input: String,
}

/// Error when an invalid domain ID is provided
#[derive(Debug, Clone, Error)]
#[error("Invalid domain: \"{input}\". Valid domains are: general, dev, medical, legal, finance")]
pub struct InvalidDomainError {
    pub input: String,
}

/// Error when configuration fails
#[derive(Debug, Clone, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    #[error("Failed to parse config file: {0}")]
    ParseError(String),

    #[error("Failed to write config file: {0}")]
    WriteError(String),

    #[error("Invalid config value for '{key}': {message}")]
    ValidationError { key: String, message: String },

    #[error("Config file already exists at: {0}")]
    AlreadyExists(String),
}
