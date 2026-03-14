use crate::client::EverMemOSClient;
use crate::error::EverMemOSError;
use crate::types::{RoleContentMessage, SearchOptions};

#[derive(Clone)]
pub struct MemoryContextBuilder {
    client: EverMemOSClient,
    retrieve_method: String,
    top_k: u32,
    memory_types: Option<Vec<String>>,
}

impl MemoryContextBuilder {
    pub fn new(client: EverMemOSClient) -> Self {
        Self {
            client,
            retrieve_method: "HYBRID".to_string(),
            top_k: 5,
            memory_types: None,
        }
    }

    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.retrieve_method = method.into();
        self
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn with_memory_types(mut self, memory_types: Vec<String>) -> Self {
        self.memory_types = Some(memory_types);
        self
    }

    pub async fn build(&self, query: impl AsRef<str>) -> Result<String, EverMemOSError> {
        let memories = self
            .client
            .search(
                query.as_ref(),
                SearchOptions::default()
                    .with_method(self.retrieve_method.clone())
                    .with_top_k(self.top_k)
                    .with_memory_types(self.memory_types.clone().unwrap_or_default()),
            )
            .await?;

        if memories.is_empty() {
            return Ok("No relevant memory found.".to_string());
        }

        let lines = memories
            .iter()
            .enumerate()
            .map(|(index, memory)| {
                let score = memory
                    .score
                    .map(|value| format!(" (score={value:.3})"))
                    .unwrap_or_default();
                let memory_type = memory.memory_type.as_deref().unwrap_or("unknown");
                let content = memory.content.as_deref().unwrap_or("").trim();
                format!("{}. [{}]{} {}", index + 1, memory_type, score, content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(lines)
    }
}

pub fn compose_system_prompt(base_prompt: &str, memory_context: &str) -> String {
    if memory_context.trim().is_empty() {
        return base_prompt.to_string();
    }
    format!(
        "{}\n\nLong-term memory context:\n{}",
        base_prompt.trim(),
        memory_context.trim()
    )
}

pub fn build_openai_messages(user_input: &str, memory_context: &str, base_system_prompt: Option<&str>) -> Vec<RoleContentMessage> {
    let system = compose_system_prompt(
        base_system_prompt.unwrap_or("You are a helpful assistant."),
        memory_context,
    );
    vec![
        RoleContentMessage {
            role: "system".to_string(),
            content: system,
        },
        RoleContentMessage {
            role: "user".to_string(),
            content: user_input.to_string(),
        },
    ]
}

pub fn build_langchain_messages(user_input: &str, memory_context: &str, base_system_prompt: Option<&str>) -> Vec<RoleContentMessage> {
    let system = compose_system_prompt(
        base_system_prompt.unwrap_or("You are a helpful assistant."),
        memory_context,
    );
    vec![
        RoleContentMessage {
            role: "system".to_string(),
            content: system,
        },
        RoleContentMessage {
            role: "human".to_string(),
            content: user_input.to_string(),
        },
    ]
}

pub fn build_llamaindex_chat_history(user_input: &str, memory_context: &str, base_system_prompt: Option<&str>) -> Vec<RoleContentMessage> {
    let system = compose_system_prompt(
        base_system_prompt.unwrap_or("You are a helpful assistant."),
        memory_context,
    );
    vec![
        RoleContentMessage {
            role: "system".to_string(),
            content: system,
        },
        RoleContentMessage {
            role: "user".to_string(),
            content: user_input.to_string(),
        },
    ]
}
