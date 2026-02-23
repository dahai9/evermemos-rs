use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Chat message roles matching the Python LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

impl LlmMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
        }
    }
}

/// Core LLM provider trait.
/// All prompts in EverMemOS return structured JSON strings which are then
/// deserialized by the caller. Temperature = 0.3 for extraction, 0.0 for
/// agentic sufficiency / query generation.
///
/// Note: `complete_json` is intentionally NOT part of this trait because
/// generic type parameters make traits non-dyn-compatible. Use the free
/// function `complete_json()` instead.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Call the LLM and return the text content of the first choice.
    async fn complete(
        &self,
        messages: Vec<LlmMessage>,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<String>;
}

/// Free function: complete and deserialize JSON — works with `Arc<dyn LlmProvider>`.
/// Strips markdown code fences if the model wraps the JSON in them.
pub async fn complete_json<T: for<'de> Deserialize<'de>>(
    llm: &dyn LlmProvider,
    messages: Vec<LlmMessage>,
    temperature: f32,
) -> Result<T> {
    let raw = llm.complete(messages, temperature, None).await?;
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    Ok(serde_json::from_str(cleaned)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoLlm(String);

    #[async_trait]
    impl LlmProvider for EchoLlm {
        async fn complete(
            &self,
            _messages: Vec<LlmMessage>,
            _temperature: f32,
            _max_tokens: Option<u32>,
        ) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn strips_json_fence() {
        let llm = EchoLlm("```json\n{\"ok\":true}\n```".into());
        let v: serde_json::Value = complete_json(&llm, vec![], 0.0).await.unwrap();
        assert_eq!(v["ok"], true);
    }

    #[tokio::test]
    async fn strips_generic_fence() {
        let llm = EchoLlm("```\n{\"x\":1}\n```".into());
        let v: serde_json::Value = complete_json(&llm, vec![], 0.0).await.unwrap();
        assert_eq!(v["x"], 1);
    }

    #[tokio::test]
    async fn parses_plain_json() {
        let llm = EchoLlm("{\"n\":42}".into());
        let v: serde_json::Value = complete_json(&llm, vec![], 0.0).await.unwrap();
        assert_eq!(v["n"], 42);
    }

    #[test]
    fn llm_message_constructors() {
        let s = LlmMessage::system("hi");
        assert_eq!(s.role, LlmRole::System);
        assert_eq!(s.content, "hi");
        let u = LlmMessage::user("bye");
        assert_eq!(u.role, LlmRole::User);
        let a = LlmMessage::assistant("ok");
        assert_eq!(a.role, LlmRole::Assistant);
    }
}
