use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::llm::vectorize::VectorizeService;
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::{en, Locale};
use crate::storage::models::EpisodicMemory;

#[derive(Deserialize)]
struct EpisodeResponse {
    summary: String,
    episode: String,
    subject: Option<String>,
    keywords: Option<Vec<String>>,
    participants: Option<Vec<String>>,
}

/// Extracts EpisodicMemory records from MemCell messages.
/// Supports both personal (first-person) and group (third-person) episodes.
pub struct EpisodeExtractor {
    llm: Arc<dyn LlmProvider>,
    vectorizer: Arc<dyn VectorizeService>,
    #[allow(dead_code)]
    locale: Locale,
    vector_model: String,
}

impl EpisodeExtractor {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        vectorizer: Arc<dyn VectorizeService>,
        locale: Locale,
        vector_model: String,
    ) -> Self {
        Self {
            llm,
            vectorizer,
            locale,
            vector_model,
        }
    }

    /// Extract a personal episode for a specific user.
    pub async fn extract_personal(
        &self,
        messages: &[Value],
        user_id: &str,
        user_name: &str,
        group_id: Option<&str>,
        group_name: Option<&str>,
        memcell_ids: Vec<String>,
    ) -> Result<EpisodicMemory> {
        debug!("EpisodeExtractor::extract_personal user={user_name}");

        let conversation = format_conversation(messages);
        let (sys, usr) = (en::EPISODE_GENERATION_SYSTEM, en::EPISODE_GENERATION_USER);
        let user_prompt = usr
            .replace("{user_name}", user_name)
            .replace("{conversation}", &conversation);

        let resp: EpisodeResponse = complete_json(
            &*self.llm,
            vec![LlmMessage::system(sys), LlmMessage::user(user_prompt)],
            0.3,
        )
        .await?;

        let search_content =
            EpisodicMemory::compute_search_content(resp.subject.as_deref(), &resp.episode);

        // Embed the episode text
        let vector = self.vectorizer.embed(&search_content).await.ok();

        Ok(EpisodicMemory {
            id: None,
            user_id: Some(user_id.to_string()),
            user_name: Some(user_name.to_string()),
            group_id: group_id.map(String::from),
            group_name: group_name.map(String::from),
            timestamp: Utc::now(),
            participants: resp.participants,
            summary: resp.summary,
            episode: resp.episode,
            subject: resp.subject,
            keywords: resp.keywords,
            memcell_ids: Some(memcell_ids),
            vector,
            vector_model: Some(self.vector_model.clone()),
            search_content: Some(search_content),
            is_deleted: false,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        })
    }

    /// Extract a group episode (third-person narrative for the group as a whole).
    pub async fn extract_group(
        &self,
        messages: &[Value],
        group_id: &str,
        group_name: &str,
        participants: &[String],
        memcell_ids: Vec<String>,
    ) -> Result<EpisodicMemory> {
        debug!("EpisodeExtractor::extract_group group={group_name}");

        let conversation = format_conversation(messages);
        let (sys, usr) = (
            en::GROUP_EPISODE_GENERATION_SYSTEM,
            en::GROUP_EPISODE_GENERATION_USER,
        );
        let user_prompt = usr
            .replace("{group_name}", group_name)
            .replace("{participants}", &participants.join(", "))
            .replace("{conversation}", &conversation);

        let resp: EpisodeResponse = complete_json(
            &*self.llm,
            vec![LlmMessage::system(sys), LlmMessage::user(user_prompt)],
            0.3,
        )
        .await?;

        let search_content =
            EpisodicMemory::compute_search_content(resp.subject.as_deref(), &resp.episode);
        let vector = self.vectorizer.embed(&search_content).await.ok();

        Ok(EpisodicMemory {
            id: None,
            user_id: None,
            user_name: None,
            group_id: Some(group_id.to_string()),
            group_name: Some(group_name.to_string()),
            timestamp: Utc::now(),
            participants: Some(participants.to_vec()),
            summary: resp.summary,
            episode: resp.episode,
            subject: resp.subject,
            keywords: resp.keywords,
            memcell_ids: Some(memcell_ids),
            vector,
            vector_model: Some(self.vector_model.clone()),
            search_content: Some(search_content),
            is_deleted: false,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        })
    }
}
