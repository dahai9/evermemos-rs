use anyhow::{Context, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestAssistantMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;

use crate::config::LlmConfig;
use super::provider::{LlmMessage, LlmProvider, LlmRole};

/// OpenAI-compatible LLM provider (works with Ollama, vLLM, any OpenAI-API endpoint).
#[derive(Clone)]
pub struct OpenAiProvider {
    client: Client<OpenAIConfig>,
    model: String,
    max_tokens: u32,
}

impl OpenAiProvider {
    pub fn new(cfg: &LlmConfig) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(&cfg.api_key)
            .with_api_base(&cfg.base_url);

        Self {
            client: Client::with_config(config),
            model: cfg.model.clone(),
            max_tokens: cfg.max_tokens,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        messages: Vec<LlmMessage>,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let chat_messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|m| match m.role {
                LlmRole::System => ChatCompletionRequestSystemMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
                LlmRole::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
                LlmRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
            })
            .collect();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(chat_messages)
            .temperature(temperature)
            .max_tokens(max_tokens.unwrap_or(self.max_tokens))
            .build()
            .context("Failed to build chat completion request")?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("LLM API call failed")?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }
}
