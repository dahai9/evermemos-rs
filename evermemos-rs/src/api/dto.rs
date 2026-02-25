use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agentic::manager::{MemoryItem, MemoryType, RetrieveMethod};

// ─────────────────────────────────────────────────────────────────────────────
// Unified API response envelope (mirrors Python BaseApiResponse[T])
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub status: &'static str,
    pub message: String,
    pub result: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(message: impl Into<String>, result: T) -> Self {
        Self {
            status: "success",
            message: message.into(),
            result: Some(result),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/v1/memories — Memorize
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MemorizeMessageRequest {
    pub message_id: String,
    pub create_time: DateTime<Utc>,
    pub sender: String,
    pub content: String,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub sender_name: Option<String>,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    /// "user" | "assistant"
    pub role: Option<String>,
    /// Previous buffered messages context
    pub history: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct MemorizeResponse {
    pub status: String,
    pub message: String,
    pub saved_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/memories — Fetch memories (paginated)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FetchMemoriesQuery {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub memory_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct FetchMemoriesResponse {
    pub memories: Vec<MemoryItem>,
    pub total_count: usize,
    pub has_more: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/memories/search — Semantic / keyword search
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchMemoriesQuery {
    pub query: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    /// KEYWORD | VECTOR | HYBRID | RRF | AGENTIC
    pub retrieve_method: Option<String>,
    /// Comma-separated list of memory types
    pub memory_types: Option<String>,
    pub top_k: Option<u32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub radius: Option<f32>,
}

impl SearchMemoriesQuery {
    pub fn parse_method(&self) -> RetrieveMethod {
        match self.retrieve_method.as_deref().unwrap_or("VECTOR") {
            "KEYWORD" => RetrieveMethod::Keyword,
            "HYBRID" => RetrieveMethod::Hybrid,
            "RRF" => RetrieveMethod::Rrf,
            "AGENTIC" => RetrieveMethod::Agentic,
            _ => RetrieveMethod::Vector,
        }
    }

    pub fn parse_memory_types(&self) -> Vec<MemoryType> {
        self.memory_types
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .map(|t| match t {
                        "foresight_record" | "FORESIGHT" => MemoryType::ForesightRecord,
                        "event_log_record" | "EVENT_LOG" => MemoryType::EventLogRecord,
                        "profile" | "PROFILE" => MemoryType::Profile,
                        "core_memory" | "CORE_MEMORY" | "CORE" => MemoryType::CoreMemory,
                        _ => MemoryType::EpisodicMemory,
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![MemoryType::All])
    }
}

#[derive(Debug, Serialize)]
pub struct SearchMemoriesResponse {
    pub memories: Vec<MemoryItem>,
    pub total_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// DELETE /api/v1/memories
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteMemoriesRequest {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    /// Specific record ID to delete
    pub event_id: Option<String>,
    pub memory_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteMemoriesResponse {
    pub deleted_count: u64,
}

// ───────────────────────────────────────────────────────────────────────────────
// POST /api/v1/memories/conversation-meta
// GET  /api/v1/memories/conversation-meta
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConversationMetaRequest {
    pub group_id: Option<String>,
    pub conv_id: Option<String>,
    pub user_id: Option<String>,
    pub scene: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConversationMetaQuery {
    pub group_id: Option<String>,
    pub conv_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConversationMetaResponse {
    pub conv_id: String,
    pub group_id: Option<String>,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

// ───────────────────────────────────────────────────────────────────────────────
// GET /api/v1/memories/status
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RequestStatusQuery {
    pub request_id: String,
}

#[derive(Debug, Serialize)]
pub struct RequestStatusResponse {
    pub request_id: String,
    pub found: bool,
    /// -1=pending, 0=processing, 1=done, -2=error
    pub sync_status: Option<i32>,
    pub status_label: String,
}

// ────────────────────────────────────────────────────────────────────────────
// PATCH /api/v1/memories/conversation-meta
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConversationMetaPatchBody {
    pub group_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub scene_desc: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
    pub user_details: Option<serde_json::Value>,
    pub default_timezone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConversationMetaPatchResponse {
    pub group_id: Option<String>,
    pub updated_fields: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// POST /api/v1/global-user-profile/custom
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CustomProfileData {
    pub initial_profile: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertCustomProfileRequest {
    pub user_id: String,
    pub custom_profile_data: CustomProfileData,
}

#[derive(Debug, Serialize)]
pub struct UpsertCustomProfileResponse {
    pub success: bool,
    pub user_id: String,
    pub message: Option<String>,
}
