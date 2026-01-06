pub mod config;
pub mod error;
pub mod models;
pub mod state_store_dynamo;

pub use config::DynamoDbConfig;
pub use error::{Error, Result};
pub use models::{ExecutionStatus, Query, SessionRecord, Variable, ViewRecord};
pub use state_store_dynamo::DynamoDbStateStore;

#[mockall::automock]
#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    async fn put_new_session(&self, session_id: &str) -> Result<()>;
    async fn put_session(&self, session: SessionRecord) -> Result<()>;
    async fn get_session(&self, session_id: &str) -> Result<SessionRecord>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn update_session(&self, session: SessionRecord) -> Result<()>;
    async fn put_query(&self, query: &Query) -> Result<()>;
    async fn get_query(&self, query_id: &str) -> Result<Query>;
    async fn get_query_by_request_id(&self, request_id: &str) -> Result<Query>;
    async fn get_queries_by_session_id(&self, session_id: &str) -> Result<Vec<Query>>;
    async fn delete_query(&self, query_id: &str) -> Result<()>;
    async fn update_query(&self, query: &Query) -> Result<()>;
}
