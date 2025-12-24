pub type QueryId = uuid::Uuid;

cfg_if::cfg_if! {
    if #[cfg(feature = "state-store-query")] {
        pub use state_store::ExecutionStatus;
    } else {
        use serde::{Deserialize, Serialize};
        use std::fmt::Debug;
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum ExecutionStatus {
            Success,
            Fail,
            Incident,
        }
    }
}
