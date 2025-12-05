use crate::server::error::CreateExecutorSnafu;
use crate::server::error::MetastoreConfigSnafu;
use crate::server::error::Result;
use crate::server::server_models::RestApiConfig;
use api_snowflake_rest_sessions::session::SessionStore;
use catalog_metastore::InMemoryMetastore;
use catalog_metastore::Metastore;
use catalog_metastore::metastore_config::MetastoreBootstrapConfig;
use executor::service::CoreExecutionService;
use executor::utils::Config as ExecutionConfig;
use snafu::ResultExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;

pub struct CoreState {
    pub executor: Arc<CoreExecutionService>,
    pub metastore: Arc<InMemoryMetastore>,
    pub rest_api_config: RestApiConfig,
}

#[derive(Clone)]
pub enum MetastoreConfig {
    ConfigPath(PathBuf),
    DefaultConfig,
    None,
}

impl CoreState {
    pub async fn new(
        execution_cfg: ExecutionConfig,
        rest_api_config: RestApiConfig,
        metastore_config: MetastoreConfig,
    ) -> Result<Self> {
        let metastore = create_metastore();
        apply_metastore_config(metastore.clone(), metastore_config).await?;
        let executor = create_executor(metastore.clone(), execution_cfg).await?;
        Ok(Self {
            executor,
            metastore,
            rest_api_config,
        })
    }

    pub fn with_session_timeout(&self, session_timeout: Duration) -> Result<()> {
        tracing::info!(
            "With session timeout, by {} seconds",
            session_timeout.as_secs()
        );
        let session_store = SessionStore::new(self.executor.clone());
        tokio::spawn(async move {
            session_store
                .continuously_delete_expired(session_timeout)
                .await;
        });
        Ok(())
    }
}

#[must_use]
pub fn create_metastore() -> Arc<InMemoryMetastore> {
    Arc::new(InMemoryMetastore::new())
}

async fn apply_metastore_config(
    metastore: Arc<InMemoryMetastore>,
    metastore_config: MetastoreConfig,
) -> Result<()> {
    match metastore_config {
        MetastoreConfig::ConfigPath(path) => {
            tracing::info!(
                path = %path.display(),
                "Bootstrapping metastore from config"
            );
            let config = MetastoreBootstrapConfig::load(&path)
                .await
                .context(MetastoreConfigSnafu)?;
            config
                .apply(metastore.clone())
                .await
                .context(MetastoreConfigSnafu)?;
        }
        MetastoreConfig::DefaultConfig => {
            tracing::info!("Bootstrapping metastore from default config");
            MetastoreBootstrapConfig::default()
                .apply(metastore.clone())
                .await
                .context(MetastoreConfigSnafu)?;
        }
        MetastoreConfig::None => {}
    }
    Ok(())
}

pub async fn create_executor(
    metastore: Arc<dyn Metastore>,
    execution_cfg: ExecutionConfig,
) -> Result<Arc<CoreExecutionService>> {
    tracing::info!("Creating execution service");
    let executor = Arc::new(
        CoreExecutionService::new(metastore, Arc::new(execution_cfg))
            .await
            .context(CreateExecutorSnafu)?,
    );
    tracing::info!("Execution service created");
    Ok(executor)
}
