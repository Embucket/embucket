use serde::{Deserialize, Serialize};
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

/// Session entity persisted in `DynamoDB`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub views: Vec<ViewRecord>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

impl SessionRecord {
    /// Create a new session record with default values and a current timestamp.
    #[must_use]
    pub fn new(session_id: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id,
            ttl_seconds: None,
            variables: Vec::new(),
            views: Vec::new(),
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
