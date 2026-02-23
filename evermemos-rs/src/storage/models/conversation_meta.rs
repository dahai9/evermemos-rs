use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,

    pub conv_id: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
