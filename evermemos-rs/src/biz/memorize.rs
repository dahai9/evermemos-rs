use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

use crate::memory::{
    manager::{MemoryManager, SceneType},
    memcell_extractor::{BoundaryStatus, MemCellExtractor},
};
use crate::storage::repository::MemCellRepo;

/// A single raw message to be memorised.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMessage {
    pub message_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub create_time: DateTime<Utc>,
    pub role: Option<String>, // "user" | "assistant"
}

/// Request to memorise a new message.
#[derive(Debug, Clone)]
pub struct MemorizeRequest {
    pub message: RawMessage,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    /// Previous messages already buffered (history context).
    pub history: Vec<Value>,
}

/// Result of the memorize operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorizeResult {
    pub status: String,
    pub message: String,
    /// How many new memory records were created.
    pub saved_count: usize,
}

/// Top-level memorise service — 4-stage pipeline.
///
/// Stage 1: Detect MemCell boundary (LLM call).
/// Stage 2: If boundary triggered, persist MemCell to SurrealDB.
/// Stage 3: Concurrently extract all memory types (episode / foresight / event log / profile).
/// Stage 4: Trigger clustering and profile lifecycle update.
pub struct MemorizeService {
    boundary_detector: MemCellExtractor,
    memory_manager: Arc<MemoryManager>,
    memcell_repo: MemCellRepo,
}

impl MemorizeService {
    pub fn new(
        boundary_detector: MemCellExtractor,
        memory_manager: Arc<MemoryManager>,
        memcell_repo: MemCellRepo,
    ) -> Self {
        Self {
            boundary_detector,
            memory_manager,
            memcell_repo,
        }
    }

    /// Process one incoming message through the full memorize pipeline.
    pub async fn memorize(&self, req: MemorizeRequest) -> Result<MemorizeResult> {
        let msg_json = serde_json::json!({
            "message_id": req.message.message_id,
            "sender": req.message.sender,
            "sender_name": req.message.sender_name,
            "content": req.message.content,
            "role": req.message.role,
            "create_time": req.message.create_time.to_rfc3339(),
        });
        let new_messages = vec![msg_json];

        let user_id_ref = req.user_id.as_deref();
        let group_id_ref = req.group_id.as_deref();

        // Stage 1 — boundary detection
        let boundary = self
            .boundary_detector
            .detect_boundary(&req.history, &new_messages, user_id_ref, group_id_ref)
            .await?;

        match boundary {
            BoundaryStatus::Accumulating => {
                info!("Memorize: still accumulating for user={:?}", req.user_id);
                return Ok(MemorizeResult {
                    status: "accumulating".into(),
                    message: "Message buffered, boundary not yet reached".into(),
                    saved_count: 0,
                });
            }
            BoundaryStatus::Extracted(mut cell) => {
                // Stage 2 — persist MemCell
                let saved = match self.memcell_repo.insert(cell.clone()).await {
                    Ok(saved) => saved,
                    Err(e) => {
                        warn!("Failed to save MemCell: {e}");
                        return Err(e);
                    }
                };
                cell = saved;

                info!("Memorize: MemCell saved, running parallel extraction");

                // Determine scene type
                let scene = if req.group_id.is_some() {
                    SceneType::Group
                } else {
                    SceneType::Assistant
                };

                let user_name = req
                    .user_name
                    .as_deref()
                    .or(req.user_id.as_deref())
                    .unwrap_or("unknown");

                let group_name = req.group_name.as_deref();

                // Stage 3 — concurrent extraction (fire-and-forget background task)
                let mm = Arc::clone(&self.memory_manager);
                let cell_clone = cell.clone();
                let user_id_owned = req.user_id.clone();
                let user_name_owned = user_name.to_string();
                let group_id_owned = req.group_id.clone();
                let group_name_owned = group_name.map(String::from);

                tokio::spawn(async move {
                    mm.process_memcell(
                        &cell_clone,
                        user_id_owned.as_deref(),
                        &user_name_owned,
                        group_id_owned.as_deref(),
                        group_name_owned.as_deref(),
                        scene,
                    )
                    .await;
                });

                Ok(MemorizeResult {
                    status: "extracted".into(),
                    message: "MemCell saved, memory extraction started".into(),
                    saved_count: 1,
                })
            }
        }
    }
}
