use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// User behavior history record.
///
/// Records various user interactions: chat, follow-up, vote, file operations, etc.
/// Mirrors the Python `BehaviorHistory` MongoDB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorHistory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: String,
    pub timestamp: DateTime<Utc>,

    /// Behavior type tags, e.g. ["chat", "follow-up"], ["Smart-Reply"], ["Vote"]
    pub behavior_type: Vec<String>,

    /// Associated memory/event ID (links to a MemCell or EpisodicMemory)
    pub event_id: Option<String>,

    /// Flexible metadata: conversation details, file info, email content, etc.
    pub meta: Option<serde_json::Value>,

    /// Reserved extension field
    pub extend: Option<serde_json::Value>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl BehaviorHistory {
    /// Primary behavior tag (first entry), or "unknown".
    pub fn primary_type(&self) -> &str {
        self.behavior_type.first().map(|s| s.as_str()).unwrap_or("unknown")
    }
}
