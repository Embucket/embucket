use error::Result;
use futures::TryFutureExt;
use futures::executor::block_on;
use snafu::ResultExt;
use tokio::runtime::{Builder, Handle, RuntimeFlavor};

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

// TBD: Should we move this into a separate crate? As this is duplicate implementation
// of what we have in the functions crate.
pub fn block_in_new_runtime<F, R>(future: F) -> Result<R>
where
    F: Future<Output = Result<R>> + Send + 'static,
    R: Send + 'static,
{
    let handle = std::thread::spawn(move || {
        // Try to create a dedicated Tokio runtime
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .context(error::CreateTokioRuntimeSnafu)?;

        // Execute the future and map its error
        rt.block_on(future)
    });

    match handle.join() {
        Ok(inner) => inner, // inner: Result<R, error::Error>
        Err(_) => error::ThreadPanickedWhileExecutingFutureSnafu.fail()?,
    }
}

fn block_on_with_timeout<F>(future: F, timeout_duration: tokio::time::Duration) -> Result<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let future_with_timeout = async move {
        tokio::time::timeout(timeout_duration, future)
            .await
            .context(error::TimeoutSnafu)
    };

    match Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            RuntimeFlavor::CurrentThread => block_on(
                tokio::task::spawn_blocking(|| block_on(future_with_timeout))
                    .unwrap_or_else(|err| std::panic::resume_unwind(err.into_panic())),
            ),
            _ => tokio::task::block_in_place(|| handle.block_on(future_with_timeout)),
        },
        Err(_) => block_on(future_with_timeout),
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
