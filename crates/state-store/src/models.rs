use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entities {
    Session,
    View,
    Query,
}

impl Display for Entities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session => write!(f, "session"),
            Self::View => write!(f, "view"),
            Self::Query => write!(f, "query"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionStatus {
    #[default]
    Success,
    Fail,
    Incident,
}

impl Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Success => "success",
            Self::Fail => "fail",
            Self::Incident => "incident",
        };
        write!(f, "{value}")
    }
}

/// Session entity persisted in `DynamoDB`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
    #[serde(default)]
    pub views: HashMap<String, ViewRecord>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

impl SessionRecord {
    /// Create a new session record with default values and a current timestamp.
    #[must_use]
    pub fn new(session_id: &str) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id: session_id.to_string(),
            ttl_seconds: None,
            variables: HashMap::new(),
            views: HashMap::new(),
            created_at,
            updated_at: None,
        }
    }

    #[must_use]
    pub fn entity(&self) -> String {
        Entities::Session.to_string()
    }
}

/// Logical view entity describing embucket-managed views.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewRecord {
    pub view_id: String,
    pub database: String,
    pub schema: String,
    pub name: String,
    pub sql_definition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    /// full name of the variable with the name space
    pub name: String,
    pub value: String,
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub query_id: String,
    pub request_id: Option<String>,
    pub query_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_type: Option<String>,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authn_event_id: Option<String>,
    pub user_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_status: Option<ExecutionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub start_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_elapsed_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_scanned: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage_scanned_from_cache: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_written: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_written_to_result: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_read_from_result: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_produced: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_inserted: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_updated: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_deleted: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_unloaded: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_deleted: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partitions_scanned: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partitions_total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_spilled_to_local_storage: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_spilled_to_remote_storage: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_sent_over_the_network: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compilation_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_provisioning_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_repair_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_overload_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_blocked_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_data_transfer_cloud: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_data_transfer_region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbound_data_transfer_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_data_transfer_cloud: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_data_transfer_region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_data_transfer_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list_external_files_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_used_cloud_services: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_function_total_invocations: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_function_total_sent_rows: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_function_total_received_rows: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_function_total_sent_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_function_total_received_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_load_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_client_generated_statement: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_acceleration_bytes_scanned: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_acceleration_partitions_scanned: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_acceleration_upper_limit_scale_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_queries_wait_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_hash_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_parameterized_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_parameterized_hash_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_role_stats: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows_written_to_result: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_retry_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_retry_cause: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault_handling_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_database_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_database_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_schema_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_schema_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_values: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_history_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_result_time: Option<u64>,
}

impl Query {
    #[must_use]
    pub fn entity(&self) -> String {
        Entities::Query.to_string()
    }

    #[must_use]
    pub fn new(query_text: String) -> Self {
        Self {
            query_id: uuid::Uuid::now_v7().to_string(),
            query_text,
            ..Default::default()
        }
    }

    // Why? warning: this could be a `const fn`
    #[allow(clippy::missing_const_for_fn)]
    pub fn set_execution_status(&mut self, status: ExecutionStatus) {
        self.execution_status = Some(status);
    }

    pub fn set_error_code(&mut self, error_code: String) {
        self.error_code = Some(error_code);
    }
}
