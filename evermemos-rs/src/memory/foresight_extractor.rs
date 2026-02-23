use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::llm::vectorize::VectorizeService;
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::en;
use crate::storage::models::ForesightRecord;

#[derive(Deserialize)]
struct ForesightItem {
    foresight: String,
    evidence: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    duration_days: Option<i32>,
}

pub struct ForesightExtractor {
    llm: Arc<dyn LlmProvider>,
    vectorizer: Arc<dyn VectorizeService>,
    #[allow(dead_code)]
    vector_model: String,
}

impl ForesightExtractor {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        vectorizer: Arc<dyn VectorizeService>,
        vector_model: String,
    ) -> Self {
        Self {
            llm,
            vectorizer,
            vector_model,
        }
    }

    pub async fn extract(
        &self,
        messages: &[Value],
        user_id: &str,
        user_name: &str,
        group_id: Option<&str>,
    ) -> Result<Vec<ForesightRecord>> {
        debug!("ForesightExtractor::extract user={user_name}");

        let conversation = format_conversation(messages);
        let current_time = Utc::now().to_rfc3339();
        let user_prompt = en::FORESIGHT_GENERATION_USER
            .replace("{current_time}", &current_time)
            .replace("{user_name}", user_name)
            .replace("{conversation}", &conversation);

        let items: Vec<ForesightItem> = complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(en::FORESIGHT_GENERATION_SYSTEM),
                LlmMessage::user(user_prompt),
            ],
            0.3,
        )
        .await
        .unwrap_or_default();

        let mut records = Vec::with_capacity(items.len());
        for item in items {
            let text = item.foresight.clone();
            let vector = self.vectorizer.embed(&text).await.ok();

            records.push(ForesightRecord {
                id: None,
                user_id: Some(user_id.to_string()),
                group_id: group_id.map(String::from),
                timestamp: Utc::now(),
                foresight: item.foresight,
                evidence: item.evidence,
                start_time: item
                    .start_time
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                end_time: item.end_time.and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                duration_days: item.duration_days,
                vector,
                search_content: Some(text),
                is_deleted: false,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            });
        }

        Ok(records)
    }
}
