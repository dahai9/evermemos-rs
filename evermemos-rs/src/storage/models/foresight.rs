use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Predictive memory — assistant-scene only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForesightRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub timestamp: DateTime<Utc>,

    /// The prediction text
    pub foresight: String,
    /// Supporting evidence quote from conversation
    pub evidence: Option<String>,

    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_days: Option<i32>,

    pub vector: Option<Vec<f32>>,
    pub search_content: Option<String>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
