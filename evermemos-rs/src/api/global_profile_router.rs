use axum::{
    extract::{Extension, State},
    routing::post,
    Json, Router,
};
use tracing::info;

use crate::api::dto::{ApiResponse, UpsertCustomProfileRequest, UpsertCustomProfileResponse};
use crate::core::error::AppError;
use crate::core::tenant::TenantContext;
use crate::storage::repository::UserProfileRepo;

#[derive(Clone)]
pub struct GlobalProfileState {
    pub up_repo: UserProfileRepo,
}

pub fn global_profile_routes(state: GlobalProfileState) -> Router {
    Router::new()
        .route(
            "/api/v1/global-user-profile/custom",
            post(upsert_custom_profile_handler),
        )
        .with_state(state)
}

// ── POST /api/v1/global-user-profile/custom ──────────────────────────────────

async fn upsert_custom_profile_handler(
    State(state): State<GlobalProfileState>,
    Extension(_tenant): Extension<TenantContext>,
    Json(body): Json<UpsertCustomProfileRequest>,
) -> Result<Json<ApiResponse<UpsertCustomProfileResponse>>, AppError> {
    if body.user_id.is_empty() {
        return Err(AppError::BadRequest("user_id is required".into()));
    }
    if body.custom_profile_data.initial_profile.is_empty() {
        return Err(AppError::BadRequest(
            "custom_profile_data.initial_profile is required and cannot be empty".into(),
        ));
    }

    info!(
        "POST /api/v1/global-user-profile/custom user_id={}",
        body.user_id
    );

    let data = serde_json::json!({
        "initial_profile": body.custom_profile_data.initial_profile
    });

    state
        .up_repo
        .upsert_custom_profile(&body.user_id, data)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(ApiResponse::ok(
        "Custom profile upserted",
        UpsertCustomProfileResponse {
            success: true,
            user_id: body.user_id,
            message: None,
        },
    )))
}
