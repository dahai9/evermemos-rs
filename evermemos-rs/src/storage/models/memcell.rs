use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::sql::Thing;

/// Atomic boundary-detected unit of raw conversation data.
/// Equivalent to Python's `MemCell` MongoDB document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub original_data: Option<Vec<Value>>,
    pub participants: Option<Vec<String>>,
    pub subject: Option<String>,
    pub keywords: Option<Vec<String>>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl MemCell {
    pub fn new(
        user_id: Option<String>,
        group_id: Option<String>,
        timestamp: DateTime<Utc>,
        summary: Option<String>,
        original_data: Option<Vec<Value>>,
        participants: Option<Vec<String>>,
        subject: Option<String>,
        keywords: Option<Vec<String>>,
    ) -> Self {
        Self {
            id: None,
            user_id,
            group_id,
            timestamp,
            summary,
            original_data,
            participants,
            subject,
            keywords,
            is_deleted: false,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}
