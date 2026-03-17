use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info};

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::en;
use crate::storage::models::UserProfile;

#[derive(Debug, Deserialize, Serialize, Default)]
struct Part1Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    personality_traits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interests: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    communication_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preferences: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Part2Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    occupation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    education: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    family_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
        info!("ProfileExtractor::extract user={user_name}");

        let conversation = format_conversation(messages);

        // Extract existing profile data for LLM context
        let existing_data = existing_profile
            .and_then(|p| p.profile_data.as_ref())
            .unwrap_or(&Value::Null);

        // Phase 1 — personality & preferences (parallel with Phase 2)
        let p1_fut = self.extract_part1(user_name, &conversation, existing_data);
        let p2_fut = self.extract_part2(user_name, &conversation, existing_data);
        let (p1, p2) = tokio::join!(p1_fut, p2_fut);

        // Merge part1 + part2 into a single profile_data object
        let mut profile_data = serde_json::Map::new();
        
        // Start with existing data
        if let Some(existing_obj) = existing_data.as_object() {
            profile_data.extend(existing_obj.clone());
        }

        match p1 {
            Ok(part1) => {
                info!("Profile Part1 extracted successfully: {:?}", part1);
                if let Ok(v) = serde_json::to_value(&part1) {
                    if let Some(obj) = v.as_object() {
                        profile_data.extend(obj.clone());
                    }
                }
            }
            Err(e) => tracing::warn!("Profile Part1 extraction failed: {e}"),
        }
        match p2 {
            Ok(part2) => {
                info!("Profile Part2 extracted successfully: {:?}", part2);
                if let Ok(v) = serde_json::to_value(&part2) {
                    if let Some(obj) = v.as_object() {
                        profile_data.extend(obj.clone());
                    }
                }
            }
            Err(e) => tracing::warn!("Profile Part2 extraction failed: {e}"),
        }

        info!("Final merged profile_data before saving: {:?}", profile_data);

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

    async fn extract_part1(&self, user_name: &str, conversation: &str, existing_data: &Value) -> Result<Part1Data> {
        let user_prompt = en::PROFILE_PART1_USER
            .replace("{user_name}", user_name)
            .replace("{existing_data}", &serde_json::to_string(existing_data).unwrap_or("null".to_string()))
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

    async fn extract_part2(&self, user_name: &str, conversation: &str, existing_data: &Value) -> Result<Part2Data> {
        let user_prompt = en::PROFILE_PART2_USER
            .replace("{user_name}", user_name)
            .replace("{existing_data}", &serde_json::to_string(existing_data).unwrap_or("null".to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    struct MockLlm {
        part1_response: String,
        part2_response: String,
        life_summary_response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(
            &self,
            messages: Vec<LlmMessage>,
            _temperature: f32,
            _max_tokens: Option<u32>,
        ) -> Result<String> {
            let sys = messages
                .iter()
                .find(|m| m.role == crate::llm::provider::LlmRole::System)
                .unwrap();
            
            if sys.content.contains("personality, preferences") {
                Ok(self.part1_response.clone())
            } else if sys.content.contains("factual and demographic") {
                Ok(self.part2_response.clone())
            } else {
                Ok(self.life_summary_response.clone())
            }
        }
    }

    #[tokio::test]
    async fn test_initial_extraction() {
        let mock = MockLlm {
            part1_response: r#"{"interests":["reading"]}"#.to_string(),
            part2_response: r#"{"location":"NY"}"#.to_string(),
            life_summary_response: r#"{"life_summary":"Lives in NY, loves reading."}"#.to_string(),
        };

        let extractor = ProfileExtractor::new(Arc::new(mock));
        let profile = extractor
            .extract(&[], "user_1", "Alice", None)
            .await
            .unwrap();

        let pd = profile.profile_data.unwrap();
        assert_eq!(pd.get("interests").unwrap()[0], "reading");
        assert_eq!(pd.get("location").unwrap(), "NY");
        assert_eq!(profile.life_summary.unwrap(), "Lives in NY, loves reading.");
    }

    #[tokio::test]
    async fn test_metabolism_update() {
        // Prepare an existing profile
        let existing = UserProfile {
            id: None,
            user_id: "user_1".to_string(),
            profile_data: Some(json!({
                "interests": ["reading"],
                "location": "NY",
                "old_hobby": "stamps" // ensure old keys not overwritten are preserved
            })),
            life_summary: Some("Lives in NY, loves reading.".to_string()),
            custom_profile_data: None,
            is_deleted: false,
            created_at: None,
            updated_at: None,
        };

        // Mock returns an updated hobby and new location (e.g. they moved and started hiking)
        let mock = MockLlm {
            part1_response: r#"{"interests":["hiking"]}"#.to_string(),
            part2_response: r#"{"location":"SF"}"#.to_string(),
            life_summary_response: r#"{"life_summary":"Moved to SF, loves hiking."}"#.to_string(),
        };

        let extractor = ProfileExtractor::new(Arc::new(mock));
        let profile = extractor
            .extract(&[], "user_1", "Alice", Some(&existing))
            .await
            .unwrap();

        let pd = profile.profile_data.unwrap();
        // The new extraction returns "hiking", which overrides "reading" because of how `serde_json::Map::extend` works.
        assert_eq!(pd.get("interests").unwrap()[0], "hiking");
        assert_eq!(pd.get("location").unwrap(), "SF");
        assert_eq!(pd.get("old_hobby").unwrap(), "stamps"); // Preserved
        assert_eq!(profile.life_summary.unwrap(), "Moved to SF, loves hiking.");
    }
}
