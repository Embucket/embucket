use super::schema::EmbucketSchema;
use crate::catalog::CATALOG_TIMEOUT;
use crate::{block_on_with_timeout, error};
use catalog_metastore::{Metastore, SchemaIdent};
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use iceberg_rust::catalog::Catalog as IcebergCatalog;
use snafu::ResultExt;
use std::{any::Any, sync::Arc};
use tracing::error;

pub struct EmbucketCatalog {
    pub database: String,
    pub metastore: Arc<dyn Metastore>,
    pub iceberg_catalog: Arc<dyn IcebergCatalog>,
}

impl EmbucketCatalog {
    pub fn new(
        database: String,
        metastore: Arc<dyn Metastore>,
        iceberg_catalog: Arc<dyn IcebergCatalog>,
    ) -> Self {
        Self {
            database,
            metastore,
            iceberg_catalog,
        }
    }

    #[must_use]
    pub fn catalog(&self) -> Arc<dyn IcebergCatalog> {
        self.iceberg_catalog.clone()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for EmbucketCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DFCatalog")
            .field("database", &self.database)
            .field("iceberg_catalog", &"")
            .finish()
    }
}

impl CatalogProvider for EmbucketCatalog {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[tracing::instrument(name = "EmbucketCatalog::schema_names", level = "debug", skip(self))]
    fn schema_names(&self) -> Vec<String> {
        let metastore = self.metastore.clone();
        let database = self.database.clone();

        #[allow(clippy::expect_used)]
        block_on_with_timeout(
            async move {
                metastore
                    .list_schemas(&database)
                    .await
                    .map(|schemas| {
                        schemas
                            .into_iter()
                            .map(|s| s.ident.schema.clone())
                            .collect()
                    })
                    .context(error::MetastoreSnafu)
            },
            CATALOG_TIMEOUT,
        )
        .expect("Catalog timeout on: list_schemas")
        .unwrap_or_else(|error| {
            error!(
                ?error,
                "Failed to list Iceberg namespaces; returning empty list"
            );
            vec![]
        })
    }

    #[tracing::instrument(name = "EmbucketCatalog::schema", level = "debug", skip(self))]
    fn schema(&self, name: &str) -> Option<Arc<dyn SchemaProvider>> {
        let metastore = self.metastore.clone();
        let iceberg_catalog = self.iceberg_catalog.clone();
        let database = self.database.clone();
        let schema_name = name.to_string();

        #[allow(clippy::expect_used)]
        block_on_with_timeout(
            async move {
                let schema_opt = metastore
                    .get_schema(&SchemaIdent::new(database.clone(), schema_name.clone()))
                    .await
                    .context(error::MetastoreSnafu)?;

                let provider = schema_opt.map(|_| {
                    let schema: Arc<dyn SchemaProvider> = Arc::new(EmbucketSchema {
                        database,
                        schema: schema_name,
                        metastore,
                        iceberg_catalog,
                    });
                    schema
                });
                Ok(provider)
            },
            CATALOG_TIMEOUT,
        )
        .expect("Catalog timeout on: get_schema")
        .unwrap_or_else(|error: error::Error| {
            error!(?error, "Failed to get schema; assuming missing");
            None
        })
    }
}
