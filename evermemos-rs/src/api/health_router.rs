use axum::{routing::get, Json, Router};
use serde_json::json;

pub fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "evermemos-rs",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
