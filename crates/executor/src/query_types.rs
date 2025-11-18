use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

pub type QueryId = Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    Running,
    Successful,
    Failed,
    Cancelled,
    TimedOut,
}
