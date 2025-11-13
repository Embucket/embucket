pub use catalog;
pub mod datafusion;
pub mod dedicated_executor;
pub mod error;
pub mod error_code;
pub mod models;
pub mod query;
pub mod query_types;
pub mod row;
pub mod running_queries;
pub mod service;
pub mod session;
pub mod snowflake_error;
pub mod tracing;
pub mod utils;

pub use error::{Error, Result};
pub use query_types::{QueryRecordId, QueryStatus};
pub use row::Row;
pub use running_queries::RunningQueryId;
pub use snowflake_error::SnowflakeError;

use crate::service::ExecutionService;
use std::sync::Arc;

pub trait ExecutionAppState {
    fn get_execution_svc(&self) -> Arc<dyn ExecutionService>;
}
