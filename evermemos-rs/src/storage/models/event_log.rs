use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Atomic fact extracted from a conversation (assistant scenes only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub timestamp: DateTime<Utc>,

    /// The extracted atomic fact sentence
    pub atomic_fact: String,

    pub vector: Option<Vec<f32>>,
    pub search_content: Option<String>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
