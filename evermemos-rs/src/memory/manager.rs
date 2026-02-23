use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::llm::provider::LlmProvider;
use crate::llm::vectorize::VectorizeService;
use crate::memory::{
    episode_extractor::EpisodeExtractor,
    event_log_extractor::EventLogExtractor,
    foresight_extractor::ForesightExtractor,
    profile_extractor::ProfileExtractor,
    prompts::Locale,
};
use crate::storage::{
    models::MemCell,
    repository::{
        EpisodicMemoryRepo, EventLogRepo, ForesightRepo, UserProfileRepo,
    },
};

/// Scene type — determines which extractors are run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneType {
    /// 1-on-1 assistant chat — runs all extractors including foresight + event log
    Assistant,
    /// Group chat — skips foresight + event log; uses group episode extractor
    Group,
}

pub struct MemoryManager {
    episode_ex: EpisodeExtractor,
    foresight_ex: ForesightExtractor,
    event_log_ex: EventLogExtractor,
    profile_ex: ProfileExtractor,
    ep_repo: EpisodicMemoryRepo,
    fs_repo: ForesightRepo,
    el_repo: EventLogRepo,
    up_repo: UserProfileRepo,
}

impl MemoryManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        vectorizer: Arc<dyn VectorizeService>,
        ep_repo: EpisodicMemoryRepo,
        fs_repo: ForesightRepo,
        el_repo: EventLogRepo,
        up_repo: UserProfileRepo,
        locale: Locale,
        vector_model: String,
    ) -> Self {
        Self {
            episode_ex: EpisodeExtractor::new(
                Arc::clone(&llm),
                Arc::clone(&vectorizer),
                locale,
                vector_model.clone(),
            ),
            foresight_ex: ForesightExtractor::new(
                Arc::clone(&llm),
                Arc::clone(&vectorizer),
                vector_model.clone(),
            ),
            event_log_ex: EventLogExtractor::new(
                Arc::clone(&llm),
                Arc::clone(&vectorizer),
                vector_model,
            ),
            profile_ex: ProfileExtractor::new(Arc::clone(&llm)),
            ep_repo,
            fs_repo,
            el_repo,
            up_repo,
        }
    }

    /// Process a complete MemCell: run all applicable extractors concurrently
    /// and persist results to SurrealDB.
    pub async fn process_memcell(
        &self,
        cell: &MemCell,
        user_id: Option<&str>,
        user_name: &str,
        group_id: Option<&str>,
        group_name: Option<&str>,
        scene: SceneType,
    ) {
        let memcell_id = cell
            .id
            .as_ref()
            .map(|t| t.id.to_raw())
            .unwrap_or_default();
        let messages = cell.original_data.as_deref().unwrap_or(&[]);

        info!("MemoryManager: processing memcell {memcell_id}, scene={scene:?}");

        // ── Run extractors concurrently ────────────────────────────────────────
        let ep_fut = async {
            if let Some(uid) = user_id {
                match self
                    .episode_ex
                    .extract_personal(messages, uid, user_name, group_id, group_name, vec![memcell_id.clone()])
                    .await
                {
                    Ok(ep) => {
                        if let Err(e) = self.ep_repo.insert(ep).await {
                            warn!("Failed to save personal episode: {e}");
                        } else {
                            debug!("Saved personal episode");
                        }
                    }
                    Err(e) => warn!("Episode extraction failed: {e}"),
                }
            }

            if let (Some(gid), Some(gname)) = (group_id, group_name) {
                let participants: Vec<String> = cell
                    .participants
                    .as_deref()
                    .unwrap_or(&[])
                    .to_vec();
                match self
                    .episode_ex
                    .extract_group(messages, gid, gname, &participants, vec![memcell_id.clone()])
                    .await
                {
                    Ok(ep) => {
                        if let Err(e) = self.ep_repo.insert(ep).await {
                            warn!("Failed to save group episode: {e}");
                        }
                    }
                    Err(e) => warn!("Group episode extraction failed: {e}"),
                }
            }
        };

        let fs_fut = async {
            if scene == SceneType::Assistant {
                if let Some(uid) = user_id {
                    match self.foresight_ex.extract(messages, uid, user_name, group_id).await {
                        Ok(records) => {
                            for r in records {
                                if let Err(e) = self.fs_repo.insert(r).await {
                                    warn!("Failed to save foresight: {e}");
                                }
                            }
                        }
                        Err(e) => warn!("Foresight extraction failed: {e}"),
                    }
                }
            }
        };

        let el_fut = async {
            if scene == SceneType::Assistant {
                if let Some(uid) = user_id {
                    match self.event_log_ex.extract(messages, uid, user_name, group_id).await {
                        Ok(records) => {
                            for r in records {
                                if let Err(e) = self.el_repo.insert(r).await {
                                    warn!("Failed to save event log: {e}");
                                }
                            }
                        }
                        Err(e) => warn!("Event log extraction failed: {e}"),
                    }
                }
            }
        };

        let profile_fut = async {
            if let Some(uid) = user_id {
                let existing = self.up_repo.get_by_user_id(uid).await.ok().flatten();
                match self
                    .profile_ex
                    .extract(messages, uid, user_name, existing.as_ref())
                    .await
                {
                    Ok(profile) => {
                        if let Err(e) = self.up_repo.upsert(profile).await {
                            warn!("Failed to upsert profile: {e}");
                        }
                    }
                    Err(e) => warn!("Profile extraction failed: {e}"),
                }
            }
        };

        // Run all concurrently (mirrors Python asyncio.gather)
        tokio::join!(ep_fut, fs_fut, el_fut, profile_fut);

        info!("MemoryManager: finished processing memcell {memcell_id}");
    }
}
