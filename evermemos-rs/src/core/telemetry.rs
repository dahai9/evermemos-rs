//! OpenTelemetry initialisation + graceful shutdown.
//!
//! Controlled by the `OTEL_ENABLED` env var (default: false).
//! When enabled, spans and metrics are exported to an OTLP collector
//! via HTTP/protobuf on the endpoint declared in `OTEL_EXPORTER_OTLP_ENDPOINT`
//! (default: http://localhost:4318).
//!
//! The returned `TelemetryGuard` **must** be held for the process lifetime.
//! Dropping it flushes in-flight spans and metrics before the process exits.

use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ── public guard ─────────────────────────────────────────────────────────────

/// Holds both providers alive. Drop → graceful flush.
pub struct TelemetryGuard {
    otel_enabled: bool,
    tracer_provider_opt: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    meter_provider_opt: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if !self.otel_enabled {
            return;
        }
        if let Some(tp) = self.tracer_provider_opt.take() {
            if let Err(e) = tp.shutdown() {
                eprintln!("[otel] tracer provider shutdown error: {e:?}");
            }
        }
        if let Some(mp) = self.meter_provider_opt.take() {
            if let Err(e) = mp.shutdown() {
                eprintln!("[otel] meter provider shutdown error: {e:?}");
            }
        }
    }
}

// ── public entry point ────────────────────────────────────────────────────────

/// Initialise tracing (and optionally OTLP export).
///
/// Always returns a `TelemetryGuard`.  Hold it for the process lifetime:
/// ```ignore
/// let _telemetry = evermemos_rs::core::telemetry::init();
/// ```
pub fn init() -> TelemetryGuard {
    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("evermemos=info,tower_http=warn"));

    if otel_enabled {
        init_with_otel(filter)
    } else {
        init_fmt_only(filter)
    }
}

// ── fmt-only (no OTLP) ───────────────────────────────────────────────────────

fn init_fmt_only(filter: EnvFilter) -> TelemetryGuard {
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    TelemetryGuard {
        otel_enabled: false,
        tracer_provider_opt: None,
        meter_provider_opt: None,
    }
}

// ── OTLP-enabled ─────────────────────────────────────────────────────────────

fn init_with_otel(filter: EnvFilter) -> TelemetryGuard {
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::{Protocol, WithExportConfig};
    use opentelemetry_sdk::{
        metrics::SdkMeterProvider,
        trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
        Resource,
    };

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());

    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "evermemos-rs".to_string());

    let resource = Resource::builder_empty()
        .with_attributes([KeyValue::new("service.name", service_name.clone())])
        .build();

    // ── Trace provider ───────────────────────────────────────────────────────
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(format!("{endpoint}/v1/traces"))
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build OTLP span exporter");

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource.clone())
        .build();

    // Register globally so `tracing-opentelemetry` layer can reach it
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    // ── Meter provider ───────────────────────────────────────────────────────
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build OTLP metric exporter");

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(resource)
        .build();

    opentelemetry::global::set_meter_provider(meter_provider.clone());

    // ── Tracer for bridge layer ───────────────────────────────────────────────
    use opentelemetry::trace::TracerProvider as _;
    let tracer = tracer_provider.tracer(service_name);

    // ── tracing subscriber: fmt + OTEL layer ─────────────────────────────────
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    tracing::info!("[otel] OpenTelemetry enabled, exporting to {endpoint}");

    TelemetryGuard {
        otel_enabled: true,
        tracer_provider_opt: Some(tracer_provider),
        meter_provider_opt: Some(meter_provider),
    }
}
