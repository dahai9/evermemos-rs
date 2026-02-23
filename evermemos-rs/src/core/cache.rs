use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// In-process async cache using moka.
/// Replaces Redis caching (rate limits, embedding results, etc.)
///
/// Parameterised over key `K` and value `V`.
#[derive(Clone)]
pub struct AppCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: Arc<Cache<K, V>>,
}

impl<K, V> AppCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new TTL cache with given capacity and time-to-live.
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();
        Self {
            inner: Arc::new(cache),
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value).await;
    }

    pub async fn invalidate(&self, key: &K) {
        self.inner.invalidate(key).await;
    }
}

/// Shared caches used across the application.
#[derive(Clone)]
pub struct Caches {
    /// Recent embeddings — key: text, value: Vec<f32>
    pub embeddings: AppCache<String, Vec<f32>>,
    /// Rate-limit counters — key: "{tenant}:{endpoint}", value: request count
    pub rate_limits: AppCache<String, u64>,
}

impl Caches {
    pub fn new() -> Self {
        Self {
            embeddings: AppCache::new(1_024, Duration::from_secs(3600)),
            rate_limits: AppCache::new(10_000, Duration::from_secs(60)),
        }
    }
}

impl Default for Caches {
    fn default() -> Self {
        Self::new()
    }
}
