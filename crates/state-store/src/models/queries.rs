use crate::models::Entities;
use crate::models::Entity;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum QueryStatus {
    Created,
    Running,
    Successful,
    Failed,
}

impl TryFrom<&str> for QueryStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "created" => Ok(Self::Created),
            "running" => Ok(Self::Running),
            "successful" => Ok(Self::Successful),
            "failed" => Ok(Self::Failed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl Display for QueryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Created => f.write_str("created"),
            Self::Running => f.write_str("running"),
            Self::Successful => f.write_str("successful"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRecord {
    pub query_id: Uuid,   // uuid_v7
    pub request_id: Uuid, // uuid_v4

    pub query_text: String,
    pub database_name: String,
    pub session_id: String,
    pub user_name: String,

    pub warehouse_size: i32,
    pub warehouse_type: String,

    pub execution_status: String, // TODO use QueryStatus or switch entirely to ExecutionStatus
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    pub start_time: i64, // unix epoch millis
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,

    pub total_elapsed_time: i64,
    pub execution_time: i64,

    pub rows_produced: i64,
    pub rows_inserted: i64,
    pub rows_updated: i64,
    pub rows_deleted: i64,
    pub rows_unloaded: i64,
    pub bytes_deleted: i64,

    pub is_client_generated_statement: bool,

    pub query_hash: String,
    pub query_hash_version: i32,
}

impl Entity for QueryRecord {
    fn entity(&self) -> String {
        Entities::Query.to_string()
    }
}
