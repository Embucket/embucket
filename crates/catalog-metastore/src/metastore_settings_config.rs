use object_store::ClientOptions;
use snafu::prelude::*;
use std::sync::OnceLock;

static GLOBAL_SETTINGS: OnceLock<MetastoreSettingsConfig> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct MetastoreSettingsConfig {
    pub object_store_config: ClientOptions,
}

#[derive(Debug, Snafu)]
pub enum MetastoreSettingsConfigError {
    #[snafu(display("Global settings are already initialized"))]
    AlreadyInitialized,

    #[snafu(display("Global settings are not initialized"))]
    NotInitialized,
}

impl MetastoreSettingsConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            object_store_config: ClientOptions::default(),
        }
    }

    #[must_use]
    pub fn with_object_store_timeout(self, timeout_secs: u64) -> Self {
        Self {
            object_store_config: self
                .object_store_config
                .with_timeout(std::time::Duration::from_secs(timeout_secs)),
        }
    }

    #[must_use]
    pub fn with_object_store_connect_timeout(self, timeout_secs: u64) -> Self {
        Self {
            object_store_config: self
                .object_store_config
                .with_connect_timeout(std::time::Duration::from_secs(timeout_secs)),
        }
    }

    pub fn initialize(self) -> Result<(), MetastoreSettingsConfigError> {
        GLOBAL_SETTINGS
            .set(self)
            .map_err(|_| MetastoreSettingsConfigError::AlreadyInitialized)
    }

    pub fn get_object_store_config() -> Result<&'static ClientOptions, MetastoreSettingsConfigError>
    {
        Ok(&GLOBAL_SETTINGS
            .get()
            .ok_or(MetastoreSettingsConfigError::NotInitialized)?
            .object_store_config)
    }
}
