use error::Result;
use snafu::ResultExt;
use std::future::Future;
use tokio::runtime::{Builder, Handle, RuntimeFlavor};
use tokio::task;
use tracing::{Span, instrument};

#[allow(clippy::module_inception)]
pub mod catalog;
pub mod catalog_list;
pub mod catalogs;
pub mod df_error;
pub mod error;
pub mod information_schema;
pub mod schema;
pub mod table;
pub mod utils;

#[cfg(test)]
pub mod tests;

/// Executes an async future in a blocking context without causing Tokio runtime deadlocks.
///
/// This function is designed to safely run async code from synchronous call sites:
/// - If no Tokio runtime is active, a new temporary runtime is created.
/// - If running inside a multithreaded Tokio runtime, `block_in_place` is used.
/// - If running inside a single-threaded runtime, the future is executed inside
///   a `spawn_blocking` thread with its own temporary runtime to avoid self-deadlock.
///
/// The goal is to provide a reliable way to synchronously wait on async futures
/// in contexts such as catalog operations or plugin layers, while preserving correct
/// runtime behavior and avoiding nested `block_on` deadlocks.
#[instrument(level = "debug", skip(future), fields(future_type = ""))]
pub fn block_in_new_runtime<F, R>(future: F) -> Result<R>
where
    F: Future<Output = Result<R>> + Send + 'static,
    R: Send + 'static,
{
    Span::current().record("future_type", std::any::type_name::<F>());
    if let Ok(handle) = Handle::try_current() {
        match handle.runtime_flavor() {
            RuntimeFlavor::CurrentThread => {
                let join_handle = task::spawn_blocking(|| futures::executor::block_on(future));
                match futures::executor::block_on(join_handle) {
                    Ok(res) => res,
                    Err(_) => error::ThreadPanickedWhileExecutingFutureSnafu.fail()?,
                }
            }
            _ => task::block_in_place(|| handle.block_on(future)),
        }
    } else {
        // Try to create a dedicated Tokio runtime when none is available
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .context(error::CreateTokioRuntimeSnafu)?;
        // Execute the future and map its error
        rt.block_on(future)
    }
}

pub mod test_utils {
    use datafusion::arrow::array::{ArrayRef, RecordBatch};
    use datafusion::arrow::compute::{
        SortColumn, SortOptions, lexsort_to_indices, take_record_batch,
    };
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[allow(clippy::unwrap_used, clippy::must_use_candidate)]
    pub fn sort_record_batch_by_sortable_columns(batch: &RecordBatch) -> RecordBatch {
        let sort_columns: Vec<SortColumn> = (0..batch.num_columns())
            .filter_map(|i| {
                let col = batch.column(i).clone();
                let field = batch.schema().field(i).clone();
                if matches!(field.data_type(), DataType::Null) {
                    None
                } else {
                    Some(SortColumn {
                        values: col,
                        options: Some(SortOptions::default()),
                    })
                }
            })
            .collect();

        if sort_columns.is_empty() {
            return batch.clone();
        }

        let indices = lexsort_to_indices(&sort_columns, Some(batch.num_rows())).unwrap();
        take_record_batch(batch, &indices).unwrap()
    }

    #[allow(clippy::unwrap_used, clippy::must_use_candidate)]
    pub fn remove_columns_from_batches<S: std::hash::BuildHasher>(
        batches: Vec<RecordBatch>,
        excluded_columns: &HashSet<&str, S>,
    ) -> Vec<RecordBatch> {
        batches
            .into_iter()
            .map(|batch| {
                let schema = batch.schema();
                let indices: Vec<usize> = schema
                    .fields()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, f)| {
                        if excluded_columns.contains(f.name().as_str()) {
                            None
                        } else {
                            Some(i)
                        }
                    })
                    .collect();

                let columns: Vec<ArrayRef> =
                    indices.iter().map(|&i| batch.column(i).clone()).collect();
                let fields: Vec<Field> = indices.iter().map(|&i| schema.field(i).clone()).collect();
                let new_schema = Arc::new(Schema::new(fields));

                RecordBatch::try_new(new_schema, columns).unwrap()
            })
            .collect()
    }
}
