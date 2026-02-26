use axum::{
    extract::{Path, Query, State},
    routing::{delete, get},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::core::error::AppError;
use crate::storage::models::BehaviorHistory;
use crate::storage::repository::BehaviorHistoryRepo;

use super::dto::ApiResponse;

#[derive(Clone)]
pub struct BehaviorHistoryState {
    pub bh_repo: BehaviorHistoryRepo,
}

pub fn behavior_history_routes(state: BehaviorHistoryState) -> Router {
    Router::new()
        .route(
            "/api/v1/behavior-history",
            get(list_handler).post(create_handler),
        )
        .route("/api/v1/behavior-history/{id}", delete(delete_handler))
        .route("/api/v1/behavior-history/stats", get(stats_handler))
        .with_state(state)
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub user_id: String,
    pub behavior_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct BehaviorHistoryItem {
    pub id: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub behavior_type: Vec<String>,
    pub event_id: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<BehaviorHistory> for BehaviorHistoryItem {
    fn from(b: BehaviorHistory) -> Self {
        Self {
            id: b.id.as_ref().map(|t| t.to_raw()).unwrap_or_default(),
            user_id: b.user_id,
            timestamp: b.timestamp,
            behavior_type: b.behavior_type,
            event_id: b.event_id,
            meta: b.meta,
            created_at: b.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub user_id: String,
    pub behavior_type: Vec<String>,
    pub event_id: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub extend: Option<serde_json::Value>,
    /// Defaults to now if not provided
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub records: Vec<BehaviorHistoryItem>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub user_id: String,
    pub total_records: u64,
    pub type_breakdown: serde_json::Value,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/behavior-history?user_id=...&limit=...
async fn list_handler(
    State(state): State<BehaviorHistoryState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<ListResponse>>, AppError> {
    let limit = q.limit.unwrap_or(50).min(500);

    let records = if let (Some(start), Some(end)) = (q.start_time, q.end_time) {
        state
            .bh_repo
            .get_by_time_range(Some(&q.user_id), start, end, limit)
            .await
            .map_err(AppError::Internal)?
    } else if let Some(btype) = &q.behavior_type {
        state
            .bh_repo
            .get_by_type(&q.user_id, btype, limit)
            .await
            .map_err(AppError::Internal)?
    } else {
        state
            .bh_repo
            .get_by_user_id(&q.user_id, limit)
            .await
            .map_err(AppError::Internal)?
    };

    let total = records.len();
    let items: Vec<BehaviorHistoryItem> = records.into_iter().map(Into::into).collect();

    info!("GET /api/v1/behavior-history user={} returned {}", q.user_id, total);

    Ok(Json(ApiResponse::ok(
        "Behavior history fetched",
        ListResponse { records: items, total },
    )))
}

/// POST /api/v1/behavior-history
async fn create_handler(
    State(state): State<BehaviorHistoryState>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<ApiResponse<BehaviorHistoryItem>>, AppError> {
    if body.user_id.is_empty() {
        return Err(AppError::BadRequest("user_id is required".into()));
    }
    if body.behavior_type.is_empty() {
        return Err(AppError::BadRequest("behavior_type must not be empty".into()));
    }

    let now = Utc::now();
    let record = BehaviorHistory {
        id: None,
        user_id: body.user_id.clone(),
        timestamp: body.timestamp.unwrap_or(now),
        behavior_type: body.behavior_type,
        event_id: body.event_id,
        meta: body.meta,
        extend: body.extend,
        is_deleted: false,
        created_at: Some(now),
        updated_at: Some(now),
    };

    let saved = state
        .bh_repo
        .insert(record)
        .await
        .map_err(AppError::Internal)?;

    info!(
        "POST /api/v1/behavior-history created for user={}",
        body.user_id
    );

    Ok(Json(ApiResponse::ok("Behavior history created", saved.into())))
}

/// DELETE /api/v1/behavior-history/:id
async fn delete_handler(
    State(state): State<BehaviorHistoryState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    state
        .bh_repo
        .soft_delete(&id)
        .await
        .map_err(AppError::Internal)?;

    info!("DELETE /api/v1/behavior-history/{id}");

    Ok(Json(ApiResponse::ok(
        "Deleted",
        serde_json::json!({ "id": id }),
    )))
}

/// GET /api/v1/behavior-history/stats?user_id=...
async fn stats_handler(
    State(state): State<BehaviorHistoryState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<StatsResponse>>, AppError> {
    let total = state
        .bh_repo
        .count_by_user(&q.user_id)
        .await
        .map_err(AppError::Internal)?;

    // Fetch recent 200 to compute type breakdown
    let records = state
        .bh_repo
        .get_by_user_id(&q.user_id, 200)
        .await
        .unwrap_or_default();

    let mut breakdown: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for r in &records {
        for bt in &r.behavior_type {
            *breakdown.entry(bt.clone()).or_insert(0) += 1;
        }
    }

    Ok(Json(ApiResponse::ok(
        "Stats computed",
        StatsResponse {
            user_id: q.user_id,
            total_records: total,
            type_breakdown: serde_json::to_value(breakdown).unwrap_or_default(),
        },
    )))
}
