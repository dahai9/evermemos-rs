use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::sql::Thing;

/// User profile — accumulated from multiple ProfileMemoryExtractor runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub user_id: String,
    /// Structured profile attributes (e.g. age, occupation, interests…)
    pub profile_data: Option<Value>,
    /// Long-form life narrative summary
    pub life_summary: Option<String>,
    /// Custom profile data (e.g. initial_profile sentences) injected via API
    pub custom_profile_data: Option<Value>,

    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
