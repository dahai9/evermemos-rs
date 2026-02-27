//! Axum middleware — HTTP request metrics via OpenTelemetry.
//!
//! Records two instruments per request:
//!   • `http.server.requests`          (Counter<u64>)
//!   • `http.server.request.duration`  (Histogram<f64>, milliseconds)
//!
//! Labels: `http.method`, `http.route`, `http.status_code`
//!
//! When OTEL is disabled, the global MeterProvider is a no-op provider
//! and all recording is effectively free (no allocations, no I/O).

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{Request, Response},
    middleware::Next,
};
use opentelemetry::{
    metrics::{Counter, Histogram, Meter},
    KeyValue,
};
use std::sync::OnceLock;
use std::time::Instant;

// ── instrument singletons ─────────────────────────────────────────────────────

struct HttpMetrics {
    requests: Counter<u64>,
    duration_ms: Histogram<f64>,
}

static HTTP_METRICS: OnceLock<HttpMetrics> = OnceLock::new();

fn get_metrics() -> &'static HttpMetrics {
    HTTP_METRICS.get_or_init(|| {
        let meter: Meter = opentelemetry::global::meter("evermemos.http");
        HttpMetrics {
            requests: meter
                .u64_counter("http.server.requests")
                .with_description("Total number of HTTP requests")
                .build(),
            duration_ms: meter
                .f64_histogram("http.server.request.duration")
                .with_description("HTTP request duration in milliseconds")
                .with_unit("ms")
                .build(),
        }
    })
}

// ── middleware ────────────────────────────────────────────────────────────────

/// Mount this with `axum::middleware::from_fn(metrics_middleware)`.
pub async fn metrics_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let method = req.method().to_string();
    // Use the matched route pattern (e.g. "/api/v1/memories/search") rather
    // than the raw path to avoid high-cardinality labels from path params.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let start = Instant::now();
    let resp = next.run(req).await;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;

    let status = resp.status().as_u16().to_string();
    let labels = [
        KeyValue::new("http.method", method),
        KeyValue::new("http.route", route),
        KeyValue::new("http.status_code", status),
    ];

    let m = get_metrics();
    m.requests.add(1, &labels);
    m.duration_ms.record(elapsed_ms, &labels);

    resp
}
