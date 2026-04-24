use datafusion::{
    arrow::{
        array::{Array, ArrayRef, BooleanArray, RecordBatch, StringArray, downcast_array},
        compute::{filter, kernels::cmp::distinct},
        datatypes::Schema,
    },
    physical_expr::EquivalenceProperties,
};
use datafusion_common::{DFSchemaRef, DataFusionError};
use datafusion_iceberg::{
    DataFusionTable, error::Error as DataFusionIcebergError, table::write_parquet_data_files,
};
use datafusion_physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
    SendableRecordBatchStream,
    coalesce_partitions::CoalescePartitionsExec,
    execution_plan::{Boundedness, EmissionType},
    metrics::{Count, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet},
    stream::RecordBatchStreamAdapter,
};
use futures::{Stream, StreamExt};
use iceberg_rust::{catalog::tabular::Tabular, error::Error as IcebergError};
use pin_project_lite::pin_project;
use snafu::ResultExt;
use std::{
    collections::HashMap,
    ops::BitAnd,
    sync::atomic::{AtomicI64, Ordering},
    sync::{Arc, Mutex},
    task::Poll,
};

use crate::error;

pub(crate) static TARGET_EXISTS_COLUMN: &str = "__target_exists";
pub(crate) static SOURCE_EXISTS_COLUMN: &str = "__source_exists";
pub(crate) static DATA_FILE_PATH_COLUMN: &str = "__data_file_path";
pub(crate) static MANIFEST_FILE_PATH_COLUMN: &str = "__manifest_file_path";
pub(crate) static MERGE_UPDATED_COLUMN: &str = "__merge_row_updated";
pub(crate) static MERGE_INSERTED_COLUMN: &str = "__merge_row_inserted";

#[derive(Debug)]
pub struct MergeIntoCOWSinkExec {
    schema: DFSchemaRef,
    input: Arc<dyn ExecutionPlan>,
    target: DataFusionTable,
    properties: PlanProperties,
    /// Per-node metrics surfaced via `EXPLAIN ANALYZE`. Populated with
    /// `updated_rows` / `inserted_rows` / `deleted_rows` counters after the
    /// write transaction commits, so `EXPLAIN ANALYZE MERGE INTO …` reports
    /// how many rows each clause produced alongside the child scan metrics.
    metrics: ExecutionPlanMetricsSet,
}

impl MergeIntoCOWSinkExec {
    pub fn new(
        schema: DFSchemaRef,
        input: Arc<dyn ExecutionPlan>,
        target: DataFusionTable,
    ) -> Self {
        // MERGE operations produce a single empty record batch after completion
        let eq_properties = EquivalenceProperties::new(Arc::new((*schema.as_arrow()).clone()));
        let partitioning = Partitioning::UnknownPartitioning(1); // Single partition for sink operations
        let emission_type = EmissionType::Final; // Final emission after all processing is complete
        let boundedness = Boundedness::Bounded; // Bounded operation that completes

        let properties =
            PlanProperties::new(eq_properties, partitioning, emission_type, boundedness);
        Self {
            schema,
            input,
            target,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }
}

impl DisplayAs for MergeIntoCOWSinkExec {
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

// Map from Manifest file to contained Datafiles
type ManifestAndDataFiles = HashMap<String, Vec<String>>;

impl ExecutionPlan for MergeIntoCOWSinkExec {
    fn name(&self) -> &'static str {
        "MergeIntoCOWSinkExec"
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

    /// Surface per-clause row counts (updated / inserted / deleted) as
    /// `EXPLAIN ANALYZE` metrics. Values are populated by `execute()` after
    /// the write transaction commits; they're zero until then.
    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion_common::Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            // Using DataFusionError::External is currently not possible as it requires Sync
            return Err(DataFusionError::Internal(
                error::LogicalExtensionChildCountSnafu {
                    name: "MergeIntoCOWSinkExec".to_string(),
                    expected: 1usize,
                }
                .build()
                .to_string(),
            ));
        }
        Ok(Arc::new(Self::new(
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

        let matching_files: Arc<Mutex<Option<ManifestAndDataFiles>>> = Arc::default();
        let updated_rows: Arc<AtomicI64> = Arc::new(AtomicI64::new(0));
        let inserted_rows: Arc<AtomicI64> = Arc::new(AtomicI64::new(0));

        // `Count` metrics surfaced via `EXPLAIN ANALYZE` on this node.
        // Updated incrementally by the stream as rows flow through.
        let updated_rows_metric: Count =
            MetricBuilder::new(&self.metrics).counter("updated_rows", partition);
        let inserted_rows_metric: Count =
            MetricBuilder::new(&self.metrics).counter("inserted_rows", partition);
        // MERGE DELETE is not supported yet; register the metric so it shows as 0.
        let _deleted_rows_metric: Count =
            MetricBuilder::new(&self.metrics).counter("deleted_rows", partition);

        let coalesce = CoalescePartitionsExec::new(self.input.clone());

        let input_batches = coalesce.execute(partition, context.clone())?;
        let count_and_project_stream = MergeCOWCountAndProjectStream::new(
            input_batches,
            updated_rows.clone(),
            inserted_rows.clone(),
            updated_rows_metric.clone(),
            inserted_rows_metric.clone(),
            matching_files.clone(),
        );

        let stream = futures::stream::once({
            let tabular = self.target.tabular.clone();
            let branch = self.target.branch.clone();
            let schema = schema.clone();
            let updated_rows = Arc::clone(&updated_rows);
            let inserted_rows = Arc::clone(&inserted_rows);
            let projected_schema = count_and_project_stream.projected_schema();
            let batches: SendableRecordBatchStream = Box::pin(RecordBatchStreamAdapter::new(
                projected_schema,
                count_and_project_stream,
            ));
            async move {
                #[allow(clippy::unwrap_used)]
                let value = tabular.read().unwrap().clone();
                let mut table = match value {
                    Tabular::Table(table) => Ok(table),
                    _ => Err(IcebergError::InvalidFormat("database entity".to_string())),
                }
                .map_err(DataFusionIcebergError::from)?;

                // Write recordbatches into parquet files on object-storage
                let datafiles =
                    write_parquet_data_files(&table, batches, &context, branch.as_deref()).await?;

                let matching_files = {
                    #[allow(clippy::unwrap_used)]
                    let mut lock = matching_files.lock().unwrap();
                    lock.take().ok_or_else(|| {
                        DataFusionError::Internal(
                            error::MatchingFilesAlreadyConsumedSnafu {}
                                .build()
                                .to_string(),
                        )
                    })?
                };

                if !datafiles.is_empty() {
                    // Commit transaction on Iceberg table
                    if matching_files.is_empty() {
                        table
                            .new_transaction(branch.as_deref())
                            .append_data(datafiles)
                            .commit()
                            .await
                            .context(error::IcebergSnafu)?;
                    } else {
                        table
                            .new_transaction(branch.as_deref())
                            .overwrite(datafiles, matching_files)
                            .commit()
                            .await
                            .context(error::IcebergSnafu)?;
                    }
                }
                // Refresh the cached table with the latest snapshot so subsequent scans
                // see the results of this MERGE operation.
                #[allow(clippy::unwrap_used)]
                let mut lock = tabular.write().unwrap();
                *lock = Tabular::Table(table);
                // Return a one-row result for DML, so clients don't render "No data result" on success.
                let updated = updated_rows.load(Ordering::Relaxed);
                let inserted = inserted_rows.load(Ordering::Relaxed);
                // MERGE DELETE is not supported yet
                let deleted = 0i64;

                let arrays = schema
                    .fields()
                    .iter()
                    .map(|f| {
                        let v = match f.name().as_str() {
                            "number of rows inserted" => inserted,
                            "number of rows updated" => updated,
                            "number of rows deleted" => deleted,
                            other => {
                                return Err(DataFusionError::Internal(format!(
                                    "Unexpected MERGE result column: {other}"
                                )));
                            }
                        };
                        let a: ArrayRef =
                            Arc::new(datafusion::arrow::array::Int64Array::from(vec![v]));
                        Ok(a)
                    })
                    .collect::<Result<Vec<_>, DataFusionError>>()?;

                RecordBatch::try_new(schema.clone(), arrays).map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to build MERGE result record batch: {e}"
                    ))
                })
            }
        })
        .boxed();

        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
    }
}

pin_project! {
    /// Stream wrapper that counts per-action MERGE rows (insert/update markers), collects
    /// matching data/manifest file pairs, and projects away auxiliary merge columns before
    /// writing to data files.
    pub struct MergeCOWCountAndProjectStream {
        projection_indices: Vec<usize>,
        projected_schema: Arc<Schema>,
        updated_idx: Option<usize>,
        inserted_idx: Option<usize>,
        updated_rows: Arc<AtomicI64>,
        inserted_rows: Arc<AtomicI64>,
        updated_rows_metric: Count,
        inserted_rows_metric: Count,
        data_file_path_idx: usize,
        manifest_file_path_idx: usize,
        matching_files: HashMap<String, String>,
        matching_files_ref: Arc<Mutex<Option<ManifestAndDataFiles>>>,

        #[pin]
        input: SendableRecordBatchStream,
    }
}

impl MergeCOWCountAndProjectStream {
    fn new(
        input: SendableRecordBatchStream,
        updated_rows: Arc<AtomicI64>,
        inserted_rows: Arc<AtomicI64>,
        updated_rows_metric: Count,
        inserted_rows_metric: Count,
        matching_files_ref: Arc<Mutex<Option<ManifestAndDataFiles>>>,
    ) -> Self {
        let input_schema = input.schema();

        let updated_idx = input_schema.index_of(MERGE_UPDATED_COLUMN).ok();
        let inserted_idx = input_schema.index_of(MERGE_INSERTED_COLUMN).ok();
        let data_file_path_idx = input_schema.index_of(DATA_FILE_PATH_COLUMN).unwrap_or(0);
        let manifest_file_path_idx = input_schema
            .index_of(MANIFEST_FILE_PATH_COLUMN)
            .unwrap_or(0);

        // Drop auxiliary columns so we only write table columns to parquet
        let projection_indices: Vec<usize> = input_schema
            .fields()
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let name = f.name();
                if name != SOURCE_EXISTS_COLUMN
                    && name != DATA_FILE_PATH_COLUMN
                    && name != MANIFEST_FILE_PATH_COLUMN
                    && name != MERGE_UPDATED_COLUMN
                    && name != MERGE_INSERTED_COLUMN
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        let projected_fields = projection_indices
            .iter()
            .map(|i| input_schema.field(*i).clone())
            .collect::<Vec<_>>();

        let projected_schema = Arc::new(Schema::new(projected_fields));

        Self {
            projection_indices,
            projected_schema,
            updated_idx,
            inserted_idx,
            updated_rows,
            inserted_rows,
            updated_rows_metric,
            inserted_rows_metric,
            data_file_path_idx,
            manifest_file_path_idx,
            matching_files: HashMap::new(),
            matching_files_ref,
            input,
        }
    }

    fn projected_schema(&self) -> Arc<Schema> {
        self.projected_schema.clone()
    }
}

impl Stream for MergeCOWCountAndProjectStream {
    type Item = Result<RecordBatch, DataFusionError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut project = self.project();
        match project.input.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(batch))) => {
                // Collect unique (data_file, manifest_file) pairs
                let data_file_col = batch.column(*project.data_file_path_idx);
                let manifest_file_col = batch.column(*project.manifest_file_path_idx);
                let file_pairs =
                    unique_files_and_manifests(data_file_col.as_ref(), manifest_file_col.as_ref())?;
                project.matching_files.extend(file_pairs);

                // Count updated/inserted rows
                if let Some(updated_idx) = *project.updated_idx
                    && let Some(col) = batch.columns().get(updated_idx)
                {
                    let updated = downcast_array::<BooleanArray>(col.as_ref());
                    let count = count_true_and_valid(&updated);
                    project
                        .updated_rows
                        .fetch_add(usize_to_i64_saturating(count), Ordering::Relaxed);
                    project.updated_rows_metric.add(count);
                }
                if let Some(inserted_idx) = *project.inserted_idx
                    && let Some(col) = batch.columns().get(inserted_idx)
                {
                    let inserted = downcast_array::<BooleanArray>(col.as_ref());
                    let count = count_true_and_valid(&inserted);
                    project
                        .inserted_rows
                        .fetch_add(usize_to_i64_saturating(count), Ordering::Relaxed);
                    project.inserted_rows_metric.add(count);
                }

                // Project away auxiliary columns
                let cols = project
                    .projection_indices
                    .iter()
                    .map(|i| batch.column(*i).clone())
                    .collect::<Vec<_>>();

                let projected = RecordBatch::try_new(project.projected_schema.clone(), cols)
                    .map_err(|e| {
                        DataFusionError::Internal(format!(
                            "Failed to project MERGE record batch: {e}"
                        ))
                    })?;
                Poll::Ready(Some(Ok(projected)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                // Store matching files for the commit phase
                let matching_files = std::mem::take(project.matching_files);
                let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
                for (file, manifest) in matching_files {
                    grouped
                        .entry(manifest)
                        .and_modify(|v| v.push(file.clone()))
                        .or_insert_with(|| vec![file]);
                }
                #[allow(clippy::unwrap_used)]
                let mut lock = project.matching_files_ref.lock().unwrap();
                lock.replace(grouped);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Fast count of `true` values, treating NULL as false, using Arrow bitmaps.
#[inline]
fn count_true_and_valid(arr: &BooleanArray) -> usize {
    if arr.null_count() == 0 {
        return arr.values().count_set_bits();
    }

    if let Some(nulls) = arr.logical_nulls() {
        let valid = nulls.inner();
        return arr.values().bitand(valid).count_set_bits();
    }

    arr.values().count_set_bits()
}

#[inline]
fn usize_to_i64_saturating(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// Creates a mapping of unique file paths to their corresponding manifest paths.
///
/// This function efficiently extracts unique file-manifest pairs from two sorted arrays by
/// comparing consecutive elements. It assumes both arrays are sorted and of equal length.
/// The function:
/// 1. Takes the first file-manifest pair as a starting point
/// 2. Identifies positions where consecutive file entries differ
/// 3. Filters both arrays to keep only the distinct file-manifest pairs
/// 4. Returns a `HashMap` mapping file paths to manifest paths
///
/// # Arguments
/// * `files` - A reference to an Array containing file paths (expected to be a `StringArray`)
/// * `manifests` - A reference to an Array containing manifest paths (expected to be a `StringArray`)
///
/// # Returns
/// * `Result<HashMap<String, String>, DataFusionError>` - `HashMap` mapping file paths to manifest paths or an error
fn unique_files_and_manifests(
    files: &dyn Array,
    manifests: &dyn Array,
) -> Result<HashMap<String, String>, DataFusionError> {
    if files.is_empty() {
        return Ok(HashMap::new());
    }

    let first_file = downcast_array::<StringArray>(files).value(0).to_owned();
    let first_manifest = downcast_array::<StringArray>(manifests).value(0).to_owned();

    let init = if first_file.is_empty() {
        HashMap::new()
    } else {
        HashMap::from_iter([(first_file, first_manifest)])
    };

    let slice_len = files.len() - 1;

    if slice_len == 0 {
        return Ok(init);
    }

    let v1 = files.slice(0, slice_len);
    let v2 = files.slice(1, slice_len);

    let manifests = manifests.slice(1, slice_len);

    // Which consecutive entries are different
    let mask = distinct(&v1, &v2)?;

    // only keep values that are diffirent from their previous value, this drastically reduces the
    // number of values needed to process
    let unique_files = filter(&v2, &mask)?;

    let unique_manifests = filter(&manifests, &mask)?;

    let file_strings = downcast_array::<StringArray>(&unique_files);
    let manifest_strings = downcast_array::<StringArray>(&unique_manifests);

    let result =
        manifest_strings
            .iter()
            .zip(file_strings.iter())
            .fold(init, |mut acc, (manifest, file)| {
                if let (Some(manifest), Some(file)) = (manifest, file)
                    && !file.is_empty()
                {
                    acc.insert(file.to_owned(), manifest.to_owned());
                }
                acc
            });

    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_unique_files_and_manifests_with_duplicates() {
        let files = Arc::new(StringArray::from(vec![
            "file1", "file2", "file3", "file4", "file5",
        ]));
        let manifests = Arc::new(StringArray::from(vec![
            "manifest1",
            "manifest1",
            "manifest2",
            "manifest2",
            "manifest3",
        ]));

        let result = unique_files_and_manifests(files.as_ref(), manifests.as_ref()).unwrap();

        let expected = HashMap::from_iter([
            ("file1".to_string(), "manifest1".to_string()),
            ("file2".to_string(), "manifest1".to_string()),
            ("file3".to_string(), "manifest2".to_string()),
            ("file4".to_string(), "manifest2".to_string()),
            ("file5".to_string(), "manifest3".to_string()),
        ]);
        assert_eq!(result, expected);
    }
}
