use super::catalogs::embucket::catalog::EmbucketCatalog;
use super::catalogs::embucket::iceberg_catalog::EmbucketIcebergCatalog;
use crate::catalog::CachingCatalog;
use crate::catalogs::slatedb::catalog::{SLATEDB_CATALOG, SlateDBCatalog};
use crate::error::{self as df_catalog_error, InvalidCacheSnafu, MetastoreSnafu, Result};
use crate::schema::CachingSchema;
use crate::table::CachingTable;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_credential_types::provider::SharedCredentialsProvider;
use core_history::HistoryStore;
use core_metastore::{AwsCredentials, Metastore, VolumeType as MetastoreVolumeType};
use core_metastore::{SchemaIdent, TableIdent};
use core_utils::scan_iterator::ScanIterator;
use dashmap::DashMap;
use datafusion::{
    catalog::{CatalogProvider, CatalogProviderList},
    execution::object_store::ObjectStoreRegistry,
};
use datafusion_common::DataFusionError;
use datafusion_iceberg::catalog::catalog::IcebergCatalog as DataFusionIcebergCatalog;
use iceberg_rust::object_store::ObjectStoreBuilder;
use iceberg_s3tables_catalog::S3TablesCatalog;
use object_store::ObjectStore;
use object_store::local::LocalFileSystem;
use snafu::ResultExt;
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use url::Url;

pub const DEFAULT_CATALOG: &str = "embucket";

#[derive(Debug, Eq, PartialEq)]
pub enum CachedEntity {
    Schema(SchemaIdent),
    Table(TableIdent),
}

pub struct EmbucketCatalogList {
    pub metastore: Arc<dyn Metastore>,
    pub history_store: Arc<dyn HistoryStore>,
    pub table_object_store: Arc<DashMap<String, Arc<dyn ObjectStore>>>,
    pub catalogs: DashMap<String, Arc<CachingCatalog>>,
}

impl EmbucketCatalogList {
    pub fn new(metastore: Arc<dyn Metastore>, history_store: Arc<dyn HistoryStore>) -> Self {
        let table_object_store: DashMap<String, Arc<dyn ObjectStore>> = DashMap::new();
        table_object_store.insert("file://".to_string(), Arc::new(LocalFileSystem::new()));
        Self {
            metastore,
            history_store,
            table_object_store: Arc::new(table_object_store),
            catalogs: DashMap::default(),
        }
    }

    /// Discovers and registers all available catalogs into the catalog registry.
    ///
    /// This method performs the following steps:
    /// 1. Retrieves internal catalogs from the metastore (typically representing Iceberg-backed databases).
    /// 2. Retrieves external catalogs (e.g., `S3Tables`) from volume definitions in the metastore.
    ///
    /// # Errors
    ///
    /// This method can fail in the following cases:
    /// - Failure to access or query the metastore (e.g., database listing or volume parsing).
    /// - Errors initializing internal or external catalogs (e.g., Iceberg metadata failures).
    #[allow(clippy::as_conversions)]
    #[tracing::instrument(
        name = "EmbucketCatalogList::register_catalogs",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn register_catalogs(self: &Arc<Self>) -> Result<()> {
        let mut all_catalogs = Vec::new();
        // Internal catalogs
        all_catalogs.extend(self.internal_catalogs().await?);
        // Add the SlateDB catalog to support querying against internal tables via SQL
        all_catalogs.push(self.slatedb_catalog());
        // Load external catalogs defined via metastore volumes (e.g., S3 tables)
        all_catalogs.extend(self.external_catalogs().await?);

        for catalog in all_catalogs {
            self.catalogs
                .insert(catalog.name.clone(), Arc::new(catalog));
        }
        Ok(())
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::internal_catalogs",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn internal_catalogs(&self) -> Result<Vec<CachingCatalog>> {
        self.metastore
            .iter_databases()
            .collect()
            .await
            .context(df_catalog_error::CoreSnafu)?
            .into_iter()
            .map(|db| {
                let iceberg_catalog =
                    EmbucketIcebergCatalog::new(self.metastore.clone(), db.ident.clone())
                        .context(MetastoreSnafu)?;
                let catalog: Arc<dyn CatalogProvider> = Arc::new(EmbucketCatalog::new(
                    db.ident.clone(),
                    self.metastore.clone(),
                    Arc::new(iceberg_catalog),
                ));
                Ok(CachingCatalog::new(catalog, db.ident.clone()).with_refresh(true))
            })
            .collect()
    }

    #[must_use]
    #[tracing::instrument(
        name = "EmbucketCatalogList::slatedb_catalog",
        level = "debug",
        skip(self)
    )]
    pub fn slatedb_catalog(&self) -> CachingCatalog {
        let catalog: Arc<dyn CatalogProvider> = Arc::new(SlateDBCatalog::new(
            self.metastore.clone(),
            self.history_store.clone(),
        ));
        CachingCatalog::new(catalog, SLATEDB_CATALOG.to_string())
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::external_catalogs",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn external_catalogs(&self) -> Result<Vec<CachingCatalog>> {
        let volumes = self
            .metastore
            .iter_volumes()
            .collect()
            .await
            .context(df_catalog_error::CoreSnafu)?
            .into_iter()
            .filter_map(|v| match v.volume.clone() {
                MetastoreVolumeType::S3Tables(s3) => Some(s3),
                _ => None,
            })
            .collect::<Vec<_>>();

        if volumes.is_empty() {
            return Ok(vec![]);
        }

        let mut catalogs = Vec::with_capacity(volumes.len());
        for volume in volumes {
            let (ak, sk, token) = match volume.credentials {
                AwsCredentials::AccessKey(ref creds) => (
                    Some(creds.aws_access_key_id.clone()),
                    Some(creds.aws_secret_access_key.clone()),
                    None,
                ),
                AwsCredentials::Token(ref t) => (None, None, Some(t.clone())),
            };
            let creds =
                Credentials::from_keys(ak.unwrap_or_default(), sk.unwrap_or_default(), token);
            let config = SdkConfig::builder()
                .behavior_version(BehaviorVersion::latest())
                .credentials_provider(SharedCredentialsProvider::new(creds))
                .region(Region::new(volume.region.clone()))
                .build();
            let catalog = S3TablesCatalog::new(
                &config,
                volume.arn.as_str(),
                ObjectStoreBuilder::S3(Box::new(volume.s3_builder())),
            )
            .context(df_catalog_error::S3TablesSnafu)?;

            let catalog = DataFusionIcebergCatalog::new(Arc::new(catalog), None)
                .await
                .context(df_catalog_error::DataFusionSnafu)?;
            catalogs.push(CachingCatalog::new(Arc::new(catalog), volume.name.clone()));
        }
        Ok(catalogs)
    }

    /// Do not keep returned references to avoid deadlocks
    fn catalog_ref_by_name(
        &self,
        name: &str,
    ) -> Result<dashmap::mapref::one::Ref<'_, String, Arc<CachingCatalog>>> {
        self.catalogs.get(name).ok_or_else(|| {
            InvalidCacheSnafu {
                entity: "catalog",
                name,
            }
            .build()
        })
    }

    /// Invalidates the cache for a specific catalog entity (schema or table).
    ///
    /// This method ensures that the cache for the specified entity is refreshed or cleared as appropriate.
    /// - For a schema: If the schema exists in the underlying catalog, it is (re-)cached; if it does not exist, the cache entry is removed.
    /// - For a table: The table cache is invalidated. If the table exists in the underlying schema, it is (re-)cached; otherwise, the cache entry is removed.
    ///
    /// # Arguments
    /// * `entity` - The cached entity to invalidate, which can be either a schema or a table.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The specified catalog or schema does not exist in the cache.
    /// - There is a failure when accessing the underlying catalog or schema provider.
    #[allow(clippy::as_conversions, clippy::too_many_lines)]
    #[tracing::instrument(
        name = "EmbucketCatalogList::refresh_schema",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn invalidate_cache(&self, entity: CachedEntity) -> Result<()> {
        match entity {
            CachedEntity::Schema(schema_ident) => {
                let SchemaIdent { schema, database } = schema_ident;
                let catalog_ref = self.catalog_ref_by_name(&database)?;
                if !catalog_ref.should_refresh {
                    return Ok(());
                }
                if let Some(schema_provider) = catalog_ref.catalog.schema(&schema) {
                    // schema exists -> ensure it's cached
                    if catalog_ref.schemas_cache.get(&schema).is_none() {
                        let schema = CachingSchema {
                            schema: schema_provider,
                            tables_cache: DashMap::default(),
                            name: schema.to_string(),
                        };
                        catalog_ref
                            .schemas_cache
                            .insert(schema.name.clone(), Arc::new(schema));
                    }
                } else {
                    // no schema exists -> ensure cache is empty
                    catalog_ref.schemas_cache.remove(&schema);
                }
            }
            CachedEntity::Table(table_ident) => {
                let TableIdent {
                    database,
                    schema,
                    table,
                } = table_ident;
                let catalog_ref = self.catalog_ref_by_name(&database)?;
                if !catalog_ref.should_refresh {
                    return Ok(());
                }
                let schema_ref = catalog_ref.schemas_cache.get(&schema).ok_or_else(|| {
                    InvalidCacheSnafu {
                        entity: "schema",
                        name: format!("{database}.{schema}"),
                    }
                    .build()
                })?;

                // invalidate table cache if table exists, noop if doesn't
                schema_ref.tables_cache.remove(&table);

                if let Some(table_provider) = schema_ref
                    .schema
                    .table(&table)
                    .await
                    .context(df_catalog_error::DataFusionSnafu)?
                {
                    // ensure table is cached
                    schema_ref.tables_cache.insert(
                        table.to_string(),
                        Arc::new(CachingTable::new_with_schema(
                            table.to_string(),
                            table_provider.schema(),
                            Arc::clone(&table_provider),
                        )),
                    );
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::as_conversions, clippy::too_many_lines)]
    #[tracing::instrument(
        name = "EmbucketCatalogList::refresh",
        level = "debug",
        skip(self),
        fields(catalogs_to_refresh),
        err
    )]
    pub async fn refresh(&self) -> Result<()> {
        // Record the result as part of the current span.
        tracing::Span::current().record(
            "catalogs_to_refresh",
            format!(
                "{:?}",
                self.catalogs
                    .iter()
                    .filter(|cat| cat.should_refresh)
                    .map(|cat| cat.name.clone())
                    .collect::<Vec<_>>()
            ),
        );

        for catalog in self.catalogs.iter_mut() {
            if catalog.should_refresh {
                let schemas = catalog.schema_names();
                for schema in schemas.clone() {
                    if let Some(schema_provider) = catalog.catalog.schema(&schema) {
                        let schema = CachingSchema {
                            schema: schema_provider,
                            tables_cache: DashMap::default(),
                            name: schema.to_string(),
                        };
                        let tables = schema.schema.table_names();
                        for table in tables {
                            if let Some(table_provider) = schema
                                .schema
                                .table(&table)
                                .await
                                .context(df_catalog_error::DataFusionSnafu)?
                            {
                                schema.tables_cache.insert(
                                    table.clone(),
                                    Arc::new(CachingTable::new_with_schema(
                                        table,
                                        table_provider.schema(),
                                        Arc::clone(&table_provider),
                                    )),
                                );
                            }
                        }
                        catalog
                            .schemas_cache
                            .insert(schema.name.clone(), Arc::new(schema));
                    }
                }
                // Cleanup removed schemas from the cache
                for schema in &catalog.schemas_cache {
                    if !schemas.contains(&schema.key().to_string()) {
                        catalog.schemas_cache.remove(schema.key());
                    }
                }
            }
        }
        Ok(())
    }

    /// Spawns a background task that periodically refreshes the list of internal catalogs
    /// by querying the metastore for newly created databases.
    ///
    /// # Description
    /// When a user creates a new database, it is not automatically available as a catalog
    /// in the current session. This method solves that limitation by launching an asynchronous
    /// background job that runs every 10 seconds.
    /// - Fetches the list of databases via `internal_catalogs()`.
    /// - Adds any new catalogs not already present in `self.catalogs`.
    ///
    /// If any error occurs during fetching or processing, the error is logged via `tracing::warn`
    /// but does not interrupt the loop.
    ///
    /// This ensures that newly created databases are gradually recognized and integrated
    /// into the query engine without requiring a full session restart.
    #[tracing::instrument(
        name = "EmbucketCatalogList::start_refresh_internal_catalogs_task",
        level = "debug",
        skip(self)
    )]
    pub fn start_refresh_internal_catalogs_task(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                match self.internal_catalogs().await {
                    Ok(catalogs) => {
                        for catalog in catalogs {
                            if self.catalogs.contains_key(&catalog.name) {
                                continue;
                            }
                            self.catalogs
                                .insert(catalog.name.clone(), Arc::new(catalog));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to refresh internal catalogs: {:?}", e);
                    }
                }
            }
        });
    }
}

impl std::fmt::Debug for EmbucketCatalogList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbucketCatalogList").finish()
    }
}

/// Get the key of a url for object store registration.
/// The credential info will be removed
#[must_use]
fn get_url_key(url: &Url) -> String {
    format!(
        "{}://{}",
        url.scheme(),
        &url[url::Position::BeforeHost..url::Position::AfterPort],
    )
}

impl ObjectStoreRegistry for EmbucketCatalogList {
    #[tracing::instrument(
        name = "ObjectStoreRegistry::register_store",
        level = "debug",
        skip(self, store)
    )]
    fn register_store(
        &self,
        url: &Url,
        store: Arc<dyn ObjectStore>,
    ) -> Option<Arc<dyn ObjectStore>> {
        let url = get_url_key(url);
        self.table_object_store.insert(url, store)
    }

    #[tracing::instrument(
        name = "ObjectStoreRegistry::get_store",
        level = "debug",
        skip(self),
        err
    )]
    fn get_store(&self, url: &Url) -> datafusion_common::Result<Arc<dyn ObjectStore>> {
        let url = get_url_key(url);
        if let Some(object_store) = self.table_object_store.get(&url) {
            Ok(object_store.clone())
        } else {
            Err(DataFusionError::Execution(format!(
                "Object store not found for url {url}"
            )))
        }
    }
}

impl CatalogProviderList for EmbucketCatalogList {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::register_catalog",
        level = "debug",
        skip(self, catalog)
    )]
    fn register_catalog(
        &self,
        name: String,
        catalog: Arc<dyn CatalogProvider>,
    ) -> Option<Arc<dyn CatalogProvider>> {
        let catalog = CachingCatalog::new(catalog, name);
        self.catalogs
            .insert(catalog.name.clone(), Arc::new(catalog))
            .map(|arc| {
                let catalog: Arc<dyn CatalogProvider> = arc;
                catalog
            })
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::catalog_names",
        level = "debug",
        skip(self)
    )]
    fn catalog_names(&self) -> Vec<String> {
        self.catalogs.iter().map(|c| c.key().clone()).collect()
    }

    #[allow(clippy::as_conversions)]
    #[tracing::instrument(name = "EmbucketCatalogList::catalog", level = "debug", skip(self))]
    fn catalog(&self, name: &str) -> Option<Arc<dyn CatalogProvider>> {
        self.catalogs
            .get(name)
            .map(|c| Arc::clone(c.value()) as Arc<dyn CatalogProvider>)
    }
}
