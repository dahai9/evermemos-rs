use anyhow::Context;
use surrealdb::{
    engine::any::{connect, Any},
    opt::auth::Root,
    Surreal,
};
use tracing::info;

use super::schema;
use crate::config::SurrealConfig;

/// Type-alias for the universally-typed SurrealDB client.
pub type Db = Surreal<Any>;

/// Connect to SurrealDB (embedded RocksDB or remote) and run the schema DDL.
///
/// `endpoint` examples:
/// - `"rocksdb://./data/surreal"` — embedded, zero-config (mobile/single-binary)
/// - `"mem://"`                   — in-memory (testing)
/// - `"ws://localhost:8000"`       — remote SurrealDB server
pub async fn init(cfg: &SurrealConfig) -> anyhow::Result<Db> {
    info!("Connecting to SurrealDB at {}", cfg.endpoint);

    let db: Db = connect(&cfg.endpoint)
        .await
        .with_context(|| format!("Failed to connect to SurrealDB at {}", cfg.endpoint))?;

    // Authenticate only for remote server connections.
    // Embedded engines (rocksdb://, mem://, file://) do not support Root sign-in.
    let is_embedded = cfg.endpoint.starts_with("rocksdb://")
        || cfg.endpoint.starts_with("mem://")
        || cfg.endpoint.starts_with("file://");

    if !is_embedded && !cfg.user.is_empty() {
        db.signin(Root {
            username: &cfg.user,
            password: &cfg.pass,
        })
        .await
        .context("SurrealDB authentication failed")?;
    }

    db.use_ns(&cfg.ns)
        .use_db(&cfg.db)
        .await
        .context("Failed to select namespace/database")?;

    info!("SurrealDB connected — ns={} db={}", cfg.ns, cfg.db);

    // Apply DDL (idempotent — all statements use IF NOT EXISTS semantics)
    schema::apply(&db).await?;

    Ok(db)
}
