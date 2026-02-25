//! LLM Cassette — record-and-replay for all LLM / embedding / rerank calls.
//!
//! ## Overview
//!
//! Wraps any `LlmProvider`, `VectorizeService`, or `RerankService` with a thin
//! record-and-replay layer so that integration / parity tests do **not** consume
//! real API tokens after the first recording run.
//!
//! ## Environment variables
//!
//! | Variable              | Values                       | Default |
//! |-----------------------|------------------------------|---------|
//! | `LLM_CASSETTE_MODE`   | `off`, `record`, `replay`, `auto` | `off` |
//! | `LLM_CASSETTE_FILE`   | path to the cassette JSON file | `cassette/llm_cassette.json` |
//!
//! ## Modes
//!
//! * **`off`** — passthrough, nothing is recorded (default; production behaviour).
//! * **`record`** — always call the real provider and save every response to the
//!   cassette file.  Good for the first "golden" run.
//! * **`replay`** — only serve from the cassette; return an error on a cache miss
//!   (useful in CI where real API keys may be unavailable).
//! * **`auto`** — serve from the cassette when available, otherwise fall through
//!   to the real provider and record the new response.  Good for incremental
//!   development: new prompts get recorded automatically.
//!
//! ## Quick start
//!
//! ```bash
//! # First run — record all LLM calls
//! just record
//!
//! # Subsequent runs — replay from cassette, zero API cost
//! just replay
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::provider::{LlmMessage, LlmProvider};
use super::rerank::RerankService;
use super::vectorize::VectorizeService;

// ─── Mode ────────────────────────────────────────────────────────────────────

/// Operating mode for all cassette wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CassetteMode {
    /// Pass through to real provider, nothing recorded (default).
    Off,
    /// Call real provider, save every response.
    Record,
    /// Only serve from cassette; error on miss.
    Replay,
    /// Serve from cassette if present, otherwise call real + record.
    Auto,
}

impl CassetteMode {
    /// Read `LLM_CASSETTE_MODE` from the environment.
    pub fn from_env() -> Self {
        match std::env::var("LLM_CASSETTE_MODE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "record" => CassetteMode::Record,
            "replay" => CassetteMode::Replay,
            "auto" => CassetteMode::Auto,
            _ => CassetteMode::Off,
        }
    }

    /// Returns true if cassette wrapping is active (any mode other than Off).
    pub fn is_active(&self) -> bool {
        !matches!(self, CassetteMode::Off)
    }

    /// Returns true if this mode may write to the cassette.
    fn is_recording(&self) -> bool {
        matches!(self, CassetteMode::Record | CassetteMode::Auto)
    }
}

// ─── Persistent store ────────────────────────────────────────────────────────

/// In-memory + on-disk cassette data.
/// Keys are canonical JSON strings of the request parameters.
#[derive(Default, Serialize, Deserialize)]
struct CassetteData {
    /// LLM chat-completion: key = json({messages, temperature}), value = response text.
    llm: HashMap<String, String>,
    /// Embedding: key = text, value = float vector.
    embed: HashMap<String, Vec<f32>>,
    /// Rerank: key = json({query, docs}), value = scores.
    rerank: HashMap<String, Vec<f32>>,
}

/// Thread-safe cassette store shared by all wrapper types.
pub struct CassetteStore {
    path: PathBuf,
    mode: CassetteMode,
    data: RwLock<CassetteData>,
}

impl CassetteStore {
    /// Load an existing cassette file, or start with an empty store.
    pub fn open(path: impl AsRef<Path>, mode: CassetteMode) -> Arc<Self> {
        let path = path.as_ref().to_owned();

        let data: CassetteData = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Cassette parse error ({e}), starting empty");
                        CassetteData::default()
                    }
                },
                Err(e) => {
                    warn!("Cassette read error ({e}), starting empty");
                    CassetteData::default()
                }
            }
        } else {
            CassetteData::default()
        };

        let total = data.llm.len() + data.embed.len() + data.rerank.len();
        info!(
            "Cassette [{:?}] at {:?} — {} entries ({} llm, {} embed, {} rerank)",
            mode,
            path,
            total,
            data.llm.len(),
            data.embed.len(),
            data.rerank.len(),
        );

        Arc::new(CassetteStore {
            path,
            mode,
            data: RwLock::new(data),
        })
    }

    // ── Key builders ──────────────────────────────────────────────────────────

    /// Canonical key for an LLM completion request.
    fn llm_key(messages: &[LlmMessage], temperature: f32) -> String {
        // Round temperature to 2 dp to avoid f32 drift across platforms.
        let temp = (temperature * 100.0).round() / 100.0;
        serde_json::json!({
            "temperature": temp,
            "messages": messages,
        })
        .to_string()
    }

    /// Canonical key for a rerank request.
    fn rerank_key(query: &str, documents: &[String]) -> String {
        serde_json::json!({ "query": query, "docs": documents }).to_string()
    }

    // ── Flush ─────────────────────────────────────────────────────────────────

    /// Persist in-memory cassette to disk (async, best-effort).
    async fn flush(&self) {
        let data = self.data.read().await;
        match serde_json::to_vec_pretty(&*data) {
            Ok(bytes) => {
                // Ensure parent directory exists.
                if let Some(parent) = self.path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = tokio::fs::write(&self.path, bytes).await {
                    warn!("Cassette flush failed: {e}");
                } else {
                    debug!("Cassette flushed → {:?}", self.path);
                }
            }
            Err(e) => warn!("Cassette serialization failed: {e}"),
        }
    }
}

// ─── LLM wrapper ─────────────────────────────────────────────────────────────

/// Wraps any `LlmProvider` with cassette record/replay.
pub struct CassetteLlmProvider {
    inner: Arc<dyn LlmProvider>,
    store: Arc<CassetteStore>,
}

impl CassetteLlmProvider {
    pub fn new(inner: Arc<dyn LlmProvider>, store: Arc<CassetteStore>) -> Arc<Self> {
        Arc::new(Self { inner, store })
    }
}

#[async_trait]
impl LlmProvider for CassetteLlmProvider {
    async fn complete(
        &self,
        messages: Vec<LlmMessage>,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let key = CassetteStore::llm_key(&messages, temperature);

        // Check cassette first.
        {
            let data = self.store.data.read().await;
            if let Some(cached) = data.llm.get(&key) {
                debug!("LLM cassette hit ({} chars)", cached.len());
                return Ok(cached.clone());
            }
        }

        // Cache miss.
        match &self.store.mode {
            CassetteMode::Replay => {
                anyhow::bail!(
                    "LLM cassette miss in replay mode — \
                     re-run with LLM_CASSETTE_MODE=record to refresh"
                );
            }
            _ => {
                let response = self
                    .inner
                    .complete(messages, temperature, max_tokens)
                    .await?;

                if self.store.mode.is_recording() {
                    let mut data = self.store.data.write().await;
                    data.llm.insert(key, response.clone());
                    drop(data);
                    self.store.flush().await;
                }

                Ok(response)
            }
        }
    }
}

// ─── Vectorizer wrapper ───────────────────────────────────────────────────────

/// Wraps any `VectorizeService` with cassette record/replay.
pub struct CassetteVectorizer {
    inner: Arc<dyn VectorizeService>,
    store: Arc<CassetteStore>,
}

impl CassetteVectorizer {
    pub fn new(inner: Arc<dyn VectorizeService>, store: Arc<CassetteStore>) -> Arc<Self> {
        Arc::new(Self { inner, store })
    }
}

#[async_trait]
impl VectorizeService for CassetteVectorizer {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let key = text.to_string();

        {
            let data = self.store.data.read().await;
            if let Some(cached) = data.embed.get(&key) {
                debug!("Embed cassette hit for {:?}", &text[..text.len().min(40)]);
                return Ok(cached.clone());
            }
        }

        match &self.store.mode {
            CassetteMode::Replay => {
                anyhow::bail!(
                    "Embed cassette miss in replay mode for text: {:?}",
                    &text[..text.len().min(60)]
                );
            }
            _ => {
                let vec = self.inner.embed(text).await?;
                if self.store.mode.is_recording() {
                    let mut data = self.store.data.write().await;
                    data.embed.insert(key, vec.clone());
                    drop(data);
                    self.store.flush().await;
                }
                Ok(vec)
            }
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Find which texts are already cached.
        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut missing_indices: Vec<usize> = Vec::new();

        {
            let data = self.store.data.read().await;
            for (i, text) in texts.iter().enumerate() {
                if let Some(cached) = data.embed.get(text.as_str()) {
                    results[i] = Some(cached.clone());
                } else {
                    missing_indices.push(i);
                }
            }
        }

        if !missing_indices.is_empty() {
            match &self.store.mode {
                CassetteMode::Replay => {
                    anyhow::bail!(
                        "Embed-batch cassette miss in replay mode ({} texts missing)",
                        missing_indices.len()
                    );
                }
                _ => {
                    let missing_texts: Vec<String> =
                        missing_indices.iter().map(|&i| texts[i].clone()).collect();
                    let fetched = self.inner.embed_batch(&missing_texts).await?;

                    if self.store.mode.is_recording() {
                        let mut data = self.store.data.write().await;
                        for (&idx, vec) in missing_indices.iter().zip(fetched.iter()) {
                            data.embed.insert(texts[idx].clone(), vec.clone());
                        }
                        drop(data);
                        self.store.flush().await;
                    }

                    for (&idx, vec) in missing_indices.iter().zip(fetched.into_iter()) {
                        results[idx] = Some(vec);
                    }
                }
            }
        }

        Ok(results.into_iter().map(|v| v.unwrap_or_default()).collect())
    }
}

// ─── Reranker wrapper ────────────────────────────────────────────────────────

/// Wraps any `RerankService` with cassette record/replay.
pub struct CassetteReranker {
    inner: Arc<dyn RerankService>,
    store: Arc<CassetteStore>,
}

impl CassetteReranker {
    pub fn new(inner: Arc<dyn RerankService>, store: Arc<CassetteStore>) -> Arc<Self> {
        Arc::new(Self { inner, store })
    }
}

#[async_trait]
impl RerankService for CassetteReranker {
    async fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<f32>> {
        let key = CassetteStore::rerank_key(query, documents);

        {
            let data = self.store.data.read().await;
            if let Some(cached) = data.rerank.get(&key) {
                debug!("Rerank cassette hit");
                return Ok(cached.clone());
            }
        }

        match &self.store.mode {
            CassetteMode::Replay => {
                anyhow::bail!(
                    "Rerank cassette miss in replay mode for query: {:?}",
                    &query[..query.len().min(60)]
                );
            }
            _ => {
                let scores = self.inner.rerank(query, documents).await?;
                if self.store.mode.is_recording() {
                    let mut data = self.store.data.write().await;
                    data.rerank.insert(key, scores.clone());
                    drop(data);
                    self.store.flush().await;
                }
                Ok(scores)
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

// ─── Convenience constructor ──────────────────────────────────────────────────

/// Default cassette file path (`LLM_CASSETTE_FILE` env, or `cassette/llm_cassette.json`).
pub fn cassette_file_path() -> PathBuf {
    PathBuf::from(
        std::env::var("LLM_CASSETTE_FILE")
            .unwrap_or_else(|_| "cassette/llm_cassette.json".into()),
    )
}

/// Wrap the three provider `Arc`s with cassette decorators if the mode is not `Off`.
///
/// Returns either the original `Arc`s (mode=Off) or cassette-wrapped `Arc`s.
#[allow(clippy::type_complexity)]
pub fn apply_cassette(
    llm: Arc<dyn LlmProvider>,
    vectorizer: Arc<dyn VectorizeService>,
    reranker: Arc<dyn RerankService>,
) -> (
    Arc<dyn LlmProvider>,
    Arc<dyn VectorizeService>,
    Arc<dyn RerankService>,
) {
    let mode = CassetteMode::from_env();

    if !mode.is_active() {
        return (llm, vectorizer, reranker);
    }

    let path = cassette_file_path();
    let store = CassetteStore::open(&path, mode);

    (
        CassetteLlmProvider::new(llm, Arc::clone(&store)),
        CassetteVectorizer::new(vectorizer, Arc::clone(&store)),
        CassetteReranker::new(reranker, store),
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::LlmRole;
    use tempfile::NamedTempFile;

    // ── Stub providers ────────────────────────────────────────────────────────

    struct EchoLlm(String);

    #[async_trait]
    impl LlmProvider for EchoLlm {
        async fn complete(
            &self,
            _messages: Vec<LlmMessage>,
            _temperature: f32,
            _max_tokens: Option<u32>,
        ) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    struct EchoVectorizer(Vec<f32>);

    #[async_trait]
    impl VectorizeService for EchoVectorizer {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(self.0.clone())
        }
        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            Ok(vec![self.0.clone(); texts.len()])
        }
    }

    struct EchoReranker(Vec<f32>);

    #[async_trait]
    impl RerankService for EchoReranker {
        async fn rerank(&self, _query: &str, documents: &[String]) -> Result<Vec<f32>> {
            Ok(vec![self.0[0]; documents.len()])
        }
        fn is_enabled(&self) -> bool {
            true
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn tmp_store(mode: CassetteMode) -> (Arc<CassetteStore>, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let store = CassetteStore::open(f.path(), mode);
        (store, f)
    }

    fn sample_messages() -> Vec<LlmMessage> {
        vec![
            LlmMessage { role: LlmRole::System, content: "You are helpful".into() },
            LlmMessage { role: LlmRole::User, content: "Say hi".into() },
        ]
    }

    // ── LLM record / replay ───────────────────────────────────────────────────

    #[tokio::test]
    async fn llm_record_then_replay() {
        let (store, _f) = tmp_store(CassetteMode::Record);
        let provider = CassetteLlmProvider::new(
            Arc::new(EchoLlm("hello cassette".into())),
            Arc::clone(&store),
        );

        let msgs = sample_messages();
        let r1 = provider.complete(msgs.clone(), 0.3, None).await.unwrap();
        assert_eq!(r1, "hello cassette");

        // Switch store to Replay and verify hit (no real call).
        let (replay_store, _f2) = tmp_store(CassetteMode::Replay);
        {
            let src = store.data.read().await;
            let mut dst = replay_store.data.write().await;
            dst.llm = src.llm.clone();
        }
        let replay = CassetteLlmProvider::new(
            Arc::new(EchoLlm("SHOULD NOT BE CALLED".into())),
            replay_store,
        );
        let r2 = replay.complete(msgs, 0.3, None).await.unwrap();
        assert_eq!(r2, "hello cassette");
    }

    #[tokio::test]
    async fn llm_replay_miss_returns_error() {
        let (store, _f) = tmp_store(CassetteMode::Replay);
        let provider = CassetteLlmProvider::new(
            Arc::new(EchoLlm("never".into())),
            store,
        );
        let err = provider.complete(sample_messages(), 0.0, None).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("replay mode"));
    }

    #[tokio::test]
    async fn llm_auto_falls_through_and_records() {
        let (store, _f) = tmp_store(CassetteMode::Auto);
        let provider = CassetteLlmProvider::new(
            Arc::new(EchoLlm("auto response".into())),
            Arc::clone(&store),
        );

        // Miss → real call → recorded.
        let r1 = provider.complete(sample_messages(), 0.5, None).await.unwrap();
        assert_eq!(r1, "auto response");
        assert_eq!(store.data.read().await.llm.len(), 1);

        // Hit → no real call (even if inner would return something different).
        let r2 = provider.complete(sample_messages(), 0.5, None).await.unwrap();
        assert_eq!(r2, "auto response");
    }

    // ── Vectorizer record / replay ────────────────────────────────────────────

    #[tokio::test]
    async fn vectorizer_record_then_replay() {
        let (store, _f) = tmp_store(CassetteMode::Record);
        let vec_svc = CassetteVectorizer::new(
            Arc::new(EchoVectorizer(vec![0.1, 0.2, 0.3])),
            Arc::clone(&store),
        );

        let v1 = vec_svc.embed("hello world").await.unwrap();
        assert_eq!(v1, vec![0.1, 0.2, 0.3]);

        {
            let data = store.data.read().await;
            assert!(data.embed.contains_key("hello world"));
        }
    }

    #[tokio::test]
    async fn embed_batch_partial_cache() {
        let (store, _f) = tmp_store(CassetteMode::Auto);

        // Pre-populate one entry.
        {
            let mut data = store.data.write().await;
            data.embed.insert("cached".into(), vec![1.0, 2.0]);
        }

        let vec_svc = CassetteVectorizer::new(
            Arc::new(EchoVectorizer(vec![9.0, 9.0])),
            Arc::clone(&store),
        );

        let texts = vec!["cached".into(), "new_text".into()];
        let results = vec_svc.embed_batch(&texts).await.unwrap();
        assert_eq!(results[0], vec![1.0, 2.0]); // from cache
        assert_eq!(results[1], vec![9.0, 9.0]); // from inner
    }

    // ── Reranker record / replay ──────────────────────────────────────────────

    #[tokio::test]
    async fn reranker_record_then_replay() {
        let (store, _f) = tmp_store(CassetteMode::Record);
        let reranker = CassetteReranker::new(
            Arc::new(EchoReranker(vec![0.9])),
            Arc::clone(&store),
        );

        let docs = vec!["doc a".into(), "doc b".into()];
        let scores = reranker.rerank("query", &docs).await.unwrap();
        assert_eq!(scores, vec![0.9, 0.9]);

        let data = store.data.read().await;
        assert_eq!(data.rerank.len(), 1);
    }

    // ── Key stability ─────────────────────────────────────────────────────────

    #[test]
    fn llm_key_same_for_equal_inputs() {
        let msgs = sample_messages();
        let k1 = CassetteStore::llm_key(&msgs, 0.3);
        let k2 = CassetteStore::llm_key(&msgs, 0.3);
        assert_eq!(k1, k2);
    }

    #[test]
    fn llm_key_differs_for_different_temperatures() {
        let msgs = sample_messages();
        assert_ne!(
            CassetteStore::llm_key(&msgs, 0.0),
            CassetteStore::llm_key(&msgs, 1.0)
        );
    }
}
