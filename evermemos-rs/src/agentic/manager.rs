use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use super::{
    prompts,
    retrieval_utils::{reciprocal_rank_fusion, tokenise},
};
use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::llm::rerank::RerankService;
use crate::llm::vectorize::VectorizeService;
use crate::storage::repository::{
    DateRange, EpisodicMemoryRepo, EventLogRepo, ForesightRepo, SearchResult, UserProfileRepo,
};

/// Retrieval method matching the Python `RetrieveMethod` enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RetrieveMethod {
    Keyword,
    Vector,
    Hybrid,
    Rrf,
    Agentic,
}

/// Memory type selector — which tables to search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    EpisodicMemory,
    ForesightRecord,
    EventLogRecord,
    Profile,
    /// Highest-priority structured user profile (mirrors Python `core_memory` collection).
    /// Backed by the same `user_profile` table; exposed with `memory_type = "core_memory"`.
    CoreMemory,
    All,
}

/// A single retrieved memory item returned to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub memory_type: String,
    pub content: String,
    pub score: f32,
    pub timestamp: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RetrieveRequest {
    pub query: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub method: RetrieveMethod,
    pub memory_types: Vec<MemoryType>,
    pub top_k: u32,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub radius: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveResponse {
    pub memories: Vec<MemoryItem>,
    pub total_count: usize,
    pub metadata: serde_json::Value,
}

/// Main agentic retrieval manager.
/// Implements KEYWORD / VECTOR / HYBRID / RRF / AGENTIC strategies.
pub struct AgenticManager {
    llm: Arc<dyn LlmProvider>,
    vectorizer: Arc<dyn VectorizeService>,
    reranker: Arc<dyn RerankService>,
    ep_repo: EpisodicMemoryRepo,
    fs_repo: ForesightRepo,
    el_repo: EventLogRepo,
    up_repo: UserProfileRepo,
}

impl AgenticManager {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        vectorizer: Arc<dyn VectorizeService>,
        reranker: Arc<dyn RerankService>,
        ep_repo: EpisodicMemoryRepo,
        fs_repo: ForesightRepo,
        el_repo: EventLogRepo,
        up_repo: UserProfileRepo,
    ) -> Self {
        Self {
            llm,
            vectorizer,
            reranker,
            ep_repo,
            fs_repo,
            el_repo,
            up_repo,
        }
    }

    /// Main dispatch — select strategy and execute.
    pub async fn retrieve(&self, req: RetrieveRequest) -> Result<RetrieveResponse> {
        debug!(
            "AgenticManager::retrieve method={:?} query={}",
            req.method, req.query
        );

        let items = match &req.method {
            RetrieveMethod::Keyword => self.keyword_search(&req).await?,
            RetrieveMethod::Vector => self.vector_search(&req).await?,
            RetrieveMethod::Hybrid => self.hybrid_search(&req, true).await?,
            RetrieveMethod::Rrf => self.rrf_search(&req).await?,
            RetrieveMethod::Agentic => self.agentic_search(&req).await?,
        };

        let total = items.len();
        Ok(RetrieveResponse {
            memories: items,
            total_count: total,
            metadata: serde_json::json!({ "method": format!("{:?}", req.method) }),
        })
    }

    // ── KEYWORD ───────────────────────────────────────────────────────────────

    async fn keyword_search(&self, req: &RetrieveRequest) -> Result<Vec<MemoryItem>> {
        let tokens = tokenise(&req.query);
        let date_range = date_range(req);
        let uid = req.user_id.as_deref();
        let gid = req.group_id.as_deref();
        let mut items = Vec::new();

        for mt in &req.memory_types {
            match mt {
                MemoryType::EpisodicMemory | MemoryType::All => {
                    let results = self
                        .ep_repo
                        .search_bm25(&tokens, uid, gid, req.top_k, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| ep_to_item(r)));
                }
                MemoryType::ForesightRecord => {
                    let results = self
                        .fs_repo
                        .search_bm25(&tokens, uid, gid, req.top_k, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| fs_to_item(r)));
                }
                MemoryType::EventLogRecord => {
                    let results = self
                        .el_repo
                        .search_bm25(&tokens, uid, gid, req.top_k, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| el_to_item(r)));
                }
                MemoryType::Profile => {
                    if let Some(uid_str) = uid {
                        if let Ok(Some(profile)) = self.up_repo.get_by_user_id(uid_str).await {
                            items.push(up_to_item(profile));
                        }
                    }
                }
                MemoryType::CoreMemory => {
                    if let Some(uid_str) = uid {
                        if let Ok(Some(profile)) = self.up_repo.get_by_user_id(uid_str).await {
                            items.push(cm_to_item(profile));
                        }
                    }
                }
            }
        }

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(req.top_k as usize);
        Ok(items)
    }

    // ── VECTOR ────────────────────────────────────────────────────────────────

    async fn vector_search(&self, req: &RetrieveRequest) -> Result<Vec<MemoryItem>> {
        let vec = self.vectorizer.embed(&req.query).await?;
        let date_range = date_range(req);
        let uid = req.user_id.as_deref();
        let gid = req.group_id.as_deref();
        let mut items = Vec::new();

        for mt in &req.memory_types {
            match mt {
                MemoryType::EpisodicMemory | MemoryType::All => {
                    let results = self
                        .ep_repo
                        .search_vector(&vec, uid, gid, req.top_k, req.radius, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| ep_to_item(r)));
                }
                MemoryType::ForesightRecord => {
                    let results = self
                        .fs_repo
                        .search_vector(&vec, uid, gid, req.top_k, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| fs_to_item(r)));
                }
                MemoryType::EventLogRecord => {
                    let results = self
                        .el_repo
                        .search_vector(&vec, uid, gid, req.top_k, &date_range)
                        .await?;
                    items.extend(results.into_iter().map(|r| el_to_item(r)));
                }
                MemoryType::Profile => {
                    if let Some(uid_str) = uid {
                        if let Ok(Some(profile)) = self.up_repo.get_by_user_id(uid_str).await {
                            items.push(up_to_item(profile));
                        }
                    }
                }
                MemoryType::CoreMemory => {
                    if let Some(uid_str) = uid {
                        if let Ok(Some(profile)) = self.up_repo.get_by_user_id(uid_str).await {
                            items.push(cm_to_item(profile));
                        }
                    }
                }
            }
        }

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(req.top_k as usize);
        Ok(items)
    }

    // ── HYBRID (keyword + vector + rerank) ────────────────────────────────────

    async fn hybrid_search(
        &self,
        req: &RetrieveRequest,
        do_rerank: bool,
    ) -> Result<Vec<MemoryItem>> {
        let (kw_items, vec_items) =
            tokio::join!(self.keyword_search(req), self.vector_search(req),);
        let mut merged = merge_dedup(kw_items?, vec_items?);

        if do_rerank && self.reranker.is_enabled() && !merged.is_empty() {
            let docs: Vec<String> = merged.iter().map(|i| i.content.clone()).collect();
            match self.reranker.rerank(&req.query, &docs).await {
                Ok(scores) => {
                    for (item, score) in merged.iter_mut().zip(scores.iter()) {
                        item.score = *score;
                    }
                    merged.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Err(e) => {
                    tracing::warn!("Rerank failed, using pre-rerank order: {e}");
                }
            }
        }

        merged.truncate(req.top_k as usize);
        Ok(merged)
    }

    // ── RRF (Reciprocal Rank Fusion, no reranker) ─────────────────────────────

    async fn rrf_search(&self, req: &RetrieveRequest) -> Result<Vec<MemoryItem>> {
        let (kw_items, vec_items) =
            tokio::join!(self.keyword_search(req), self.vector_search(req),);
        let kw = kw_items?
            .into_iter()
            .map(|i| (i.id.clone(), i))
            .collect::<Vec<_>>();
        let vc = vec_items?
            .into_iter()
            .map(|i| (i.id.clone(), i))
            .collect::<Vec<_>>();

        let rrf_scores = reciprocal_rank_fusion(vec![kw.clone(), vc.clone()], 60.0);

        // Build a merged map from id → item
        let mut item_map: HashMap<String, MemoryItem> = HashMap::new();
        for (id, item) in kw.into_iter().chain(vc.into_iter()) {
            item_map.entry(id).or_insert(item);
        }

        let mut result: Vec<MemoryItem> = rrf_scores
            .into_iter()
            .filter_map(|(id, score)| {
                item_map.remove(&id).map(|mut item| {
                    item.score = score;
                    item
                })
            })
            .collect();

        result.truncate(req.top_k as usize);
        Ok(result)
    }

    // ── AGENTIC (multi-round LLM-guided) ──────────────────────────────────────

    async fn agentic_search(&self, req: &RetrieveRequest) -> Result<Vec<MemoryItem>> {
        // Round 1: hybrid search with larger top_k
        let round1_req = RetrieveRequest {
            top_k: 20,
            ..req.clone()
        };
        let mut round1 = self.hybrid_search(&round1_req, true).await?;
        round1.truncate(5); // Keep top-5 for sufficiency check

        // Sufficiency check
        let memory_text = round1
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. {}", i + 1, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        #[derive(Deserialize)]
        struct SufficiencyResp {
            is_sufficient: bool,
            missing_information: Option<Vec<String>>,
        }

        let sufficiency: SufficiencyResp = complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(prompts::SUFFICIENCY_CHECK_SYSTEM),
                LlmMessage::user(
                    prompts::SUFFICIENCY_CHECK_USER
                        .replace("{query}", &req.query)
                        .replace("{memories}", &memory_text),
                ),
            ],
            0.0,
        )
        .await
        .unwrap_or(SufficiencyResp {
            is_sufficient: true,
            missing_information: None,
        });

        if sufficiency.is_sufficient {
            round1.truncate(req.top_k as usize);
            return Ok(round1);
        }

        // Round 2: generate refined queries
        let missing = sufficiency
            .missing_information
            .unwrap_or_default()
            .join(", ");

        let refined_queries: Vec<String> = complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(prompts::MULTI_QUERY_GENERATION_SYSTEM),
                LlmMessage::user(
                    prompts::MULTI_QUERY_GENERATION_USER
                        .replace("{query}", &req.query)
                        .replace("{missing_info}", &missing),
                ),
            ],
            0.0,
        )
        .await
        .unwrap_or_default();

        // Concurrently run hybrid search for each refined query
        // Build sub-requests first, then run them sequentially (avoids borrow lifetime issue)
        let sub_reqs: Vec<RetrieveRequest> = refined_queries
            .iter()
            .take(3)
            .map(|rq| RetrieveRequest {
                query: rq.clone(),
                top_k: 50,
                ..req.clone()
            })
            .collect();

        let mut all_items: Vec<MemoryItem> = round1;
        for sr in &sub_reqs {
            if let Ok(items) = self.hybrid_search(sr, false).await {
                all_items.extend(items);
            }
        }

        let mut merged = deduplicate(all_items);

        // Final rerank
        if self.reranker.is_enabled() && !merged.is_empty() {
            let docs: Vec<String> = merged.iter().map(|i| i.content.clone()).collect();
            if let Ok(scores) = self.reranker.rerank(&req.query, &docs).await {
                for (item, score) in merged.iter_mut().zip(scores.iter()) {
                    item.score = *score;
                }
            }
        }

        merged.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        merged.truncate(req.top_k as usize);
        Ok(merged)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn date_range(req: &RetrieveRequest) -> DateRange {
    DateRange {
        start: req.start_time,
        end: req.end_time,
    }
}

fn ep_to_item(r: SearchResult<crate::storage::models::EpisodicMemory>) -> MemoryItem {
    let id = r.item.id.as_ref().map(|t| t.to_raw()).unwrap_or_default();
    MemoryItem {
        id,
        memory_type: "episodic_memory".into(),
        content: r.item.episode.clone(),
        score: r.score,
        timestamp: Some(r.item.timestamp),
        metadata: serde_json::json!({
            "summary": r.item.summary,
            "subject": r.item.subject,
            "keywords": r.item.keywords,
        }),
    }
}

fn fs_to_item(r: SearchResult<crate::storage::models::ForesightRecord>) -> MemoryItem {
    let id = r.item.id.as_ref().map(|t| t.to_raw()).unwrap_or_default();
    MemoryItem {
        id,
        memory_type: "foresight_record".into(),
        content: r.item.foresight.clone(),
        score: r.score,
        timestamp: Some(r.item.timestamp),
        metadata: serde_json::json!({ "evidence": r.item.evidence }),
    }
}

fn el_to_item(r: SearchResult<crate::storage::models::EventLogRecord>) -> MemoryItem {
    let id = r.item.id.as_ref().map(|t| t.to_raw()).unwrap_or_default();
    MemoryItem {
        id,
        memory_type: "event_log_record".into(),
        content: r.item.atomic_fact.clone(),
        score: r.score,
        timestamp: Some(r.item.timestamp),
        metadata: serde_json::Value::Null,
    }
}

fn up_to_item(profile: crate::storage::models::UserProfile) -> MemoryItem {
    let id = profile.id.as_ref().map(|t| t.to_raw()).unwrap_or_default();
    let content = profile.life_summary.clone().unwrap_or_else(|| {
        profile
            .profile_data
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default()
    });
    MemoryItem {
        id,
        memory_type: "profile".into(),
        content,
        score: 1.0,
        timestamp: profile.created_at,
        metadata: serde_json::json!({
            "user_id": profile.user_id,
            "profile_data": profile.profile_data,
        }),
    }
}

/// Same backing data as `up_to_item` but exposed as `memory_type = "core_memory"`.
/// Mirrors Python's `CoreMemory` collection — highest-priority user profile memory.
fn cm_to_item(profile: crate::storage::models::UserProfile) -> MemoryItem {
    let id = profile.id.as_ref().map(|t| t.to_raw()).unwrap_or_default();
    let content = profile.life_summary.clone().unwrap_or_else(|| {
        profile
            .profile_data
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default()
    });
    MemoryItem {
        id,
        memory_type: "core_memory".into(),
        content,
        score: 1.0,
        timestamp: profile.created_at,
        metadata: serde_json::json!({
            "user_id": profile.user_id,
            "profile_data": profile.profile_data,
            "is_latest": true,
        }),
    }
}

fn merge_dedup(a: Vec<MemoryItem>, b: Vec<MemoryItem>) -> Vec<MemoryItem> {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();
    for item in a.into_iter().chain(b) {
        if seen.insert(item.id.clone()) {
            merged.push(item);
        }
    }
    merged
}

fn deduplicate(items: Vec<MemoryItem>) -> Vec<MemoryItem> {
    merge_dedup(items, vec![])
}
