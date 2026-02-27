use axum::{routing::get, Json, Router};
use serde_json::json;

pub fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(health))
}

async fn health() -> Json<serde_json::Value> {
    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());

    Json(json!({
        "status": "ok",
        "service": "evermemos-rs",
        "version": env!("CARGO_PKG_VERSION"),
        "observability": {
            "otel_enabled": otel_enabled,
            "otlp_endpoint": if otel_enabled { otel_endpoint } else { "disabled".to_string() }
        }
    }))
}

