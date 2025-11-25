use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion_common::Result as DataFusionResult;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tracing::warn;

/// Fetch the table providers of a schema with bounded concurrency.
///
/// Tables that fail to resolve (e.g. removed mid-refresh) are skipped, while
/// errors from the underlying schema provider are propagated.
#[allow(clippy::type_complexity)]
pub async fn fetch_table_providers(
    schema_provider: Arc<dyn SchemaProvider>,
    max_concurrent_fetches: usize,
) -> DataFusionResult<Vec<(String, Arc<dyn TableProvider>)>> {
    let concurrency = max_concurrent_fetches.max(1);
    let table_names = schema_provider.table_names();

    let results: Vec<DataFusionResult<Option<(String, Arc<dyn TableProvider>)>>> =
        stream::iter(table_names.into_iter())
            .map(|table_name| {
                let schema_provider = Arc::clone(&schema_provider);
                async move {
                    match schema_provider.table(&table_name).await {
                        Ok(table) => Ok(table.map(|table_provider| (table_name, table_provider))),
                        Err(err) => {
                            warn!(
                                table = %table_name,
                                error = %err,
                                "Failed to fetch table provider; skipping table"
                            );
                            Ok(None)
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

    let mut tables = Vec::new();
    for result in results {
        if let Some(table) = result? {
            tables.push(table);
        }
    }

    Ok(tables)
}
