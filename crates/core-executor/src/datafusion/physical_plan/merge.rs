use datafusion::arrow::{
    array::{Array, BooleanArray, RecordBatch, StringArray, downcast_array},
    compute::{filter, kernels::cmp::distinct},
    datatypes::Schema,
};
use datafusion_common::{DFSchemaRef, DataFusionError, HashSet};
use datafusion_iceberg::{DataFusionTable, error::Error as DataFusionIcebergError};
use datafusion_physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PhysicalExpr, PlanProperties, RecordBatchStream,
    SendableRecordBatchStream, expressions::Column, projection::ProjectionExec,
    stream::RecordBatchStreamAdapter,
};
use futures::{Stream, StreamExt, TryStreamExt};
use iceberg_rust::{
    arrow::write::write_parquet_partitioned, catalog::tabular::Tabular,
    error::Error as IcebergError,
};
use pin_project_lite::pin_project;
use std::{sync::Arc, task::Poll};

static SOURCE_EXISTS_COLUMN: &str = "__source_exists";
static DATA_FILE_PATH_COLUMN: &str = "__data_file_path";
static MANIFEST_FILE_PATH_COLUMN: &str = "__manfiest_file_path";

#[derive(Debug)]
pub struct MergeIntoSinkExec {
    schema: DFSchemaRef,
    input: Arc<dyn ExecutionPlan>,
    target: DataFusionTable,
    properties: PlanProperties,
}

impl MergeIntoSinkExec {
    pub fn new(
        schema: DFSchemaRef,
        input: Arc<dyn ExecutionPlan>,
        target: DataFusionTable,
    ) -> Self {
        let properties = input.properties().clone();
        Self {
            schema,
            input,
            target,
            properties,
        }
    }
}

impl DisplayAs for MergeIntoSinkExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default
            | DisplayFormatType::Verbose
            | DisplayFormatType::TreeRender => {
                write!(f, "MergeIntoSinkExec")
            }
        }
    }
}

impl ExecutionPlan for MergeIntoSinkExec {
    fn name(&self) -> &'static str {
        "MergeIntoSinkExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion_common::Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(
                "MergeIntoSinkExec requires exactly one child".to_string(),
            ));
        }
        Ok(Arc::new(MergeIntoSinkExec::new(
            self.schema.clone(),
            children[0].clone(),
            self.target.clone(),
        )))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion_common::Result<datafusion_physical_plan::SendableRecordBatchStream> {
        let schema = Arc::new(self.schema.as_arrow().clone());

        // Filter out rows whoose __data_file_path doesn't have a matching row
        let filtered: Arc<dyn ExecutionPlan> =
            Arc::new(SourceExistFilterExec::new(self.input.clone()));

        // Remove auxiliary columns
        let projection = ProjectionExec::try_new(schema_projection(&schema), filtered)?;

        let batches = projection.execute(partition, context)?;

        let stream = futures::stream::once({
            let tabular = self.target.tabular.clone();
            let branch = self.target.branch.clone();
            let schema = schema.clone();
            async move {
                let mut lock = tabular.write().await;
                let table = if let Tabular::Table(table) = &mut *lock {
                    Ok(table)
                } else {
                    Err(IcebergError::InvalidFormat("database entity".to_string()))
                }
                .map_err(DataFusionIcebergError::from)?;

                // Write recordbatches into parquet files on object-storage
                let datafiles = write_parquet_partitioned(
                    table,
                    batches.map_err(Into::into),
                    branch.as_deref(),
                )
                .await?;

                // Commit transaction on Iceberg table
                table
                    .new_transaction(branch.as_deref())
                    .append_data(datafiles)
                    .commit()
                    .await
                    .map_err(DataFusionIcebergError::from)?;

                //TODO
                // remove files to be overwritten from iceberg metadata

                Ok(RecordBatch::new_empty(schema))
            }
        })
        .boxed();

        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
    }
}

#[derive(Debug)]
struct SourceExistFilterExec {
    input: Arc<dyn ExecutionPlan>,
    properties: PlanProperties,
}

impl SourceExistFilterExec {
    fn new(input: Arc<dyn ExecutionPlan>) -> Self {
        let properties = input.properties().clone();
        Self { input, properties }
    }
}

impl DisplayAs for SourceExistFilterExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default
            | DisplayFormatType::Verbose
            | DisplayFormatType::TreeRender => {
                write!(f, "SourceExistFilterExec")
            }
        }
    }
}

impl ExecutionPlan for SourceExistFilterExec {
    fn name(&self) -> &str {
        "SourceExistFilterExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion_common::Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(
                "SourceExistFilterExec requires exactly one child".to_string(),
            ));
        }
        Ok(Arc::new(SourceExistFilterExec::new(children[0].clone())))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion_common::Result<SendableRecordBatchStream> {
        Ok(Box::pin(SourceExistFilterStream::new(
            self.input.execute(partition, context)?,
        )))
    }
}

// Filters a stream of Recordbatches by whether the value in the "__data_file_column" already had
// a row where the "__source_exists" column equals "true".
pin_project! {
    pub struct SourceExistFilterStream {
        // Files which already encountered a "__source_exists" = true value
        matching_files: HashSet<String>,
        // The current datafiles being processed
        current_file: Option<String>,
        // If the current datafiles hasn't had any rows with "__source_exists" = true, this is used
        // to buffer the recordbatches. If a "__source_exists" = true is encountered in a later
        // recordbatch, the buffered recordbatches will be returned. If the `current_file` is set
        // to another value, the buffer can be discarded as there won't be more any records from
        // the same file.
        current_buffer: Vec<RecordBatch>,
        // RecordBatches ready to be consumed
        ready_batches: Vec<RecordBatch>,

        #[pin]
        input: SendableRecordBatchStream
    }
}

impl SourceExistFilterStream {
    fn new(input: SendableRecordBatchStream) -> Self {
        Self {
            matching_files: HashSet::new(),
            current_file: None,
            current_buffer: Vec::new(),
            ready_batches: Vec::new(),
            input,
        }
    }
}

impl Stream for SourceExistFilterStream {
    type Item = Result<RecordBatch, DataFusionError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let project = self.project();

        // Return early if a batch is ready
        if let Some(batch) = project.ready_batches.pop() {
            return Poll::Ready(Some(Ok(batch)));
        }

        match project.input.poll_next(cx) {
            Poll::Ready(Some(Ok(batch))) => {
                let schema = batch.schema();

                let source_exists_index = schema.index_of(SOURCE_EXISTS_COLUMN)?;
                let data_file_path_index = schema.index_of(DATA_FILE_PATH_COLUMN)?;
                let manifest_file_path_index = schema.index_of(MANIFEST_FILE_PATH_COLUMN)?;

                let source_exists = batch.column(source_exists_index);
                let data_file_path = batch.column(data_file_path_index);
                let manifest_file_path = batch.column(manifest_file_path_index);

                let required_data_files = filter(
                    &data_file_path,
                    &downcast_array::<BooleanArray>(source_exists),
                )?;

                let unique_data_files = unique_values(&required_data_files)?;

                let required_manifest_files = filter(
                    &manifest_file_path,
                    &downcast_array::<BooleanArray>(source_exists),
                )?;

                let _unique_manifest_files = unique_values(&required_manifest_files)?;

                if unique_data_files.is_empty() {
                    Poll::Pending
                } else {
                    Poll::Ready(Some(Ok(batch)))
                }
            }
            x => x,
        }
    }
}

impl RecordBatchStream for SourceExistFilterStream {
    fn schema(&self) -> datafusion::arrow::datatypes::SchemaRef {
        self.input.schema()
    }
}

// Computes a HashSet of all string values in the array
fn unique_values(array: &dyn Array) -> Result<HashSet<String>, DataFusionError> {
    let first = downcast_array::<StringArray>(array).value(0).to_owned();

    let slice_len = array.len() - 1;

    if slice_len == 0 {
        return Ok(HashSet::from_iter([first]));
    }

    let v1 = array.slice(0, slice_len);
    let v2 = array.slice(1, slice_len);

    let mask = distinct(&v1, &v2)?;

    let unique = filter(&v2, &mask)?;

    let strings = downcast_array::<StringArray>(&unique);

    let result = strings
        .iter()
        .fold(HashSet::from_iter([first]), |mut acc, x| {
            if let Some(x) = x {
                acc.insert(x.to_owned());
            }
            acc
        });

    Ok(result)
}

// Remove auxiliary columns from schema
fn schema_projection(schema: &Schema) -> Vec<(Arc<dyn PhysicalExpr>, String)> {
    schema
        .fields()
        .iter()
        .enumerate()
        .filter_map(|(i, field)| {
            let name = field.name();
            if name != SOURCE_EXISTS_COLUMN
                && name != DATA_FILE_PATH_COLUMN
                && name != MANIFEST_FILE_PATH_COLUMN
            {
                Some((
                    Arc::new(Column::new(name, i)) as Arc<dyn PhysicalExpr>,
                    name.to_owned(),
                ))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_unique_values_with_duplicates() {
        let array = Arc::new(StringArray::from(vec!["a", "a", "b", "b", "c"]));
        let result = unique_values(array.as_ref()).unwrap();

        let expected: HashSet<String> = ["a", "b", "c"].iter().map(|&s| s.to_string()).collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_unique_values_all_same() {
        let array = Arc::new(StringArray::from(vec!["same", "same", "same"]));
        let result = unique_values(array.as_ref()).unwrap();

        let expected: HashSet<String> = ["same"].iter().map(|&s| s.to_string()).collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_unique_values_with_nulls() {
        let array = Arc::new(StringArray::from(vec![
            Some("a"),
            None,
            Some("b"),
            None,
            Some("a"),
        ]));
        let result = unique_values(array.as_ref()).unwrap();

        let expected: HashSet<String> = ["a", "b"].iter().map(|&s| s.to_string()).collect();

        assert_eq!(result, expected);
    }
}
