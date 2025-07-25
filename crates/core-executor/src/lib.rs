pub use df_catalog as catalog;
use std::sync::Arc;
pub mod datafusion;
pub mod dedicated_executor;
pub mod error;
pub mod models;
pub mod query;
pub mod service;
pub mod session;
pub mod snowflake_error;
pub mod utils;

#[cfg(test)]
pub mod tests;

use crate::service::ExecutionService;
pub use error::{Error, Result};
pub use snowflake_error::SnowflakeError;

pub trait ExecutionAppState {
    fn get_execution_svc(&self) -> Arc<dyn ExecutionService>;
}
