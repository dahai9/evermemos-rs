use anyhow::{Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::CreateEmbeddingRequestArgs,
    Client,
};
use async_trait::async_trait;
use tracing::debug;

use crate::config::VectorizeConfig;
use crate::core::cache::Caches;

/// Vectorisation service trait — produces 1024-dim float embeddings.
#[async_trait]
pub trait VectorizeService: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// OpenAI-compatible embedding service (works with any OpenAI `/embeddings` endpoint).
#[derive(Clone)]
pub struct OpenAiVectorizer {
    client: Client<OpenAIConfig>,
    model: String,
    dimensions: usize,
    caches: Caches,
}

impl OpenAiVectorizer {
    pub fn new(cfg: &VectorizeConfig, caches: Caches) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(&cfg.api_key)
            .with_api_base(&cfg.base_url);

        Self {
            client: Client::with_config(config),
            model: cfg.model.clone(),
            dimensions: cfg.dimensions,
            caches,
        }
    }
}

#[async_trait]
impl VectorizeService for OpenAiVectorizer {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Cache lookup
        if let Some(cached) = self.caches.embeddings.get(&text.to_string()).await {
            debug!("Embedding cache hit");
            return Ok(cached);
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(text)
            .dimensions(self.dimensions as u32)
            .build()
            .context("Failed to build embedding request")?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .context("Embedding API call failed")?;

        let vec = response
            .data
            .into_iter()
            .next()
            .map(|e| e.embedding)
            .unwrap_or_default();

        // Truncate / pad to target dimension
        let mut result = vec;
        result.truncate(self.dimensions);

        self.caches.embeddings.insert(text.to_string(), result.clone()).await;
        Ok(result)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(texts.to_vec())
            .dimensions(self.dimensions as u32)
            .build()
            .context("Failed to build batch embedding request")?;

        let mut response = self
            .client
            .embeddings()
            .create(request)
            .await
            .context("Batch embedding API call failed")?;

        // Sort by index (API guarantees order but let's be safe)
        response.data.sort_by_key(|e| e.index);

        Ok(response
            .data
            .into_iter()
            .map(|e| {
                let mut v = e.embedding;
                v.truncate(self.dimensions);
                v
            })
            .collect())
    }
}
