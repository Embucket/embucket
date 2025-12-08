use arrow_schema::SchemaRef;
use catalog::utils::normalize_schema_case;
use datafusion::arrow::datatypes::Schema as ArrowSchema;
use datafusion::datasource::physical_plan::{
    FileScanConfig, FileScanConfigBuilder, FileSource, ParquetSource,
};
use datafusion::datasource::schema_adapter::{
    DefaultSchemaAdapterFactory, SchemaAdapter, SchemaAdapterFactory, SchemaMapper,
};
use datafusion::datasource::source::DataSourceExec;
use datafusion::error::Result as DFResult;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion_common::config::ConfigOptions;
use datafusion_common::tree_node::{Transformed, TransformedResult, TreeNode};
use datafusion_physical_plan::ExecutionPlan;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct CaseInsensitiveSchemaDataSourceExec;

impl CaseInsensitiveSchemaDataSourceExec {
    pub const fn new() -> Self {
        Self
    }
}

/// The rule which use schema adapter factory that normalizes file field names to lowercase
/// before delegating to the default adapter, ensuring case-insensitive mapping
/// between table schema and physical Parquet files.
impl PhysicalOptimizerRule for CaseInsensitiveSchemaDataSourceExec {
    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        plan.transform_up(|plan| {
            if let Some(source_exec) = plan.as_any().downcast_ref::<DataSourceExec>()
                && let Some(config) = source_exec
                    .data_source()
                    .as_any()
                    .downcast_ref::<FileScanConfig>()
                && let Some(parquet_source) =
                    config.file_source.as_any().downcast_ref::<ParquetSource>()
                && !config
                    .file_schema
                    .fields()
                    .iter()
                    .any(|field| field.name().eq(&field.name().to_ascii_uppercase()))
            {
                let schema_adapter_factory: Arc<dyn SchemaAdapterFactory> =
                    Arc::new(CaseInsensitiveSchemaAdapterFactory);

                let file_source =
                    parquet_source.with_schema_adapter_factory(schema_adapter_factory)?;

                let data_source = Arc::new(
                    FileScanConfigBuilder::from(config.clone())
                        .with_source(file_source)
                        .build(),
                );

                let plan = Arc::new(source_exec.clone().with_data_source(data_source));
                return Ok(Transformed::yes(plan));
            }

            Ok(Transformed::no(plan))
        })
        .data()
    }

    fn name(&self) -> &'static str {
        "CaseInsensitiveSchemaDataSourceExec"
    }

    fn schema_check(&self) -> bool {
        true
    }
}

/// A schema adapter factory that normalizes file field names to lowercase
/// before delegating to the default adapter, ensuring case-insensitive mapping
/// between table schema and physical Parquet files.
#[derive(Debug, Default)]
struct CaseInsensitiveSchemaAdapterFactory;

impl SchemaAdapterFactory for CaseInsensitiveSchemaAdapterFactory {
    fn create(
        &self,
        projected_table_schema: SchemaRef,
        _file_schema: SchemaRef,
    ) -> Box<dyn SchemaAdapter> {
        Box::new(CaseInsensitiveSchemaAdapter {
            inner: DefaultSchemaAdapterFactory::from_schema(projected_table_schema),
        })
    }
}

struct CaseInsensitiveSchemaAdapter {
    inner: Box<dyn SchemaAdapter>,
}

impl std::fmt::Debug for CaseInsensitiveSchemaAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaseInsensitiveSchemaAdapter").finish()
    }
}

impl SchemaAdapter for CaseInsensitiveSchemaAdapter {
    fn map_column_index(&self, index: usize, file_schema: &ArrowSchema) -> Option<usize> {
        let normalized = normalize_schema_case(file_schema);
        self.inner.map_column_index(index, &normalized)
    }

    fn map_schema(
        &self,
        file_schema: &ArrowSchema,
    ) -> DFResult<(Arc<dyn SchemaMapper>, Vec<usize>)> {
        let normalized = normalize_schema_case(file_schema);
        self.inner.map_schema(&normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::listing::PartitionedFile;
    use datafusion::datasource::object_store::ObjectStoreUrl;
    use datafusion::datasource::physical_plan::{FileGroup, FileScanConfigBuilder};
    use datafusion_common::config::TableParquetOptions;

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![Field::new("ID", DataType::Int32, false)]))
    }

    #[tokio::test]
    async fn test_sets_schema_adapter_on_parquet_source() -> DFResult<()> {
        let object_store_url = ObjectStoreUrl::parse("s3://bucket")?;
        let file_source = Arc::new(ParquetSource::new(TableParquetOptions::default()));
        let file_scan_config = Arc::new(
            FileScanConfigBuilder::new(object_store_url, schema(), file_source)
                .with_file_groups(vec![FileGroup::new(vec![PartitionedFile::new("path", 1)])])
                .build(),
        );

        let data_source_exec = Arc::new(DataSourceExec::new(file_scan_config));
        let rule = CaseInsensitiveSchemaDataSourceExec::new();
        let optimized = rule.optimize(data_source_exec, &ConfigOptions::default())?;

        let data_source_exec = optimized
            .as_any()
            .downcast_ref::<DataSourceExec>()
            .expect("expected DataSourceExec");

        let file_scan_config = data_source_exec
            .data_source()
            .as_any()
            .downcast_ref::<FileScanConfig>()
            .expect("expected FileScanConfig");

        let parquet_source = file_scan_config
            .file_source
            .as_any()
            .downcast_ref::<ParquetSource>()
            .expect("expected ParquetSource");

        assert!(parquet_source.schema_adapter_factory().is_some());

        Ok(())
    }
}
