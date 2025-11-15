use super::QuerySource;
use super::QueryStatus;
use super::ResultFormat;
use super::diesel_schema::queries;
use chrono::prelude::*;
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Eq, PartialEq)]
#[diesel(table_name = queries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Query {
    pub id: Uuid,
    pub sql: String,
    pub status: QueryStatus,
    pub source: QuerySource,
    pub request_id: Option<Uuid>,
    pub request_metadata: Value,
    pub result_format: ResultFormat,
    pub created_at: DateTime<Utc>,
    pub limit_exceeded_at: Option<DateTime<Utc>>,
    pub queued_at: Option<DateTime<Utc>>,
    pub running_at: Option<DateTime<Utc>>,
    pub successful_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub timedout_at: Option<DateTime<Utc>>,
    pub duration_ms: i64,
    pub rows_count: i64,
    pub error: Option<String>,
}

impl Query {
    #[must_use]
    pub fn new(sql: String, source: QuerySource, format: ResultFormat) -> Self {
        let mut query = Self {
            id: Uuid::new_v4(),
            sql,
            status: QueryStatus::Created,
            source,
            result_format: format,
            request_id: None,
            request_metadata: Value::Null,
            created_at: Utc::now(),
            limit_exceeded_at: None,
            queued_at: None,
            running_at: None,
            successful_at: None,
            failed_at: None,
            cancelled_at: None,
            timedout_at: None,
            duration_ms: 0,
            rows_count: 0,
            error: None,
        };
        query.set_status(QueryStatus::Created);
        query
    }

    #[must_use]
    pub fn with_request_id(self, request_id: Uuid) -> Self {
        Self {
            request_id: Some(request_id),
            ..self
        }
    }

    #[must_use]
    pub fn with_request_metadata(self, request_metadata: Value) -> Self {
        Self {
            request_metadata,
            ..self
        }
    }

    #[must_use]
    pub fn with_source(self, source: QuerySource) -> Self {
        Self { source, ..self }
    }

    #[must_use]
    pub fn with_result_format(self, result_format: ResultFormat) -> Self {
        Self {
            result_format,
            ..self
        }
    }

    pub fn set_status(&mut self, status: QueryStatus) {
        self.status = status;
        let now = Utc::now();
        match status {
            QueryStatus::Created => {} // created_at is set in constructor once
            QueryStatus::LimitExceeded => self.limit_exceeded_at = Some(now),
            QueryStatus::Queued => self.queued_at = Some(now),
            QueryStatus::Running => self.running_at = Some(now),
            QueryStatus::Successful => {
                self.successful_at = Some(now);
                self.duration_ms = now
                    .signed_duration_since(self.created_at)
                    .num_milliseconds();
            }
            QueryStatus::Failed => self.failed_at = Some(now),
            QueryStatus::Cancelled => self.cancelled_at = Some(now),
            QueryStatus::TimedOut => self.timedout_at = Some(now),
        }
    }

    pub fn set_successful(&mut self, rows_count: i64) {
        self.set_status(QueryStatus::Successful);
        self.rows_count = rows_count;
    }

    pub fn set_failed(&mut self, error: String) {
        self.set_status(QueryStatus::Failed);
        self.error = Some(error);
    }
}
