//! Compatibility shim — delegates to `core::telemetry`.
//!
//! Existing callers that import `core::tracing as app_tracing` and call
//! `app_tracing::init()` continue to work unchanged.
//! The returned `TelemetryGuard` must be held for the process lifetime:
//! ```ignore
//! let _telemetry = app_tracing::init();
//! ```

pub use super::telemetry::{init, TelemetryGuard};
