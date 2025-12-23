use super::error as ex_error;
use super::error::{Error, Result};
use super::error_code::ErrorCode;
use super::models::QueryResult;
use super::query_types::ExecutionStatus;
use super::snowflake_error::SnowflakeError;
use snafu::ResultExt;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type TaskFuture = tokio::task::JoinHandle<std::result::Result<QueryResult, Error>>;

pub struct ExecutionTaskResult {
    pub result: Result<QueryResult>,
    pub execution_status: ExecutionStatus,
    pub error_code: Option<ErrorCode>,
}

pub async fn query_task_execution_result(
    task_future: TaskFuture,
    query_token: CancellationToken,
    query_id: Uuid,
    timeout_duration: Duration,
) -> ExecutionTaskResult {
    let subtask_abort_handle = task_future.abort_handle();

    // wait for any future to be resolved
    tokio::select! {
        finished = task_future => {
            match finished {
                Ok(inner_result) => {
                    let execution_status = inner_result.as_ref().map_or_else(|_| ExecutionStatus::Fail, |_| ExecutionStatus::Success);
                    let error_code = match inner_result.as_ref() {
                        Ok(_) => None,
                        Err(err) => Some(SnowflakeError::from_executor_error(err).error_code()),
                    };
                    // set query execution status to successful or failed
                    ExecutionTaskResult {
                        result: inner_result.context(ex_error::QueryExecutionSnafu { query_id }),
                        execution_status,
                        error_code,
                    }
                },
                Err(error) => {
                    tracing::error!("Query {query_id} sub task join error: {error:?}");
                    ExecutionTaskResult {
                        result: Err(error).context(ex_error::QuerySubtaskJoinSnafu).context(ex_error::QueryExecutionSnafu {
                            query_id,
                        }),
                        execution_status: ExecutionStatus::Incident,
                        error_code: Some(crate::error_code::ErrorCode::QueryTask),
                    }
                },
            }
        },
        () = query_token.cancelled() => {
            tracing::info_span!("query_cancelled_do_abort");
            subtask_abort_handle.abort();
            ExecutionTaskResult {
                result: ex_error::QueryCancelledSnafu { query_id }.fail().context(ex_error::QueryExecutionSnafu {
                    query_id,
                }),
                execution_status: ExecutionStatus::Fail,
                error_code: Some(crate::error_code::ErrorCode::Cancelled),
            }
        },
        // Execute the query with a timeout to prevent long-running or stuck queries
        // from blocking system resources indefinitely. If the timeout is exceeded,
        // convert the timeout into a standard QueryTimeout error so it can be handled
        // and recorded like any other execution failure
        () = tokio::time::sleep(timeout_duration) => {
            tracing::info_span!("query_timeout_received_do_abort");
            subtask_abort_handle.abort();
            ExecutionTaskResult {
                result: ex_error::QueryTimeoutSnafu.fail().context(ex_error::QueryExecutionSnafu {
                    query_id,
                }),
                execution_status: ExecutionStatus::Fail,
                error_code: Some(crate::error_code::ErrorCode::Timeout),
            }
        }
    }
}
