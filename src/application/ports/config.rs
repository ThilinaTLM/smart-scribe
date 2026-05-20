//! Configuration port interface.
//!
//! The store works on [`RawAppConfig`] (the on-disk schema) so `config
//! set/get/list` can manipulate individual optional fields and persist them.
//! Validation into the runtime [`AppConfig`](crate::domain::config::AppConfig)
//! happens in the merge step at startup, not inside the store.

use async_trait::async_trait;
use std::path::PathBuf;

use crate::domain::config::RawAppConfig;
use crate::domain::error::ConfigError;

/// Port for configuration storage.
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// Load the persisted (raw) configuration.
    ///
    /// Returns an empty [`RawAppConfig`] if no file exists; never panics.
    async fn load(&self) -> Result<RawAppConfig, ConfigError>;

    /// Persist the (raw) configuration.
    async fn save(&self, config: &RawAppConfig) -> Result<(), ConfigError>;

    /// Get the configuration file path.
    fn path(&self) -> PathBuf;

    /// Check if configuration file exists.
    fn exists(&self) -> bool;

    /// Initialise the configuration file with sensible defaults.
    /// Fails if the file already exists.
    async fn init(&self) -> Result<(), ConfigError>;
}
