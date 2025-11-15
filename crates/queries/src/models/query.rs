use uuid::Uuid;
use chrono::prelude::*;
use serde_json::Value;
use super::QuerySource;
use super::QueryStatus;
use super::ResultFormat;
use super::diesel_schema::queries;
use diesel::prelude::*;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Eq, PartialEq)]
#[diesel(table_name = queries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Query {
    pub id: Uuid,
    pub request_id: Uuid,
    pub request_metadata: Value,
    pub sql: String,
    pub source: QuerySource,
    pub created_at: Option<DateTime<Utc>>,
    pub queued_at: Option<DateTime<Utc>>,
    pub running_at: Option<DateTime<Utc>>,
    pub successful_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub timedout_at: Option<DateTime<Utc>>,
    pub duration_ms: i64,
    pub rows_count: i64,
    pub result_format: ResultFormat,
    pub status: QueryStatus,
    pub error: Option<String>,
}