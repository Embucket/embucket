use crate::snowflake_table::CaseInsensitiveTable;
use crate::table::{CachingTable, IcebergTableBuilder};
use crate::{block_on_without_deadlock, df_error};
use async_trait::async_trait;
use dashmap::DashMap;
use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion_common::DataFusionError;
use datafusion_expr::TableType;
use datafusion_iceberg::DataFusionTable;
use iceberg_rust::catalog::Catalog;
use iceberg_rust::catalog::tabular::Tabular as IcebergTabular;
use iceberg_rust_spec::identifier::Identifier;
use iceberg_rust_spec::namespace::Namespace;
use snafu::ResultExt;
use std::any::Any;
use std::sync::Arc;

pub struct CachingSchema {
    pub schema: Arc<dyn SchemaProvider>,
    pub iceberg_catalog: Option<Arc<dyn Catalog>>,
    pub name: String,
    pub tables_cache: DashMap<String, Arc<CachingTable>>,
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for CachingSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schema")
            .field("schema", &"")
            .field("name", &self.name)
            .field("tables_cache", &self.tables_cache)
            .finish()
    }
}

#[async_trait]
impl SchemaProvider for CachingSchema {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        match &self.iceberg_catalog {
            Some(catalog) => {
                let catalog = catalog.clone();
                let Ok(namespace) = Namespace::try_new(std::slice::from_ref(&self.name)) else {
                    return vec![];
                };
                block_on_without_deadlock(async move {
                    catalog
                        .list_tabulars(&namespace)
                        .await
                        .map(|tables| {
                            tables
                                .into_iter()
                                .map(|identifier| identifier.name().to_owned())
                                .collect()
                        })
                        .unwrap_or_default()
                })
            }
            None => self.schema.table_names(),
        }
    }

    #[allow(clippy::as_conversions)]
    #[tracing::instrument(name = "CachingSchema::table", level = "debug", skip(self), err)]
    async fn table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>, DataFusionError> {
        // NOTE: We should always rely on the original schema provider instead of the cache,
        // because the underlying Iceberg catalog may have updated the table metadata outside
        // of SQL (e.g., via direct catalog API calls). In such cases, our cache could contain
        // stale metadata and ignore the latest snapshot updates.
        //
        // However, since we assume that users will interact with the Iceberg catalog
        // exclusively through Embucket, we can safely enable caching — in this case,
        // the data will remain consistent across all queries.
        if let Some(table) = self.tables_cache.get(name) {
            return Ok(Some(Arc::clone(table.value()) as Arc<dyn TableProvider>));
        }

        if let Some(table) = self.schema.table(name).await? {
            let caching_table = Arc::new(CachingTable::new(name.to_string(), Arc::clone(&table)));

            // Optionally update the cache for reuse (not as source of truth)
            self.tables_cache
                .insert(name.to_string(), Arc::clone(&caching_table));

            Ok(Some(caching_table as Arc<dyn TableProvider>))
        } else {
            Ok(None)
        }
    }

    fn register_table(
        &self,
        name: String,
        table: Arc<dyn TableProvider>,
    ) -> datafusion_common::Result<Option<Arc<dyn TableProvider>>> {
        let table_provider: Arc<dyn TableProvider> = if let Some(catalog) = &self.iceberg_catalog
            && let Some(iceberg_builder) = table.as_any().downcast_ref::<IcebergTableBuilder>()
            && table.table_type() != TableType::View
        {
            let catalog = Arc::clone(catalog);
            let mut builder = iceberg_builder.builder.clone();
            let namespace = vec![self.name.clone()];
            let table_name = name.clone();

            block_on_without_deadlock(async move {
                let ident = Identifier::new(&namespace, &table_name);
                let iceberg_table = builder
                    .build(ident.namespace(), catalog)
                    .await
                    .context(df_error::IcebergSnafu)?;
                let tabular = IcebergTabular::Table(iceberg_table);
                let table_provider: Arc<dyn TableProvider> = Arc::new(CaseInsensitiveTable::new(
                    Arc::new(DataFusionTable::new(tabular, None, None, None)),
                ));
                Ok::<Arc<dyn TableProvider>, DataFusionError>(table_provider)
            })?
        } else {
            return self.schema.register_table(name, Arc::clone(&table));
        };

        let caching_table = Arc::new(CachingTable::new(name.clone(), Arc::clone(&table_provider)));
        self.tables_cache.insert(name, caching_table);
        Ok(Some(table_provider))
    }

    #[allow(clippy::as_conversions)]
    fn deregister_table(
        &self,
        name: &str,
    ) -> datafusion_common::Result<Option<Arc<dyn TableProvider>>> {
        let table = self.tables_cache.remove(name);

        if let Some((_, caching_table)) = table {
            if caching_table.table_type() != TableType::View {
                if let Some(catalog) = &self.iceberg_catalog {
                    let catalog = Arc::clone(catalog);
                    let namespace = vec![self.name.clone()];
                    let table_name = name.to_string();

                    block_on_without_deadlock(async move {
                        let ident = Identifier::new(&namespace, &table_name);
                        catalog
                            .drop_table(&ident)
                            .await
                            .context(df_error::IcebergSnafu)
                    })?;
                } else {
                    return self.schema.deregister_table(name);
                }
            }
            return Ok(Some(caching_table as Arc<dyn TableProvider>));
        }
        Ok(None)
    }

    fn table_exist(&self, name: &str) -> bool {
        self.tables_cache.contains_key(name)
    }
}
