use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::sql::Thing;

/// A single discussion topic extracted from group conversations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopicInfo {
    pub name: String,
    pub summary: Option<String>,
    /// "exploring" | "disagreement" | "consensus" | "implemented"
    pub status: Option<String>,
}

/// Aggregated group-chat profile (topics, summary, subject).
/// One record per group_id — upserted on every new memcell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub group_id: String,
    pub group_name: Option<String>,

    /// Serialised `Vec<TopicInfo>` — stored as a JSON array.
    pub topics: Option<Value>,

    /// One-sentence overview of the group's current focus.
    pub summary: Option<String>,

    /// Long-term positioning / purpose of the group, or "not_found".
    pub subject: Option<String>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for GroupProfile {
    fn default() -> Self {
        Self {
            id: None,
            group_id: String::new(),
            group_name: None,
            topics: None,
            summary: None,
            subject: None,
            is_deleted: false,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}
