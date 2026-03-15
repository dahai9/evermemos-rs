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
DEFINE FIELD IF NOT EXISTS user_id              ON user_profile TYPE string;
DEFINE FIELD IF NOT EXISTS profile_data         ON user_profile TYPE option<object> FLEXIBLE;
DEFINE FIELD IF NOT EXISTS custom_profile_data  ON user_profile TYPE option<object> FLEXIBLE;
DEFINE FIELD IF NOT EXISTS life_summary         ON user_profile TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted           ON user_profile TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at           ON user_profile TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at           ON user_profile TYPE any DEFAULT time::now();

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
-- group_profile — aggregated group-chat profile (topics, summary, subject)
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS group_profile SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS group_id    ON group_profile TYPE string;
DEFINE FIELD IF NOT EXISTS group_name  ON group_profile TYPE option<string>;
-- topics: serialised JSON array of TopicInfo objects
DEFINE FIELD IF NOT EXISTS topics      ON group_profile TYPE option<array>;
DEFINE FIELD IF NOT EXISTS summary     ON group_profile TYPE option<string>;
DEFINE FIELD IF NOT EXISTS subject     ON group_profile TYPE option<string>;
DEFINE FIELD IF NOT EXISTS is_deleted  ON group_profile TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at  ON group_profile TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at  ON group_profile TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_gp_group ON group_profile FIELDS group_id UNIQUE;

-- ─────────────────────────────────────────────────────────────────────────────
-- memory_request_log — message ingestion audit log + pending queue
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS memory_request_log SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS message_id   ON memory_request_log TYPE string;
DEFINE FIELD IF NOT EXISTS user_id      ON memory_request_log TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id     ON memory_request_log TYPE option<string>;
DEFINE FIELD IF NOT EXISTS payload      ON memory_request_log TYPE object FLEXIBLE;
-- sync_status: -1=pending, 0=processing, 1=done, -2=error
DEFINE FIELD IF NOT EXISTS sync_status  ON memory_request_log TYPE int DEFAULT -1;
DEFINE FIELD IF NOT EXISTS created_at   ON memory_request_log TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at   ON memory_request_log TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_mrl_msg    ON memory_request_log FIELDS message_id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_mrl_status ON memory_request_log FIELDS sync_status;

-- ─────────────────────────────────────────────────────────────────────────────
-- behavior_history — user behavior tracking (chat, vote, file ops, etc.)
-- Mirrors Python BehaviorHistory MongoDB document.
-- ─────────────────────────────────────────────────────────────────────────────
DEFINE TABLE IF NOT EXISTS behavior_history SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id        ON behavior_history TYPE string;
DEFINE FIELD IF NOT EXISTS timestamp      ON behavior_history TYPE any;
-- behavior_type: array of tags e.g. ["chat", "follow-up"]
DEFINE FIELD IF NOT EXISTS behavior_type  ON behavior_history TYPE array<string>;
DEFINE FIELD IF NOT EXISTS event_id       ON behavior_history TYPE option<string>;
DEFINE FIELD IF NOT EXISTS meta           ON behavior_history TYPE option<object> FLEXIBLE;
DEFINE FIELD IF NOT EXISTS extend         ON behavior_history TYPE option<object> FLEXIBLE;
DEFINE FIELD IF NOT EXISTS is_deleted     ON behavior_history TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at     ON behavior_history TYPE any DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at     ON behavior_history TYPE any DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_bh_user      ON behavior_history FIELDS user_id;
DEFINE INDEX IF NOT EXISTS idx_bh_ts        ON behavior_history FIELDS timestamp;
DEFINE INDEX IF NOT EXISTS idx_bh_type      ON behavior_history FIELDS behavior_type;
DEFINE INDEX IF NOT EXISTS idx_bh_event     ON behavior_history FIELDS event_id;
DEFINE INDEX IF NOT EXISTS idx_bh_user_ts   ON behavior_history FIELDS user_id, timestamp;

-- ─────────────────────────────────────────────────────────────────────────────
-- Graph Relations (Spatiotemporal Graphs)
-- ─────────────────────────────────────────────────────────────────────────────

-- user table (nodes)
DEFINE TABLE IF NOT EXISTS user SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS user_id ON user TYPE string;
DEFINE INDEX IF NOT EXISTS idx_user_id ON user FIELDS user_id UNIQUE;

-- location table (nodes)
DEFINE TABLE IF NOT EXISTS location SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON location TYPE string;
DEFINE FIELD IF NOT EXISTS coords ON location TYPE option<geometry<point>>;
DEFINE INDEX IF NOT EXISTS idx_location_name ON location FIELDS name UNIQUE;

-- experienced relation: user -> experienced -> episodic_memory
DEFINE TABLE IF NOT EXISTS experienced TYPE RELATION;
DEFINE FIELD IF NOT EXISTS timestamp ON experienced TYPE any DEFAULT time::now();

-- produced relation: memcell -> produced -> (episodic_memory | foresight_record | event_log_record)
DEFINE TABLE IF NOT EXISTS produced TYPE RELATION;

-- located_at relation: (episodic_memory | user) -> located_at -> location
DEFINE TABLE IF NOT EXISTS located_at TYPE RELATION;
DEFINE FIELD IF NOT EXISTS timestamp ON located_at TYPE any DEFAULT time::now();

"#;

pub async fn apply(db: &Db) -> anyhow::Result<()> {
    info!("Applying SurrealDB schema DDL…");
    db.query(SCHEMA_SQL)
        .await
        .context("Failed to apply SurrealDB schema")?;
    info!("SurrealDB schema applied successfully");
    Ok(())
}
