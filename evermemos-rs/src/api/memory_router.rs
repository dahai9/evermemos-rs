use axum::{
    extract::{Extension, Query, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::agentic::manager::{AgenticManager, RetrieveRequest};
use crate::api::dto::{
    ApiResponse, ConversationMetaPatchBody, ConversationMetaPatchResponse,
    ConversationMetaQuery, ConversationMetaRequest, ConversationMetaResponse,
    DeleteMemoriesRequest, DeleteMemoriesResponse, FetchMemoriesQuery,
    FetchMemoriesResponse, MemorizeMessageRequest, MemorizeResponse, RequestStatusQuery,
    RequestStatusResponse, SearchMemoriesQuery, SearchMemoriesResponse,
};
use crate::biz::memorize::{MemorizeRequest, MemorizeService, RawMessage};
use crate::core::error::AppError;
use crate::core::tenant::TenantContext;
use crate::storage::models::ConversationMeta;
use crate::storage::repository::{ConversationMetaRepo, DateRange, EpisodicMemoryRepo, MemoryRequestLogRepo};

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    pub memorize_svc: Arc<MemorizeService>,
    pub agentic: Arc<AgenticManager>,
    pub ep_repo: EpisodicMemoryRepo,
    pub conv_meta_repo: ConversationMetaRepo,
    pub req_log_repo: MemoryRequestLogRepo,
}

pub fn memory_routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/memories",
            post(memorize_handler)
                .get(fetch_handler)
                .delete(delete_handler),
        )
        .route("/api/v1/memories/search", get(search_handler))
        .route(
            "/api/v1/memories/conversation-meta",
            post(conv_meta_upsert_handler)
                .get(conv_meta_get_handler)
                .patch(conv_meta_patch_handler),
        )
        .route("/api/v1/memories/status", get(request_status_handler))
        .with_state(state)
}

// ── POST /api/v1/memories ─────────────────────────────────────────────────────

async fn memorize_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Json(body): Json<MemorizeMessageRequest>,
) -> Result<Json<ApiResponse<MemorizeResponse>>, AppError> {
    info!(
        "POST /api/v1/memories user={:?} group={:?}",
        body.user_id, body.group_id
    );

    let req = MemorizeRequest {
        message: RawMessage {
            message_id: body.message_id,
            sender: body.sender,
            sender_name: body.sender_name,
            content: body.content,
            create_time: body.create_time,
            role: body.role,
        },
        user_id: body.user_id,
        user_name: body.user_name,
        group_id: body.group_id,
        group_name: body.group_name,
        history: body.history.unwrap_or_default(),
    };

    let result = state
        .memorize_svc
        .memorize(req)
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(ApiResponse::ok(
        "Memory processed",
        MemorizeResponse {
            status: result.status,
            message: result.message,
            saved_count: result.saved_count,
        },
    )))
}

// ── GET /api/v1/memories ──────────────────────────────────────────────────────

async fn fetch_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Query(q): Query<FetchMemoriesQuery>,
) -> Result<Json<ApiResponse<FetchMemoriesResponse>>, AppError> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    let date_range = DateRange {
        start: q.start_time,
        end: q.end_time,
    };

    let memories = state
        .ep_repo
        .list(
            q.user_id.as_deref(),
            q.group_id.as_deref(),
            &date_range,
            limit,
            offset,
        )
        .await
        .map_err(|e| AppError::Internal(e))?;

    let items: Vec<_> = memories
        .into_iter()
        .map(|m| crate::agentic::manager::MemoryItem {
            id: m.id.as_ref().map(|t| t.to_raw()).unwrap_or_default(),
            memory_type: "episodic_memory".into(),
            content: m.episode,
            score: 0.0,
            timestamp: Some(m.timestamp),
            metadata: serde_json::json!({
                "summary": m.summary,
                "subject": m.subject,
                "keywords": m.keywords,
            }),
        })
        .collect();

    let has_more = items.len() == limit as usize;
    let total = items.len();

    Ok(Json(ApiResponse::ok(
        "Memories fetched",
        FetchMemoriesResponse {
            memories: items,
            total_count: total,
            has_more,
        },
    )))
}

// ── GET /api/v1/memories/search ───────────────────────────────────────────────

async fn search_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Query(q): Query<SearchMemoriesQuery>,
) -> Result<Json<ApiResponse<SearchMemoriesResponse>>, AppError> {
    let method = q.parse_method();
    let memory_types = q.parse_memory_types();

    let req = RetrieveRequest {
        query: q.query,
        user_id: q.user_id,
        group_id: q.group_id,
        method,
        memory_types,
        top_k: q.top_k.unwrap_or(10).min(50),
        start_time: q.start_time,
        end_time: q.end_time,
        radius: q.radius,
    };

    let resp = state
        .agentic
        .retrieve(req)
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(ApiResponse::ok(
        "Search complete",
        SearchMemoriesResponse {
            total_count: resp.total_count,
            memories: resp.memories,
        },
    )))
}

// ── DELETE /api/v1/memories ───────────────────────────────────────────────────

async fn delete_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Json(body): Json<DeleteMemoriesRequest>,
) -> Result<Json<ApiResponse<DeleteMemoriesResponse>>, AppError> {
    if body.user_id.is_none() && body.group_id.is_none() && body.event_id.is_none() {
        return Err(AppError::BadRequest(
            "At least one of event_id, user_id, or group_id is required".into(),
        ));
    }

    let count = state
        .ep_repo
        .soft_delete_by_filter(body.user_id.as_deref(), body.group_id.as_deref())
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(ApiResponse::ok(
        "Memories deleted",
        DeleteMemoriesResponse {
            deleted_count: count,
        },
    )))
}

// ── POST /api/v1/memories/conversation-meta ───────────────────────────────────

async fn conv_meta_upsert_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Json(body): Json<ConversationMetaRequest>,
) -> Result<Json<ApiResponse<ConversationMetaResponse>>, AppError> {
    let conv_id = body.conv_id.clone().or_else(|| body.group_id.clone()).unwrap_or_default();
    let meta = ConversationMeta {
        id: None,
        conv_id: conv_id.clone(),
        user_id: body.user_id.clone(),
        group_id: body.group_id.clone(),
        name: body.name.clone(),
        description: None,
        scene: body.scene.clone(),
        scene_desc: None,
        tags: None,
        user_details: None,
        default_timezone: None,
        title: body.title.clone().or_else(|| body.name.clone()),
        summary: body.summary.clone(),
        created_at: None,
        updated_at: None,
    };

    let saved = if body.group_id.is_some() {
        state
            .conv_meta_repo
            .upsert_by_group_id(meta)
            .await
            .map_err(|e| AppError::Internal(e))?
    } else {
        state
            .conv_meta_repo
            .upsert(meta)
            .await
            .map_err(|e| AppError::Internal(e))?
    };

    Ok(Json(ApiResponse::ok(
        "Conversation meta saved",
        ConversationMetaResponse {
            conv_id: saved.conv_id,
            group_id: saved.group_id,
            user_id: saved.user_id,
            title: saved.title,
            summary: saved.summary,
        },
    )))
}

// ── GET /api/v1/memories/conversation-meta ────────────────────────────────────

async fn conv_meta_get_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Query(q): Query<ConversationMetaQuery>,
) -> Result<Json<ApiResponse<Option<ConversationMetaResponse>>>, AppError> {
    let meta = if let Some(gid) = q.group_id.as_deref() {
        state
            .conv_meta_repo
            .get_by_group_id(gid)
            .await
            .map_err(|e| AppError::Internal(e))?
    } else if let Some(cid) = q.conv_id.as_deref() {
        state
            .conv_meta_repo
            .get(cid)
            .await
            .map_err(|e| AppError::Internal(e))?
    } else {
        None
    };

    let resp = meta.map(|m| ConversationMetaResponse {
        conv_id: m.conv_id,
        group_id: m.group_id,
        user_id: m.user_id,
        title: m.title,
        summary: m.summary,
    });

    Ok(Json(ApiResponse::ok("Conversation meta retrieved", resp)))
}

// ── GET /api/v1/memories/status ───────────────────────────────────────────────

// ── PATCH /api/v1/memories/conversation-meta ─────────────────────────────────

async fn conv_meta_patch_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Json(body): Json<ConversationMetaPatchBody>,
) -> Result<Json<ApiResponse<ConversationMetaPatchResponse>>, AppError> {
    let group_id = body.group_id.as_deref().unwrap_or("").to_string();
    if group_id.is_empty() {
        return Err(AppError::BadRequest("group_id is required for PATCH".into()));
    }

    let mut patch = serde_json::Map::new();
    let mut updated_fields: Vec<String> = Vec::new();
    macro_rules! maybe_set {
        ($key:expr, $val:expr) => {
            if let Some(v) = $val {
                patch.insert($key.to_string(), serde_json::to_value(v).unwrap_or_default());
                updated_fields.push($key.to_string());
            }
        };
    }
    maybe_set!("name",             body.name);
    maybe_set!("description",      body.description);
    maybe_set!("scene_desc",       body.scene_desc);
    maybe_set!("tags",             body.tags);
    maybe_set!("user_details",     body.user_details);
    maybe_set!("default_timezone", body.default_timezone);

    if patch.is_empty() {
        return Ok(Json(ApiResponse::ok(
            "No fields to update",
            ConversationMetaPatchResponse { group_id: Some(group_id), updated_fields: vec![] },
        )));
    }

    let patched = state
        .conv_meta_repo
        .patch_by_group_id(&group_id, serde_json::Value::Object(patch))
        .await
        .map_err(AppError::Internal)?;

    if patched.is_none() {
        return Err(AppError::NotFound(format!(
            "conversation-meta not found for group_id={group_id}"
        )));
    }

    Ok(Json(ApiResponse::ok(
        "Conversation meta patched",
        ConversationMetaPatchResponse {
            group_id: Some(group_id),
            updated_fields,
        },
    )))
}

async fn request_status_handler(
    State(state): State<AppState>,
    Extension(_tenant): Extension<TenantContext>,
    Query(q): Query<RequestStatusQuery>,
) -> Result<Json<ApiResponse<RequestStatusResponse>>, AppError> {
    let log = state
        .req_log_repo
        .get_by_message_id(&q.request_id)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let (found, sync_status, label) = match log {
        Some(entry) => {
            let label = match entry.sync_status {
                -1 => "pending",
                0  => "processing",
                1  => "done",
                -2 => "error",
                _  => "unknown",
            };
            (true, Some(entry.sync_status), label.to_string())
        }
        None => (false, None, "not_found".to_string()),
    };

    Ok(Json(ApiResponse::ok(
        "Request status retrieved",
        RequestStatusResponse {
            request_id: q.request_id,
            found,
            sync_status,
            status_label: label,
        },
    )))
}
