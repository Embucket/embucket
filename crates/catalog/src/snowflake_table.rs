use async_trait::async_trait;
use datafusion::arrow::datatypes::{Schema, SchemaRef};
use datafusion::catalog::{Session, TableProvider};
use datafusion_common::tree_node::{Transformed, TransformedResult, TreeNode};
use datafusion_common::{Statistics, project_schema};
use datafusion_expr::dml::InsertOp;
use datafusion_expr::{Expr, TableProviderFilterPushDown, TableType};
use datafusion_physical_plan::expressions::Column;
use datafusion_physical_plan::projection::ProjectionExec;
use datafusion_physical_plan::{ExecutionPlan, PhysicalExpr};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

/// A [`TableProvider`] adapter that rewrites column names to match Snowflake's
/// case-insensitive semantics.
///
/// Snowflake stores schemas in uppercase, while queries are typically written
/// in lowercase. `DataFusion` treats identifiers as case-sensitive, which causes
/// mismatches between the logical projection expressions (lowercase) and the
/// physical input schema (uppercase). This adapter:
///
/// - Rewrites column references in filter expressions to the original schema
///   casing before delegating to the underlying table provider.
/// - Wraps the produced physical plan in a projection that aliases columns to
///   lowercase names, so the output schema matches the logical expectations.
#[derive(Debug)]
pub struct CaseInsensitiveTable {
    inner: Arc<dyn TableProvider>,
    original_schema: SchemaRef,
    normalized_schema: SchemaRef,
    requires_case_rewrite: bool,
}

impl CaseInsensitiveTable {
    pub fn new(inner: Arc<dyn TableProvider>) -> Self {
        let original_schema = inner.schema();
        let requires_case_rewrite = original_schema
            .fields()
            .iter()
            .any(|field| field.name().eq(&field.name().to_ascii_uppercase()));
        let normalized_schema = Arc::new(normalize_schema_case(&original_schema));
        Self {
            inner,
            original_schema,
            normalized_schema,
            requires_case_rewrite,
        }
    }

    fn rewrite_expr(&self, expr: Expr) -> datafusion_common::Result<Expr> {
        expr.transform_up(|e| {
            if let Expr::Column(col) = &e {
                let lookup = self
                    .original_schema
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
}

#[async_trait]
impl TableProvider for CaseInsensitiveTable {
    fn as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }

    fn schema(&self) -> SchemaRef {
        Arc::clone(&self.normalized_schema)
    }

    fn table_type(&self) -> TableType {
        self.inner.table_type()
    }

    #[allow(clippy::as_conversions)]
    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> datafusion_common::Result<Arc<dyn ExecutionPlan>> {
        if !self.requires_case_rewrite {
            return self.inner.scan(state, projection, filters, limit).await;
        }

        let rewritten_filters = filters
            .iter()
            .map(|expr| self.rewrite_expr(expr.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let plan = self
            .inner
            .scan(state, projection, &rewritten_filters, limit)
            .await?;

        let target_schema = if let Some(indices) = projection {
            project_schema(&self.normalized_schema, Some(indices))?
        } else {
            Arc::clone(&self.normalized_schema)
        };

        let mut projection_exprs = Vec::with_capacity(plan.schema().fields().len());
        for (idx, field) in plan.schema().fields().iter().enumerate() {
            let target_name = target_schema.field(idx).name().clone();

            projection_exprs.push((
                Arc::new(Column::new(field.name(), idx)) as Arc<dyn PhysicalExpr>,
                target_name,
            ));
        }

        let projected_plan = ProjectionExec::try_new(projection_exprs, plan)?;
        Ok(Arc::new(projected_plan))
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> datafusion_common::Result<Vec<TableProviderFilterPushDown>> {
        if !self.requires_case_rewrite {
            return self.inner.supports_filters_pushdown(filters);
        }

        let rewritten = filters
            .iter()
            .map(|expr| self.rewrite_expr((*expr).clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let rewritten_refs = rewritten.iter().collect::<Vec<_>>();
        self.inner.supports_filters_pushdown(&rewritten_refs)
    }

    fn statistics(&self) -> Option<Statistics> {
        self.inner.statistics()
    }

    #[allow(clippy::as_conversions)]
    async fn insert_into(
        &self,
        state: &dyn Session,
        input: Arc<dyn ExecutionPlan>,
        insert_op: InsertOp,
    ) -> datafusion_common::Result<Arc<dyn ExecutionPlan>> {
        self.inner.insert_into(state, input, insert_op).await
    }
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
