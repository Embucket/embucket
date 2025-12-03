use super::catalogs::embucket::catalog::EmbucketCatalog;
use super::catalogs::embucket::iceberg_catalog::EmbucketIcebergCatalog;
use crate::catalog::{CachingCatalog, CatalogType, Properties};
use crate::df_error;
use crate::error::{
    self as catalog_error, InvalidCacheSnafu, MetastoreSnafu, MissingVolumeSnafu,
    NotImplementedSnafu, Result, UnsupportedFeature,
};
use crate::schema::CachingSchema;
use crate::table::CachingTable;
use crate::utils::fetch_table_providers;
use catalog_metastore::{
    AwsCredentials, Database, Metastore, RwObject, S3TablesVolume, VolumeType,
};
use dashmap::DashMap;
use datafusion::{
    catalog::{CatalogProvider, CatalogProviderList},
    execution::object_store::ObjectStoreRegistry,
};
use datafusion_iceberg::catalog::catalog::IcebergCatalog as DataFusionIcebergCatalog;
use iceberg_rest_catalog::apis::configuration::{AWSv4Key, Configuration};
use iceberg_rest_catalog::catalog::RestCatalog;
use iceberg_rust::catalog::Catalog;
use iceberg_rust::object_store::ObjectStoreBuilder;
use object_store::ObjectStore;
use object_store::local::LocalFileSystem;
use secrecy::SecretString;
use snafu::OptionExt;
use snafu::ResultExt;
use std::any::Any;
use std::sync::Arc;
use url::Url;

pub const DEFAULT_CATALOG: &str = "embucket";

pub struct EmbucketCatalogList {
    pub metastore: Arc<dyn Metastore>,
    pub table_object_store: Arc<DashMap<String, Arc<dyn ObjectStore>>>,
    pub catalogs: DashMap<String, Arc<CachingCatalog>>,
    pub config: CatalogListConfig,
}

#[derive(Default, Clone)]
pub struct CatalogListConfig {
    pub max_concurrent_table_fetches: usize,
}

impl EmbucketCatalogList {
    pub fn new(metastore: Arc<dyn Metastore>, config: CatalogListConfig) -> Self {
        let table_object_store: DashMap<String, Arc<dyn ObjectStore>> = DashMap::new();
        table_object_store.insert("file://".to_string(), Arc::new(LocalFileSystem::new()));
        Self {
            metastore,
            table_object_store: Arc::new(table_object_store),
            catalogs: DashMap::default(),
            config,
        }
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::drop_catalog",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn drop_catalog(&self, name: &str, cascade: bool) -> Result<()> {
        let Some((_, catalog)) = self.catalogs.remove(name) else {
            return InvalidCacheSnafu {
                entity: "catalog",
                name,
            }
            .fail();
        };
        match catalog.catalog_type {
            CatalogType::Embucket | CatalogType::Memory => self
                .metastore
                .delete_database(&name.to_string(), cascade)
                .await
                .context(MetastoreSnafu),
            CatalogType::S3tables => NotImplementedSnafu {
                feature: UnsupportedFeature::DropS3TablesDatabase,
                details: "Dropping S3 tables catalogs is not supported",
            }
            .fail(),
        }
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::create_catalog",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn create_catalog(&self, catalog_name: &str, volume_ident: &str) -> Result<()> {
        let volume = self
            .metastore
            .get_volume(&volume_ident.to_string())
            .await
            .context(MetastoreSnafu)?
            .context(MissingVolumeSnafu {
                name: volume_ident.to_string(),
            })?;

        let ident = Database {
            ident: catalog_name.to_owned(),
            volume: volume_ident.to_owned(),
            properties: None,
            should_refresh: false,
        };
        let database = self
            .metastore
            .create_database(&catalog_name.to_owned(), ident)
            .await
            .context(MetastoreSnafu)?;

        let catalog = match &volume.volume {
            VolumeType::S3(_) | VolumeType::File(_) => self.get_embucket_catalog(&database)?,
            VolumeType::Memory => self
                .get_embucket_catalog(&database)?
                .with_catalog_type(CatalogType::Memory),
            VolumeType::S3Tables(vol) => {
                self.s3tables_iceberg_catalog(vol.clone(), &database)
                    .await?
            }
        };
        self.catalogs
            .insert(catalog_name.to_owned(), Arc::new(catalog));
        Ok(())
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
        // Add metastore databases as catalogs
        let all_catalogs = self.metastore_catalogs().await?;
        for catalog in all_catalogs {
            self.catalogs
                .insert(catalog.name.clone(), Arc::new(catalog));
        }
        Ok(())
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::internal_catalogs",
        level = "trace",
        skip(self),
        err
    )]
    pub async fn metastore_catalogs(&self) -> Result<Vec<CachingCatalog>> {
        let mut catalogs = Vec::new();
        let databases = self
            .metastore
            .list_databases()
            .await
            .context(MetastoreSnafu)?;
        for db in databases {
            let volume = self
                .metastore
                .get_volume(&db.volume)
                .await
                .context(MetastoreSnafu)?
                .context(MissingVolumeSnafu {
                    name: db.volume.clone(),
                })?;
            // Create catalog depending on the volume type
            let catalog = match &volume.volume {
                VolumeType::S3Tables(vol) => {
                    self.s3tables_iceberg_catalog(vol.clone(), &db).await?
                }
                _ => self.get_embucket_catalog(&db)?,
            };
            catalogs.push(catalog);
        }
        Ok(catalogs)
    }

    fn get_embucket_catalog(&self, db: &RwObject<Database>) -> Result<CachingCatalog> {
        let iceberg_catalog: Arc<dyn Catalog> = Arc::new(
            EmbucketIcebergCatalog::new(self.metastore.clone(), db.ident.clone())
                .context(MetastoreSnafu)?,
        );
        let catalog_provider: Arc<dyn CatalogProvider> = Arc::new(EmbucketCatalog::new(
            db.ident.clone(),
            self.metastore.clone(),
            iceberg_catalog.clone(),
        ));
        Ok(
            CachingCatalog::new(catalog_provider, db.ident.clone(), Some(iceberg_catalog))
                .with_refresh(db.should_refresh)
                .with_properties(Properties {
                    created_at: db.created_at,
                    updated_at: db.created_at,
                })
                .with_metastore(self.metastore.clone()),
        )
    }

    #[tracing::instrument(
        name = "EmbucketCatalogList::s3tables_catalog",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn s3tables_iceberg_catalog(
        &self,
        volume: S3TablesVolume,
        db: &RwObject<Database>,
    ) -> Result<CachingCatalog> {
        let (ak, sk, token) = match volume.credentials {
            AwsCredentials::AccessKey(ref creds) => (
                Some(creds.aws_access_key_id.clone()),
                Some(creds.aws_secret_access_key.clone()),
                None,
            ),
            AwsCredentials::Token(ref t) => (None, None, Some(t.clone())),
        };
        let base_path = volume.endpoint.clone().unwrap_or_else(|| {
            format!("https://s3tables.{}.amazonaws.com/iceberg", volume.region())
        });
        let iceberg_catalog: Arc<dyn Catalog> = Arc::new(RestCatalog::new(
            Some(volume.arn.as_str()),
            Configuration {
                base_path,
                aws_v4_key: Some(AWSv4Key {
                    access_key: ak.unwrap_or_default(),
                    secret_key: SecretString::new(sk.unwrap_or_default()),
                    session_token: token.map(SecretString::new),
                    region: volume.region(),
                    service: "s3tables".to_string(),
                }),
                ..Default::default()
            },
            Some(ObjectStoreBuilder::S3(Box::new(volume.s3_builder()))),
        ));
        let catalog = DataFusionIcebergCatalog::new_sync(iceberg_catalog.clone(), None);
        Ok(
            CachingCatalog::new(Arc::new(catalog), db.ident.clone(), Some(iceberg_catalog))
                .with_refresh(db.should_refresh)
                .with_catalog_type(CatalogType::S3tables),
        )
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
                            name: schema.clone(),
                            iceberg_catalog: catalog.iceberg_catalog.clone(),
                        };
                        let table_providers = fetch_table_providers(
                            Arc::clone(&schema.schema),
                            self.config.max_concurrent_table_fetches,
                        )
                        .await
                        .context(catalog_error::DataFusionSnafu)?;

                        for (table_name, table_provider) in table_providers {
                            schema.tables_cache.insert(
                                table_name.clone(),
                                Arc::new(CachingTable::new_with_schema(
                                    table_name,
                                    table_provider.schema(),
                                    Arc::clone(&table_provider),
                                )),
                            );
                        }
                        catalog
                            .schemas_cache
                            .insert(schema.name.clone(), Arc::new(schema));
                    }
                }
                // Cleanup removed schemas from the cache
                for schema in &catalog.schemas_cache {
                    if !schemas.contains(&schema.key().clone()) {
                        catalog.schemas_cache.remove(schema.key());
                    }
                }
            }
        }
        Ok(())
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
            df_error::ObjectStoreNotFoundSnafu { url }.fail()?
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
        let catalog = CachingCatalog::new(catalog, name, None);
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
