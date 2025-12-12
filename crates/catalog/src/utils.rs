use datafusion::arrow::datatypes::{Schema, SchemaRef};
use datafusion::catalog::{SchemaProvider, TableProvider};
use datafusion_common::Result as DataFusionResult;
use datafusion_common::tree_node::{Transformed, TransformedResult, TreeNode};
use datafusion_expr::Expr;
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

#[must_use]
pub fn normalize_schema_case(schema: &Schema) -> Schema {
    let fields = schema
        .fields()
        .iter()
        .map(|field| {
            let mut cloned = field.as_ref().clone();
            cloned.set_name(field.name().to_ascii_lowercase());
            cloned
        })
        .collect::<Vec<_>>();
    Schema::new(fields)
}

pub fn rewrite_expr_case(schema: &Schema, expr: Expr) -> datafusion_common::Result<Expr> {
    expr.transform_up(|e| {
        if let Expr::Column(col) = &e {
            let lookup = schema
                .fields()
                .iter()
                .find(|field| field.name().eq_ignore_ascii_case(&col.name));

            if let Some(field) = lookup {
                let mut updated = col.clone();
                updated.name.clone_from(field.name());
                return Ok(Transformed::yes(Expr::Column(updated)));
            }
        }
        Ok(Transformed::no(e))
    })
    .data()
}
#[must_use]
pub fn case_sensitive_schema(schema: &SchemaRef) -> bool {
    schema
        .fields()
        .iter()
        .any(|field| field.name().eq(&field.name().to_ascii_uppercase()))
}
