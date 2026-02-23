use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::llm::provider::{complete_json, LlmMessage, LlmProvider};
use crate::memory::prompts::{en, Locale};
use crate::storage::models::MemCell;

/// Result of boundary detection.
#[derive(Debug, Clone)]
pub enum BoundaryStatus {
    /// Conversation crossed a boundary — MemCell is complete and ready.
    Extracted(MemCell),
    /// Still accumulating — not enough for a complete MemCell yet.
    Accumulating,
}

#[derive(Deserialize)]
struct BoundaryResponse {
    is_boundary: bool,
    #[allow(dead_code)]
    reason: Option<String>,
}

/// Extracts MemCells by detecting topic boundaries via LLM.
/// Mirrors Python `ConvMemCellExtractor`.
pub struct MemCellExtractor {
    llm: Arc<dyn LlmProvider>,
    locale: Locale,
}

impl MemCellExtractor {
    pub fn new(llm: Arc<dyn LlmProvider>, locale: Locale) -> Self {
        Self { llm, locale }
    }

    /// Given accumulated history and the latest new messages,
    /// determine if a boundary has been crossed.
    ///
    /// `messages` — each item is a JSON `{role, content, sender}` object.
    pub async fn detect_boundary(
        &self,
        history: &[Value],
        new_messages: &[Value],
        user_id: Option<&str>,
        group_id: Option<&str>,
    ) -> Result<BoundaryStatus> {
        debug!(
            "MemCellExtractor::detect_boundary messages={}",
            new_messages.len()
        );

        let history_text = format_messages(history);
        let new_text = format_messages(new_messages);

        let (sys, usr) = match self.locale {
            Locale::Zh => (en::BOUNDARY_DETECTION_SYSTEM, en::BOUNDARY_DETECTION_USER),
            Locale::En => (en::BOUNDARY_DETECTION_SYSTEM, en::BOUNDARY_DETECTION_USER),
        };

        let user_prompt = usr
            .replace("{history}", &history_text)
            .replace("{new_messages}", &new_text);

        let resp: BoundaryResponse = complete_json(
            &*self.llm,
            vec![LlmMessage::system(sys), LlmMessage::user(user_prompt)],
            0.3,
        )
        .await?;

        if !resp.is_boundary {
            return Ok(BoundaryStatus::Accumulating);
        }

        // Build MemCell from the combined messages
        let all_messages: Vec<Value> = history.iter().chain(new_messages).cloned().collect();
        let participants = extract_participants(&all_messages);

        let cell = MemCell::new(
            user_id.map(String::from),
            group_id.map(String::from),
            Utc::now(),
            None, // summary filled by episode extractor
            Some(all_messages),
            Some(participants),
            None,
            None,
        );

        Ok(BoundaryStatus::Extracted(cell))
    }
}

fn format_messages(messages: &[Value]) -> String {
    messages
        .iter()
        .map(|m| {
            let sender = m["sender"].as_str().unwrap_or("unknown");
            let content = m["content"].as_str().unwrap_or("");
            format!("{sender}: {content}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_participants(messages: &[Value]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    messages
        .iter()
        .filter_map(|m| m["sender"].as_str())
        .filter(|s| seen.insert(*s))
        .map(String::from)
        .collect()
}

/// Convenience: format a conversation from serde_json Values into a readable string.
pub fn format_conversation(messages: &[Value]) -> String {
    format_messages(messages)
}
