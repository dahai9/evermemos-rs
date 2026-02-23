use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// All task payloads exchanged over NATS.
/// Encoded as JSON and published to the configured NATS subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPayload {
    /// Request to memorise a single message.
    Memorize(MemorizeTask),
    /// Request to synchronise/rebuild a user's memory.
    Sync(SyncTask),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorizeTask {
    /// Unique task ID for deduplication.
    pub task_id: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub message_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub create_time: DateTime<Utc>,
    pub role: Option<String>,
    /// Previously buffered messages (JSON values).
    pub history: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub task_id: String,
    pub user_id: String,
    pub reason: String,
}
