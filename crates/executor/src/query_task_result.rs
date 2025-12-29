use super::error as ex_error;
use super::error::Result;
use super::error_code::ErrorCode;
use super::models::QueryResult;
use super::query_types::ExecutionStatus;
use super::snowflake_error::SnowflakeError;
use snafu::ResultExt;
use tokio::task::JoinError;
use uuid::Uuid;

// pub type TaskFuture = tokio::task::JoinHandle<std::result::Result<QueryResult, Error>>;

pub struct ExecutionTaskResult {
    pub result: Result<QueryResult>,
    pub execution_status: ExecutionStatus,
    pub error_code: Option<ErrorCode>,
}

impl ExecutionTaskResult {
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
    pub fn assign_query_attributes(&self, query: &mut state_store::Query) {
        query.set_execution_status(self.execution_status);
        if let Some(error_code) = self.error_code {
            query.set_error_code(error_code.to_string());
        }
        if let Err(err) = &self.result {
            query.set_error_message(err.to_string());
        }
        query.set_end_time();
    }
}
