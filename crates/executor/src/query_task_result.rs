use super::error as ex_error;
use super::error::Result;
use super::error_code::ErrorCode;
use super::models::QueryResult;
use super::query_types::ExecutionStatus;
cfg_if::cfg_if! {
    if #[cfg(feature = "state-store-query")] {
        use super::query_types::{DmlStType, QueryType};
        use datafusion::arrow::array::{Int64Array, UInt64Array};
    }
}
use super::snowflake_error::SnowflakeError;
use snafu::ResultExt;
use state_store::QueryMetric;
use tokio::task::JoinError;
use uuid::Uuid;

// pub type TaskFuture = tokio::task::JoinHandle<std::result::Result<QueryResult, Error>>;

pub struct ExecutionTaskResult {
    pub result: Result<QueryResult>,
    pub execution_status: ExecutionStatus,
    pub error_code: Option<ErrorCode>,
}

impl ExecutionTaskResult {
    #[must_use]
    pub fn from_query_result(query_id: Uuid, result: Result<QueryResult>) -> Self {
        let execution_status = result
            .as_ref()
            .map_or_else(|_| ExecutionStatus::Fail, |_| ExecutionStatus::Success);
        let error_code = match result.as_ref() {
            Ok(_) => None,
            Err(err) => Some(SnowflakeError::from_executor_error(err).error_code()),
        };
        // set query execution status to successful or failed
        Self {
            result: result.context(ex_error::QueryExecutionSnafu { query_id }),
            execution_status,
            error_code,
        }
    }

    #[must_use]
    pub fn from_query_limit_exceeded(query_id: Uuid) -> Self {
        Self {
            result: ex_error::ConcurrencyLimitSnafu
                .fail()
                .context(ex_error::QueryExecutionSnafu { query_id }),
            execution_status: ExecutionStatus::Incident,
            error_code: Some(ErrorCode::LimitExceeded),
        }
    }

    #[must_use]
    pub fn from_failed_query_task(query_id: Uuid, task_error: JoinError) -> Self {
        Self {
            result: Err(task_error)
                .context(ex_error::QuerySubtaskJoinSnafu)
                .context(ex_error::QueryExecutionSnafu { query_id }),
            execution_status: ExecutionStatus::Incident,
            error_code: Some(ErrorCode::QueryTask),
        }
    }

    #[must_use]
    pub fn from_cancelled_query_task(query_id: Uuid) -> Self {
        Self {
            result: ex_error::QueryCancelledSnafu { query_id }
                .fail()
                .context(ex_error::QueryExecutionSnafu { query_id }),
            execution_status: ExecutionStatus::Fail,
            error_code: Some(ErrorCode::Cancelled),
        }
    }

    #[must_use]
    pub fn from_timeout_query_task(query_id: Uuid) -> Self {
        Self {
            result: ex_error::QueryTimeoutSnafu
                .fail()
                .context(ex_error::QueryExecutionSnafu { query_id }),
            execution_status: ExecutionStatus::Fail,
            error_code: Some(ErrorCode::Timeout),
        }
    }

    #[cfg(feature = "state-store-query")]
    pub fn assign_rows_counts_attributes(
        &self,
        query: &mut state_store::Query,
        query_type: QueryType,
    ) {
        if let Ok(result) = &self.result
            && let QueryType::Dml(query_type) = query_type
        {
            if let DmlStType::Select = query_type {
                let rows_count: u64 = result.records.iter().map(|r| r.num_rows() as u64).sum();
                query.set_rows_produced(rows_count);
            } else if let Some(rows_count) = value_by_row_column(&result, 0, 0) {
                match query_type {
                    DmlStType::Insert => query.set_rows_inserted(rows_count as u64),
                    DmlStType::Update => query.set_rows_updated(rows_count as u64),
                    DmlStType::Delete => query.set_rows_deleted(rows_count as u64),
                    DmlStType::Truncate => query.set_rows_deleted(rows_count as u64),
                    DmlStType::Merge => {
                        // merge has 2 columns, currently map values to insert/select rows counts
                        query.set_rows_inserted(rows_count as u64);
                        if let Some(rows_count) = value_by_row_column(&result, 0, 1) {
                            query.set_rows_produced(rows_count as u64);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    #[cfg(feature = "state-store-query")]
    pub fn assign_query_attributes(&self, query: &mut state_store::Query) {
        query.set_execution_status(self.execution_status);
        if let Some(error_code) = self.error_code {
            query.set_error_code(error_code.to_string());
        }
        if let Err(err) = &self.result {
            query.set_error_message(err.to_string());
        }

        // Collect result metrics
        if let Ok(res) = &self.result {
            query.set_query_metrics(
                res.metrics
                    .iter()
                    .map(|metric| QueryMetric {
                        node_id: metric.node_id,
                        parent_node_id: metric.parent_node_id,
                        operator: metric.operator.clone(),
                        metrics: metric.metrics.clone(),
                    })
                    .collect(),
            );
        }
        query.set_end_time();
    }
}

#[cfg(feature = "state-store-query")]
fn value_by_row_column(result: &QueryResult, row_idx: usize, col_idx: usize) -> Option<u64> {
    result.records[0].columns().get(col_idx).and_then(|col| {
        if let Some(cols) = col.as_any().downcast_ref::<Int64Array>() {
            if row_idx < cols.len() {
                Some(cols.value(row_idx) as u64)
            } else {
                None
            }
        } else if let Some(cols) = col.as_any().downcast_ref::<UInt64Array>() {
            if row_idx < cols.len() {
                Some(cols.value(row_idx))
            } else {
                None
            }
        } else {
            None
        }
    })
}
