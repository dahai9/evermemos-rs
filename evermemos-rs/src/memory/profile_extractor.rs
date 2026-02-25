use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::en;
use crate::storage::models::UserProfile;

#[derive(Debug, Deserialize, Serialize, Default)]
struct Part1Data {
    personality_traits: Option<Vec<String>>,
    interests: Option<Vec<String>>,
    communication_style: Option<String>,
    values: Option<Vec<String>>,
    preferences: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Part2Data {
    occupation: Option<String>,
    location: Option<String>,
    education: Option<String>,
    skills: Option<Vec<String>>,
    family_status: Option<String>,
    goals: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct LifeSummaryResponse {
    life_summary: String,
}

/// Multi-stage user profile extractor.
/// Mirrors Python `ProfileMemoryExtractor` (Part1 + Part2 + Life phases).
pub struct ProfileExtractor {
    llm: Arc<dyn LlmProvider>,
}

impl ProfileExtractor {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Run all three extraction phases and merge with existing profile.
    pub async fn extract(
        &self,
        messages: &[Value],
        user_id: &str,
        user_name: &str,
        existing_profile: Option<&UserProfile>,
    ) -> Result<UserProfile> {
        debug!("ProfileExtractor::extract user={user_name}");

        let conversation = format_conversation(messages);

        // Phase 1 — personality & preferences (parallel with Phase 2)
        let p1_fut = self.extract_part1(user_name, &conversation);
        let p2_fut = self.extract_part2(user_name, &conversation);
        let (p1, p2) = tokio::join!(p1_fut, p2_fut);

        // Merge part1 + part2 into a single profile_data object
        let mut profile_data = serde_json::Map::new();
        if let Ok(part1) = p1 {
            if let Ok(v) = serde_json::to_value(&part1) {
                if let Some(obj) = v.as_object() {
                    profile_data.extend(obj.clone());
                }
            }
        }
        if let Ok(part2) = p2 {
            if let Ok(v) = serde_json::to_value(&part2) {
                if let Some(obj) = v.as_object() {
                    profile_data.extend(obj.clone());
                }
            }
        }

        // Phase 3 — life summary update
        let existing_summary = existing_profile
            .and_then(|p| p.life_summary.as_deref())
            .unwrap_or("No existing summary.");

        let life_summary = self
            .update_life_summary(user_name, &conversation, existing_summary, existing_profile.is_none())
            .await
            .unwrap_or_else(|_| existing_summary.to_string());

        Ok(UserProfile {
            id: existing_profile.and_then(|p| p.id.clone()),
            user_id: user_id.to_string(),
            profile_data: Some(Value::Object(profile_data)),
            life_summary: Some(life_summary),
            custom_profile_data: existing_profile.and_then(|p| p.custom_profile_data.clone()),
            is_deleted: false,
            created_at: existing_profile
                .and_then(|p| p.created_at)
                .or_else(|| Some(Utc::now())),
            updated_at: Some(Utc::now()),
        })
    }

    async fn extract_part1(&self, user_name: &str, conversation: &str) -> Result<Part1Data> {
        let user_prompt = en::PROFILE_PART1_USER
            .replace("{user_name}", user_name)
            .replace("{conversation}", conversation);
        complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(en::PROFILE_PART1_SYSTEM),
                LlmMessage::user(user_prompt),
            ],
            0.3,
        )
        .await
    }

    async fn extract_part2(&self, user_name: &str, conversation: &str) -> Result<Part2Data> {
        let user_prompt = en::PROFILE_PART2_USER
            .replace("{user_name}", user_name)
            .replace("{conversation}", conversation);
        complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(en::PROFILE_PART2_SYSTEM),
                LlmMessage::user(user_prompt),
            ],
            0.3,
        )
        .await
    }

    async fn update_life_summary(
        &self,
        user_name: &str,
        conversation: &str,
        existing_summary: &str,
        is_initial: bool,
    ) -> Result<String> {
        let (sys, usr) = if is_initial {
            (en::PROFILE_LIFE_INITIAL_SYSTEM, en::PROFILE_LIFE_INITIAL_USER)
        } else {
            (en::PROFILE_LIFE_UPDATE_SYSTEM, en::PROFILE_LIFE_UPDATE_USER)
        };

        let user_prompt = usr
            .replace("{user_name}", user_name)
            .replace("{existing_summary}", existing_summary)
            .replace("{conversation}", conversation);

        let resp: LifeSummaryResponse = complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(sys),
                LlmMessage::user(user_prompt),
            ],
            0.3,
        )
        .await?;

        Ok(resp.life_summary)
    }
}
