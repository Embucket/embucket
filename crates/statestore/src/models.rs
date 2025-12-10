use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
    pub ttl_seconds: Option<i64>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub views: Vec<ViewRecord>,
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
    pub ttl_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    /// full name of the variable with the name space
    pub name: String,
    pub value: String,
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}
