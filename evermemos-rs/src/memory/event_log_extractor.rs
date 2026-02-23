use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::llm::vectorize::VectorizeService;
use crate::memory::memcell_extractor::format_conversation;
use crate::memory::prompts::en;
use crate::storage::models::EventLogRecord;

#[derive(Deserialize)]
struct FactItem {
    atomic_fact: String,
}

pub struct EventLogExtractor {
    llm: Arc<dyn LlmProvider>,
    vectorizer: Arc<dyn VectorizeService>,
    #[allow(dead_code)]
    vector_model: String,
}

impl EventLogExtractor {
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        vectorizer: Arc<dyn VectorizeService>,
        vector_model: String,
    ) -> Self {
        Self { llm, vectorizer, vector_model }
    }

    pub async fn extract(
        &self,
        messages: &[Value],
        user_id: &str,
        user_name: &str,
        group_id: Option<&str>,
    ) -> Result<Vec<EventLogRecord>> {
        debug!("EventLogExtractor::extract user={user_name}");

        let conversation = format_conversation(messages);
        let user_prompt = en::EVENT_LOG_USER
            .replace("{user_name}", user_name)
            .replace("{conversation}", &conversation);

        let items: Vec<FactItem> = complete_json(
            &*self.llm,
            vec![
                LlmMessage::system(en::EVENT_LOG_SYSTEM),
                LlmMessage::user(user_prompt),
            ],
            0.3,
        )
        .await
        .unwrap_or_default();

        let mut records = Vec::with_capacity(items.len());
        for item in items {
            let text = item.atomic_fact.clone();
            let vector = self.vectorizer.embed(&text).await.ok();

            records.push(EventLogRecord {
                id: None,
                user_id: Some(user_id.to_string()),
                group_id: group_id.map(String::from),
                timestamp: Utc::now(),
                atomic_fact: item.atomic_fact,
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
