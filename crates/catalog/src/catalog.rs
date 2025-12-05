use crate::catalogs::embucket::schema::EmbucketSchema;
use crate::schema::CachingSchema;
use crate::{block_on_without_deadlock, df_error};
use catalog_metastore::Metastore;
use chrono::NaiveDateTime;
use dashmap::DashMap;
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion_common::DataFusionError;
use datafusion_iceberg::catalog::catalog::IcebergCatalog;
use datafusion_iceberg::catalog::schema::IcebergSchema;
use iceberg_rust::catalog::Catalog;
use iceberg_rust_spec::namespace::Namespace;
use snafu::{OptionExt, ResultExt};
use std::fmt::{Display, Formatter};
use std::{any::Any, sync::Arc};

#[derive(Clone)]
pub struct CachingCatalog {
    pub catalog: Arc<dyn CatalogProvider>,
    pub metastore: Option<Arc<dyn Metastore>>,
    pub iceberg_catalog: Option<Arc<dyn Catalog>>,
    pub catalog_type: CatalogType,
    pub schemas_cache: DashMap<String, Arc<CachingSchema>>,
    pub should_refresh: bool,
    pub name: String,
    pub enable_information_schema: bool,
    pub properties: Option<Properties>,
}

#[derive(Clone)]
pub struct Properties {
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Default for Properties {
    fn default() -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CatalogType {
    Embucket,
    Memory,
    S3tables,
}

impl Display for CatalogType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Embucket => write!(f, "embucket"),
            Self::Memory => write!(f, "memory"),
            Self::S3tables => write!(f, "s3_tables"),
        }
    }
}

impl CachingCatalog {
    pub fn new(
        catalog_provider: Arc<dyn CatalogProvider>,
        name: String,
        iceberg_catalog: Option<Arc<dyn Catalog>>,
    ) -> Self {
        Self {
            catalog: catalog_provider,
            iceberg_catalog,
            metastore: None,
            schemas_cache: DashMap::new(),
            should_refresh: false,
            enable_information_schema: true,
            name,
            catalog_type: CatalogType::Embucket,
            properties: None,
        }
    }
    #[must_use]
    pub const fn with_refresh(mut self, refresh: bool) -> Self {
        self.should_refresh = refresh;
        self
    }
    #[must_use]
    pub const fn with_information_schema(mut self, enable_information_schema: bool) -> Self {
        self.enable_information_schema = enable_information_schema;
        self
    }

    #[must_use]
    pub const fn with_catalog_type(mut self, catalog_type: CatalogType) -> Self {
        self.catalog_type = catalog_type;
        self
    }

    #[must_use]
    pub const fn with_properties(mut self, properties: Properties) -> Self {
        self.properties = Some(properties);
        self
    }

    #[must_use]
    pub fn with_metastore(mut self, metastore: Arc<dyn Metastore>) -> Self {
        self.metastore = Some(metastore);
        self
    }

    #[tracing::instrument(name = "CachingCatalog::iceberg_schema_provider", level = "debug", skip(self))]
    #[allow(clippy::as_conversions)]
    fn iceberg_schema_provider(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        let Some(iceberg_catalog) = &self.iceberg_catalog else {
            return None;
        };

        let namespace = Namespace::try_new(std::slice::from_ref(&name.to_string())).ok()?;
        let namespace_to_check = namespace.clone();
        let catalog = iceberg_catalog.clone();

        // Check if schema exists
        if !block_on_without_deadlock(async move {
            catalog
                .namespace_exists(&namespace_to_check)
                .await
                .unwrap_or(false)
        }) {
            return None;
        }
        let iceberg_catalog = self.catalog.as_any().downcast_ref::<IcebergCatalog>()?;
        Some(
            Arc::new(IcebergSchema::new(namespace, iceberg_catalog.mirror()))
                as Arc<dyn SchemaProvider>,
        )
    }

    #[tracing::instrument(name = "CachingCatalog::lookup_schema_provider", level = "debug", skip(self))]
    fn lookup_schema_provider(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        self.iceberg_schema_provider(name)
            .or_else(|| self.catalog.schema(name))
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for CachingCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalog")
            .field("name", &self.name)
            .field("should_refresh", &self.should_refresh)
            .field("schemas_cache", &self.schemas_cache)
            .field("catalog", &"")
            .finish()
    }
}

impl CatalogProvider for CachingCatalog {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[tracing::instrument(
        name = "CachingCatalog::schema_names",
        level = "debug",
        skip(self),
        fields(schemas_names_count, catalog_name=format!("{:?}", self.name)),
    )]
    fn schema_names(&self) -> Vec<String> {
        let schema_names = match &self.iceberg_catalog {
            Some(catalog) => {
                let catalog = catalog.clone();
                block_on_without_deadlock(async move {
                    tracing::info!("list_namespaces");
                    let l = catalog.list_namespaces(None).await.map_or_else(
                        |_| vec![],
                        |namespaces| namespaces.into_iter().map(|ns| ns.to_string()).collect(),
                    );
                    tracing::info!("list_namespaces result: {l:?}");
                    l
                })
            }
            None => self.catalog.schema_names(),
        };

        // Remove outdated records
        let schema_names_set: std::collections::HashSet<_> = schema_names.iter().cloned().collect();
        self.schemas_cache
            .retain(|name, _| schema_names_set.contains(name));

        // Record the result as part of the current span.
        tracing::Span::current().record("schemas_names_count", schema_names.len());

        schema_names
    }

    #[tracing::instrument(name = "CachingCatalog::schema", level = "debug", skip(self))]
    #[allow(clippy::as_conversions)]
    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        if let Some(schema) = self.schemas_cache.get(name) {
            Some(Arc::clone(schema.value()) as Arc<dyn SchemaProvider>)
        } else if let Some(schema) = self.lookup_schema_provider(name) {
            let caching_schema = Arc::new(CachingSchema {
                name: name.to_string(),
                schema: Arc::clone(&schema),
                tables_cache: DashMap::new(),
                iceberg_catalog: self.iceberg_catalog.clone(),
            });

            self.schemas_cache
                .insert(name.to_string(), Arc::clone(&caching_schema));
            Some(caching_schema as Arc<dyn SchemaProvider>)
        } else {
            None
        }
    }

    #[tracing::instrument(
        name = "CachingCatalog::register_schema",
        level = "debug",
        skip(self),
        fields(schemas_names_count, catalog_name=format!("{:?}", self.name)),
    )]
    fn register_schema(
        &self,
        name: &str,
        schema: Arc<dyn SchemaProvider>,
    ) -> datafusion_common::Result<Option<Arc<dyn SchemaProvider>>> {
        let schema_provider = if let Some(catalog) = &self.iceberg_catalog {
            let namespace = Namespace::try_new(std::slice::from_ref(&name.to_string()))
                .map_err(|err| DataFusionError::External(Box::new(err)))?;

            let schema_provider: Arc<dyn SchemaProvider> = match self.catalog_type {
                CatalogType::Embucket | CatalogType::Memory => {
                    let metastore = self
                        .metastore
                        .clone()
                        .context(df_error::MetastoreIsMissingSnafu)?;
                    Arc::new(EmbucketSchema {
                        database: self.name.clone(),
                        schema: name.to_string(),
                        metastore: Arc::clone(&metastore),
                        iceberg_catalog: catalog.clone(),
                    })
                }
                CatalogType::S3tables => {
                    let Some(iceberg_catalog) =
                        self.catalog.as_any().downcast_ref::<IcebergCatalog>()
                    else {
                        return Err(DataFusionError::Plan(format!(
                            "Catalog {} is not an Iceberg catalog.",
                            self.name
                        )));
                    };
                    Arc::new(IcebergSchema::new(
                        namespace.clone(),
                        iceberg_catalog.mirror(),
                    ))
                }
            };
            let catalog = catalog.clone();
            block_on_without_deadlock(async move {
                tracing::info!("create_namespace");
                let c = catalog
                    .create_namespace(&namespace, None)
                    .await
                    .context(df_error::IcebergSnafu);
                tracing::info!("create_namespace result: {c:?}");
                c
            })?;
            schema_provider
        } else {
            return self.catalog.register_schema(name, schema);
        };

        let caching_schema = Arc::new(CachingSchema {
            name: name.to_string(),
            schema: schema_provider,
            tables_cache: DashMap::new(),
            iceberg_catalog: self.iceberg_catalog.clone(),
        });
        self.schemas_cache
            .insert(name.to_string(), Arc::clone(&caching_schema));
        Ok(Some(caching_schema))
    }

    #[tracing::instrument(
        name = "CachingCatalog::deregister_schema",
        level = "debug",
        skip(self),
        fields(schemas_names_count, catalog_name=format!("{:?}", self.name)),
    )]
    fn deregister_schema(
        &self,
        name: &str,
        cascade: bool,
    ) -> datafusion_common::Result<Option<Arc<dyn SchemaProvider>>> {
        let schema = self.schemas_cache.remove(name);

        if let Some(catalog) = &self.iceberg_catalog {
            let namespace = Namespace::try_new(std::slice::from_ref(&name.to_string()))
                .map_err(|err| DataFusionError::External(Box::new(err)))?;
            let catalog = catalog.clone();
            block_on_without_deadlock(async move {
                tracing::info!("drop_namespace");
                let c = catalog
                    .drop_namespace(&namespace)
                    .await
                    .context(df_error::IcebergSnafu);
                tracing::info!("drop_namespace result: {c:?}");
                c
            })?;
        } else {
            return self.catalog.deregister_schema(name, cascade);
        }
        if let Some((_, caching_schema)) = schema {
            return Ok(Some(caching_schema));
        }
        Ok(None)
    }
}
