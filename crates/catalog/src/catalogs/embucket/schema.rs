use crate::snowflake_table::CaseInsensitiveTable;
use crate::{block_in_new_runtime, error};
use async_trait::async_trait;
use catalog_metastore::error as metastore_error;
use catalog_metastore::{Metastore, SchemaIdent, TableIdent};
use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion_common::DataFusionError;
use datafusion_iceberg::DataFusionTable as IcebergDataFusionTable;
use iceberg_rust::catalog::Catalog as IcebergCatalog;
use iceberg_rust::{catalog::tabular::Tabular as IcebergTabular, table::Table as IcebergTable};
use snafu::ResultExt;
use std::any::Any;
use std::sync::Arc;
use tracing::error;

pub struct EmbucketSchema {
    pub database: String,
    pub schema: String,
    pub metastore: Arc<dyn Metastore>,
    pub iceberg_catalog: Arc<dyn IcebergCatalog>,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for EmbucketSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DFSchema")
            .field("database", &self.database)
            .field("schema", &self.schema)
            .field("metastore", &"")
            .field("iceberg_catalog", &"")
            .finish()
    }
}

#[async_trait]
impl SchemaProvider for EmbucketSchema {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[tracing::instrument(
        name = "EmbucketSchema::table_names",
        level = "debug",
        skip(self),
        fields(tables_names_count, schema_name=format!("{}.{}", self.database, self.schema))
    )]
    fn table_names(&self) -> Vec<String> {
        let metastore = self.metastore.clone();
        let database = self.database.clone();
        let schema = self.schema.clone();

        let table_names = block_in_new_runtime(async move {
            metastore
                .list_tables(&SchemaIdent::new(database, schema))
                .await
                .map(|tables| tables.into_iter().map(|s| s.ident.table.clone()).collect())
                .context(error::MetastoreSnafu)
        })
        .unwrap_or_else(|error| {
            error!(?error, "Failed to list tables; returning empty list");
            vec![]
        });
        // Record the result as part of the current span.
        tracing::Span::current().record("tables_names_count", table_names.len());

        table_names
    }

    #[tracing::instrument(name = "EmbucketSchema::table", level = "debug", skip(self), err)]
    async fn table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>, DataFusionError> {
        let ident = &TableIdent::new(&self.database.clone(), &self.schema.clone(), name);
        let object_store = self
            .metastore
            .table_object_store(ident)
            .await
            .map_err(|e| DataFusionError::External(Box::new(e)))?
            .ok_or_else(|| {
                DataFusionError::External(Box::new(
                    metastore_error::TableObjectStoreNotFoundSnafu {
                        table: ident.table.clone(),
                        schema: ident.schema.clone(),
                        db: ident.database.clone(),
                    }
                    .build(),
                ))
            })?;
        match self.metastore.get_table(ident).await {
            Ok(Some(table)) => {
                let iceberg_table = IcebergTable::new(
                    ident.to_iceberg_ident(),
                    self.iceberg_catalog.clone(),
                    object_store,
                    table.metadata.clone(),
                )
                .await
                .map_err(|e| DataFusionError::External(Box::new(e)))?;
                let tabular = IcebergTabular::Table(iceberg_table);

                let table_provider: Arc<dyn TableProvider> = Arc::new(CaseInsensitiveTable::new(
                    Arc::new(IcebergDataFusionTable::new(tabular, None, None, None)),
                ));
                Ok(Some(table_provider))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(DataFusionError::External(Box::new(e))),
        }
    }

    #[tracing::instrument(name = "EmbucketSchema::table_exist", level = "debug", skip(self))]
    fn table_exist(&self, name: &str) -> bool {
        let iceberg_catalog = self.iceberg_catalog.clone();
        let database = self.database.clone();
        let schema = self.schema.clone();
        let table = name.to_string();
        let ident = TableIdent::new(&database, &schema, &table);
        let ident_for_runtime = ident.clone();

        block_in_new_runtime(async move {
            iceberg_catalog
                .tabular_exists(&ident_for_runtime.to_iceberg_ident())
                .await
                .context(error::IcebergSnafu)
        })
        .unwrap_or_else(|error| {
            error!(
                ?error,
                schema_name = %ident.schema,
                table_name = %ident.table,
                "Failed to check table existence; assuming missing",
            );
            false
        })
    }
}
