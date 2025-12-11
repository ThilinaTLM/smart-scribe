//! Configuration port interface

use async_trait::async_trait;
use std::path::PathBuf;

use crate::domain::config::AppConfig;
use crate::domain::error::ConfigError;

/// Port for configuration storage
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// Load configuration from storage.
    ///
    /// # Returns
    /// The loaded config (may have None fields if file doesn't exist)
    async fn load(&self) -> Result<AppConfig, ConfigError>;

    /// Save configuration to storage.
    ///
    /// # Arguments
    /// * `config` - The configuration to save
    async fn save(&self, config: &AppConfig) -> Result<(), ConfigError>;

    /// Get the configuration file path.
    fn path(&self) -> PathBuf;

    /// Check if configuration file exists.
    fn exists(&self) -> bool;

    /// Initialize configuration file with defaults.
    /// Fails if file already exists.
    async fn init(&self) -> Result<(), ConfigError>;
}
