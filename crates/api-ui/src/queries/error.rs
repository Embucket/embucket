use crate::error::IntoStatusCode;
use core_history::Error as HistoryStoreError;
use http::status::StatusCode;
use snafu::Location;
use snafu::prelude::*;

pub(crate) type QueryRecordResult<T> = Result<T, QueryError>;

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Query execution error: {source}"))]
    Query {
        source: QueryError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error getting queries: {source}"))]
    Queries {
        source: QueryError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error getting query record: {source}"))]
    GetQueryRecord {
        source: QueryError,
        #[snafu(implicit)]
        location: Location,
    },
}

// kind of common errors for query
#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
#[error_stack_trace::debug]
pub enum QueryError {
    #[snafu(display("Query execution error: {source}"))]
    Execution {
        source: core_executor::error::ExecutionError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("History store: {source}"))]
    Store {
        source: HistoryStoreError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse row JSON: {error}"))]
    ResultParse {
        #[snafu(source)]
        error: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("ResultSet create error: {error}"))]
    CreateResultSet {
        #[snafu(source)]
        error: datafusion::arrow::error::ArrowError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error encoding UTF8 string: {error}"))]
    Utf8 {
        #[snafu(source)]
        error: std::string::FromUtf8Error,
        #[snafu(implicit)]
        location: Location,
    },
}

// Select which status code to return.
impl IntoStatusCode for Error {
    #[allow(clippy::match_wildcard_for_single_variants)]
    #[allow(clippy::collapsible_match)]
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Query { source, .. } => match &source {
                QueryError::Execution { .. } => StatusCode::UNPROCESSABLE_ENTITY,
                QueryError::Store { .. } => StatusCode::BAD_REQUEST,
                QueryError::ResultParse { .. }
                | QueryError::Utf8 { .. }
                | QueryError::CreateResultSet { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::Queries { source, .. } => match &source {
                QueryError::ResultParse { .. } => StatusCode::UNPROCESSABLE_ENTITY,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::GetQueryRecord { source, .. } => match &source {
                QueryError::Store { source, .. } => match &source {
                    HistoryStoreError::QueryGet { .. } | HistoryStoreError::BadKey { .. } => {
                        StatusCode::NOT_FOUND
                    }
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
