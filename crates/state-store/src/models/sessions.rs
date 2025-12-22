use crate::models::{Entities, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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

impl Entity for SessionRecord {
    fn entity(&self) -> String {
        Entities::Session.to_string()
    }
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

impl Entity for ViewRecord {
    fn entity(&self) -> String {
        Entities::View.to_string()
    }
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

impl Entity for Variable {
    fn entity(&self) -> String {
        Entities::Variable.to_string()
    }
}
