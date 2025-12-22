pub mod sessions;

cfg_if::cfg_if! {
    if #[cfg(feature = "statestore-queries")] {
        pub mod queries;
        pub use queries::QueryStatus;
        pub use queries::QueryRecord;
    }
}

use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub trait Entity {
    fn entity(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entities {
    Session,
    View,
    Variable,
    Query,
}

impl Display for Entities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session => write!(f, "session"),
            Self::View => write!(f, "view"),
            Self::Variable => write!(f, "variable"),
            Self::Query => write!(f, "query"),
        }
    }
}
