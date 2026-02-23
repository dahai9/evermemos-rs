use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config::RerankConfig;

/// Reranking service trait — scores (query, document) pairs.
#[async_trait]
pub trait RerankService: Send + Sync {
    /// Rerank documents against a query. Returns scores in input order.
    async fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<f32>>;
    fn is_enabled(&self) -> bool;
}

/// No-op reranker used when reranking is not configured.
#[derive(Clone)]
pub struct NoopReranker;

#[async_trait]
impl RerankService for NoopReranker {
    async fn rerank(&self, _query: &str, documents: &[String]) -> Result<Vec<f32>> {
        // Return uniform scores — caller can sort by original retrieval score
        Ok(vec![1.0; documents.len()])
    }
    fn is_enabled(&self) -> bool { false }
}

/// OpenAI-compatible cross-encoder reranker for HYBRID/AGENTIC retrieval.
/// Matches the Python `VllmRerankService` / `DeepInfraRerankService` interface.
#[derive(Clone)]
pub struct OpenAiReranker {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiReranker {
    pub fn new(cfg: &RerankConfig) -> Self {
        Self {
            http: Client::new(),
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
            model: cfg.model.clone(),
        }
    }

    /// Build the reranker — returns NoopReranker if not configured.
    pub fn build(cfg: &RerankConfig) -> Box<dyn RerankService> {
        if cfg.enabled && !cfg.base_url.is_empty() && !cfg.model.is_empty() {
            Box::new(OpenAiReranker::new(cfg))
        } else {
            Box::new(NoopReranker)
        }
    }
}

#[derive(Serialize)]
struct RerankRequest<'a> {
    model: &'a str,
    query: &'a str,
    documents: &'a [String],
}

#[derive(Deserialize)]
struct RerankResponse {
    results: Vec<RerankResult>,
}

#[derive(Deserialize)]
struct RerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl RerankService for OpenAiReranker {
    async fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<f32>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        debug!("Reranking {} documents", documents.len());

        let url = format!("{}/rerank", self.base_url);
        let body = RerankRequest {
            model: &self.model,
            query,
            documents,
        };

        let resp: RerankResponse = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // Restore original order
        let mut scores = vec![0.0f32; documents.len()];
        for r in resp.results {
            if r.index < scores.len() {
                scores[r.index] = r.relevance_score;
            }
        }
        Ok(scores)
    }

    fn is_enabled(&self) -> bool { true }
}
