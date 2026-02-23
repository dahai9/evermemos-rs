use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialise `tracing-subscriber` from the `RUST_LOG` environment variable.
/// Falls back to `info` level if `RUST_LOG` is unset.
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("evermemos=info,tower_http=warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact())
        .init();
}
