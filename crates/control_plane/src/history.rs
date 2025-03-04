use utils::{Db, IteratableEntity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: Uuid,
    pub query: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status_code: u16,
}

impl IteratableEntity for HistoryItem {
    fn id(&self) -> Uuid {
        self.id
    }

    fn prefix() -> &'static str {
        "hi."
    }

    fn key_from_time(time: DateTime<Utc>) -> String {
        format!("{}{}", Self::prefix(), time.to_string())
    }

    fn key(&self) -> String {
        Self::key_from_time(self.start_time)
    }
}
