use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Vectorize error: {0}")]
    Vectorize(String),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Llm(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Vectorize(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        };

        let body = json!({
            "status": "error",
            "message": message,
            "result": null
        });

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn not_found_status() {
        let err = AppError::NotFound("memory not found".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn bad_request_status() {
        let err = AppError::BadRequest("invalid param".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unauthorized_status() {
        let err = AppError::Unauthorized;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn llm_error_is_bad_gateway() {
        let err = AppError::Llm("timeout".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn error_display() {
        let err = AppError::NotFound("x".into());
        assert!(err.to_string().contains("Not found"));
    }
}
