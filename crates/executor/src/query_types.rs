pub type QueryId = uuid::Uuid;
use std::fmt::Display;

cfg_if::cfg_if! {
    if #[cfg(feature = "state-store-query")] {
        pub use state_store::ExecutionStatus;
    } else {
        use serde::{Deserialize, Serialize};
        use std::fmt::Debug;
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum ExecutionStatus {
            Running,
            Success,
            Fail,
            Incident,
        }
    }
}

#[derive(Debug, Clone, strum::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum DmlStType {
    Select,
    Insert,
    Update,
    Delete,
    Truncate,
    Merge,
}

#[derive(Debug, Clone, strum::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum DdlStType {
    CreateExternalTable,
    CreateTable,
    CreateView,
    CreateDatabase,
    CreateVolume,
    CreateSchema,
    CreateStage,
    CopyIntoSnowflake,
    DropTable,
    AlterTable,
    AlterSession,
    Drop,
}

#[derive(Debug, Clone, strum::Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum MiscStType {
    Use,
    Set,
    Begin,
    Commit,
    Rollback,
    ShowColumns,
    ShowFunctions,
    ShowVariables,
    ShowObjects,
    ShowVariable,
    ShowDatabases,
    ShowSchemas,
    ShowTables,
    ShowViews,
    ExplainTable,
}

#[derive(Debug, Clone)]
pub enum QueryType {
    Dml(DmlStType),
    Ddl(DdlStType),
    Misc(MiscStType),
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dml(dml) => write!(f, "{dml}"),
            Self::Ddl(ddl) => write!(f, "{ddl}"),
            Self::Misc(misc) => write!(f, "{misc}"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryStats {
    pub query_type: Option<QueryType>,
    pub rows_count: Option<u64>,
}

impl QueryStats {
    pub fn with_query_type(self, query_type: QueryType) -> Self {
        Self {
            query_type: Some(query_type),
            ..self
        }
    }

    pub fn with_rows_count(self, rows_count: u64) -> Self {
        Self {
            rows_count: Some(rows_count),
            ..self
        }
    }
}