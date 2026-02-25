use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::en;
use crate::storage::models::GroupProfile;

/// LLM response shape for group profile extraction.
#[derive(Debug, Deserialize, Default)]
struct GroupProfileResponse {
    #[serde(default)]
    topics: Vec<Value>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    subject: String,
}

/// Extracts an aggregated GroupProfile from a group chat MemCell.
///
/// One LLM call: topics + summary + subject combined (mirrors Python
/// `CONTENT_ANALYSIS_PROMPT`-based flow, simplified — no incremental
/// topic IDs or evidence tracking in this version).
pub struct GroupProfileExtractor {
    llm: Arc<dyn LlmProvider>,
}

impl GroupProfileExtractor {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Run extraction and return a `GroupProfile` ready to be upserted.
    pub async fn extract(
        &self,
        messages: &[Value],
        group_id: &str,
        group_name: &str,
        existing: Option<&GroupProfile>,
    ) -> Result<GroupProfile> {
        debug!("GroupProfileExtractor::extract group={group_name}");

        let conversation = format_conversation(messages);

        let existing_json = existing
            .and_then(|gp| serde_json::to_string_pretty(gp).ok())
            .unwrap_or_else(|| "{}".to_string());

        let user_content = en::GROUP_PROFILE_USER
            .replace("{group_id}", group_id)
            .replace("{group_name}", group_name)
            .replace("{existing_profile}", &existing_json)
            .replace("{conversation}", &conversation);

        let messages_vec = vec![
            LlmMessage::system(en::GROUP_PROFILE_SYSTEM),
            LlmMessage::user(user_content),
        ];

        let resp: GroupProfileResponse =
            complete_json(&*self.llm, messages_vec, 0.3).await?;

        let topics_value = if resp.topics.is_empty() {
            None
        } else {
            Some(serde_json::Value::Array(resp.topics))
        };

        let subject = if resp.subject.is_empty() || resp.subject == "not_found" {
            None
        } else {
            Some(resp.subject)
        };

        let now = Utc::now();
        Ok(GroupProfile {
            id: existing.and_then(|gp| gp.id.clone()),
            group_id: group_id.to_string(),
            group_name: Some(group_name.to_string()),
            topics: topics_value,
            summary: if resp.summary.is_empty() { None } else { Some(resp.summary) },
            subject,
            is_deleted: false,
            created_at: existing
                .and_then(|gp| gp.created_at)
                .or(Some(now)),
            updated_at: Some(now),
        })
    }
}
