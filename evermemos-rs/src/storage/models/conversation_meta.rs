use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDetail {
    pub full_name: String,
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub conv_id: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,

    // Python-compatible metadata fields
    pub name: Option<String>,
    pub description: Option<String>,
    pub scene: Option<String>,
    pub scene_desc: Option<Value>,
    pub tags: Option<Vec<String>>,
    pub user_details: Option<Value>,
    pub default_timezone: Option<String>,

    // Legacy / internal
    pub title: Option<String>,
    pub summary: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
