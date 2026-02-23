/// SurrealDB DDL — applied once at startup (all statements are idempotent).
///
/// Replaces:
///   MongoDB   — DEFINE TABLE + FIELDs
///   Elasticsearch — DEFINE INDEX … SEARCH ANALYZER … BM25
///   Milvus    — DEFINE INDEX … HNSW DIMENSION 1024 DIST COSINE
use anyhow::Context;
use tracing::info;

use super::db::Db;

/// All DDL executed in a single query batch.
const SCHEMA_SQL: &str = r#"

-- ─────────────────────────────────────────────────────────────────────────────
-- Full-text analysers
-- ─────────────────────────────────────────────────────────────────────────────

DEFINE ANALYZER IF NOT EXISTS ana_en
    TOKENIZERS class
    FILTERS lowercase, snowball(english);

-- Chinese: store pre-tokenised (space-separated) tokens produced by jieba-rs
DEFINE ANALYZER IF NOT EXISTS ana_zh
    TOKENIZERS blank
    FILTERS lowercase;

-- ─────────────────────────────────────────────────────────────────────────────
-- memcell — boundary-detected raw conversation unit
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS memcell SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id         ON memcell TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id        ON memcell TYPE option<string>;
DEFINE FIELD IF NOT EXISTS timestamp       ON memcell TYPE any;
DEFINE FIELD IF NOT EXISTS summary         ON memcell TYPE option<string>;
DEFINE FIELD IF NOT EXISTS original_data   ON memcell TYPE option<array>;
DEFINE FIELD IF NOT EXISTS participants    ON memcell TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS subject         ON memcell TYPE option<string>;
DEFINE FIELD IF NOT EXISTS keywords        ON memcell TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS is_deleted      ON memcell TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at      ON memcell TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at      ON memcell TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_memcell_user  ON memcell FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_memcell_group ON memcell FIELDS group_id;
DEFINE INDEX IF NOT EXISTS idx_memcell_ts    ON memcell FIELDS timestamp;

-- ─────────────────────────────────────────────────────────────────────────────
-- episodic_memory — episode narratives (replaces MongoDB + ES + Milvus)
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS episodic_memory SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id          ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS user_name        ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id         ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_name       ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS timestamp        ON episodic_memory TYPE any;
DEFINE FIELD IF NOT EXISTS participants     ON episodic_memory TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS summary          ON episodic_memory TYPE string;
DEFINE FIELD IF NOT EXISTS episode          ON episodic_memory TYPE string;
DEFINE FIELD IF NOT EXISTS subject          ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS keywords         ON episodic_memory TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS memcell_ids      ON episodic_memory TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS vector           ON episodic_memory TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS vector_model     ON episodic_memory TYPE option<string>;
-- search_content = subject + " " + episode (pre-computed for BM25)
DEFINE FIELD IF NOT EXISTS search_content   ON episodic_memory TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted       ON episodic_memory TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at       ON episodic_memory TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at       ON episodic_memory TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_ep_user   ON episodic_memory FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_ep_group  ON episodic_memory FIELDS group_id;
DEFINE INDEX IF NOT EXISTS idx_ep_ts     ON episodic_memory FIELDS timestamp;

-- BM25 full-text (replaces Elasticsearch)
DEFINE INDEX IF NOT EXISTS idx_ep_fts
    ON episodic_memory
    FIELDS search_content
    SEARCH ANALYZER ana_en BM25;

-- HNSW vector search (replaces Milvus)
DEFINE INDEX IF NOT EXISTS idx_ep_vec
    ON episodic_memory
    FIELDS vector
    HNSW DIMENSION 1024 DIST COSINE EFC 200 M 16;

-- ─────────────────────────────────────────────────────────────────────────────
-- foresight_record — predictive memories (assistant scenes only)
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS foresight_record SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id          ON foresight_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id         ON foresight_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS timestamp        ON foresight_record TYPE any;
DEFINE FIELD IF NOT EXISTS foresight        ON foresight_record TYPE string;
DEFINE FIELD IF NOT EXISTS evidence         ON foresight_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS start_time       ON foresight_record TYPE any;
DEFINE FIELD IF NOT EXISTS end_time         ON foresight_record TYPE any;
DEFINE FIELD IF NOT EXISTS duration_days    ON foresight_record TYPE option<int>;
DEFINE FIELD IF NOT EXISTS vector           ON foresight_record TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS search_content   ON foresight_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted       ON foresight_record TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at       ON foresight_record TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at       ON foresight_record TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_fs_user  ON foresight_record FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_fs_group ON foresight_record FIELDS group_id;

DEFINE INDEX IF NOT EXISTS idx_fs_fts
    ON foresight_record
    FIELDS search_content
    SEARCH ANALYZER ana_en BM25;

DEFINE INDEX IF NOT EXISTS idx_fs_vec
    ON foresight_record
    FIELDS vector
    HNSW DIMENSION 1024 DIST COSINE EFC 200 M 16;

-- ─────────────────────────────────────────────────────────────────────────────
-- event_log_record — atomic facts (assistant scenes only)
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS event_log_record SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id        ON event_log_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id       ON event_log_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS timestamp      ON event_log_record TYPE any;
DEFINE FIELD IF NOT EXISTS atomic_fact    ON event_log_record TYPE string;
DEFINE FIELD IF NOT EXISTS vector         ON event_log_record TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS search_content ON event_log_record TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted     ON event_log_record TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at     ON event_log_record TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at     ON event_log_record TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_el_user  ON event_log_record FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_el_group ON event_log_record FIELDS group_id;

DEFINE INDEX IF NOT EXISTS idx_el_fts
    ON event_log_record
    FIELDS search_content
    SEARCH ANALYZER ana_en BM25;

DEFINE INDEX IF NOT EXISTS idx_el_vec
    ON event_log_record
    FIELDS vector
    HNSW DIMENSION 1024 DIST COSINE EFC 200 M 16;

-- ─────────────────────────────────────────────────────────────────────────────
-- user_profile — user characteristics
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS user_profile SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id       ON user_profile TYPE string;
DEFINE FIELD IF NOT EXISTS profile_data  ON user_profile TYPE option<object>;
DEFINE FIELD IF NOT EXISTS life_summary  ON user_profile TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted    ON user_profile TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at    ON user_profile TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at    ON user_profile TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_up_user ON user_profile FIELDS user_id UNIQUE;

-- ─────────────────────────────────────────────────────────────────────────────
-- cluster_state — MemCell clustering metadata
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS cluster_state SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id         ON cluster_state TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id        ON cluster_state TYPE option<string>;
DEFINE FIELD IF NOT EXISTS cluster_id      ON cluster_state TYPE string;
DEFINE FIELD IF NOT EXISTS memcell_ids     ON cluster_state TYPE array<string>;
DEFINE FIELD IF NOT EXISTS centroid        ON cluster_state TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS last_updated    ON cluster_state TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS created_at      ON cluster_state TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_cs_user    ON cluster_state FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_cs_cluster ON cluster_state FIELDS cluster_id UNIQUE;

-- ─────────────────────────────────────────────────────────────────────────────
-- conversation_meta — metadata per conversation session
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS conversation_meta SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS conv_id     ON conversation_meta TYPE string;
DEFINE FIELD IF NOT EXISTS user_id     ON conversation_meta TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id    ON conversation_meta TYPE option<string>;
DEFINE FIELD IF NOT EXISTS title       ON conversation_meta TYPE option<string>;
DEFINE FIELD IF NOT EXISTS summary     ON conversation_meta TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at  ON conversation_meta TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at  ON conversation_meta TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_cm_conv  ON conversation_meta FIELDS conv_id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_cm_user  ON conversation_meta FIELDS user_id;

-- ─────────────────────────────────────────────────────────────────────────────
-- memory_request_log — message ingestion audit log + pending queue
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS memory_request_log SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS message_id   ON memory_request_log TYPE string;
DEFINE FIELD IF NOT EXISTS user_id      ON memory_request_log TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id     ON memory_request_log TYPE option<string>;
DEFINE FIELD IF NOT EXISTS payload      ON memory_request_log TYPE object;
-- sync_status: -1=pending, 0=processing, 1=done, -2=error
DEFINE FIELD IF NOT EXISTS sync_status  ON memory_request_log TYPE int DEFAULT -1;
DEFINE FIELD IF NOT EXISTS created_at   ON memory_request_log TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at   ON memory_request_log TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_mrl_msg    ON memory_request_log FIELDS message_id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_mrl_status ON memory_request_log FIELDS sync_status;

"#;

pub async fn apply(db: &Db) -> anyhow::Result<()> {
    info!("Applying SurrealDB schema DDL…");
    db.query(SCHEMA_SQL)
        .await
        .context("Failed to apply SurrealDB schema")?;
    info!("SurrealDB schema applied successfully");
    Ok(())
}
