use object_store::ClientOptions;

#[derive(Debug, Clone, Default)]
pub struct MetastoreSettingsConfig {
    pub object_store_client_options: ClientOptions,
}

impl MetastoreSettingsConfig {
    #[must_use]
    pub fn with_object_store_timeout(self, timeout_secs: u64) -> Self {
        Self {
            object_store_client_options: self
                .object_store_client_options
                .with_timeout(std::time::Duration::from_secs(timeout_secs)),
        }
    }

    #[must_use]
    pub fn with_object_store_connect_timeout(self, timeout_secs: u64) -> Self {
        Self {
            object_store_client_options: self
                .object_store_client_options
                .with_connect_timeout(std::time::Duration::from_secs(timeout_secs)),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<ClientOptions> for MetastoreSettingsConfig {
    fn into(self) -> ClientOptions {
        self.object_store_client_options
    }
}
