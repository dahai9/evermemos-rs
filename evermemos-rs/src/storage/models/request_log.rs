use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::sql::Thing;

/// Audit log and pending-message queue for message ingestion.
/// sync_status: -1=pending, 0=processing, 1=done, -2=error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequestLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub message_id: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub payload: Value,
    #[serde(default = "default_sync_status")]
    pub sync_status: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_sync_status() -> i32 {
    -1
}

impl MemoryRequestLog {
    pub fn is_pending(&self) -> bool {
        self.sync_status == -1
    }
    pub fn is_done(&self) -> bool {
        self.sync_status == 1
    }
}
