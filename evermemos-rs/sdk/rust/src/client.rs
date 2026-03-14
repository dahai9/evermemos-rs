use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::EverMemOSError;
use crate::types::{
    ApiEnvelope, DeleteOptions, FetchMemoriesResult, FetchOptions, GenericMapResult, MemorizePayload,
    SearchOptions, SearchResult,
};

#[derive(Debug, Clone)]
pub struct EverMemOSConfig {
    pub base_url: String,
    pub org_id: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub api_key: Option<String>,
}

impl Default for EverMemOSConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            org_id: "default-org".to_string(),
            user_id: None,
            group_id: None,
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EverMemOSClientBuilder {
    config: EverMemOSConfig,
}

impl EverMemOSClientBuilder {
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.config.base_url = value.into().trim_end_matches('/').to_string();
        self
    }

    pub fn org_id(mut self, value: impl Into<String>) -> Self {
        self.config.org_id = value.into();
        self
    }

    pub fn user_id(mut self, value: impl Into<String>) -> Self {
        self.config.user_id = Some(value.into());
        self
    }

    pub fn group_id(mut self, value: impl Into<String>) -> Self {
        self.config.group_id = Some(value.into());
        self
    }

    pub fn api_key(mut self, value: impl Into<String>) -> Self {
        self.config.api_key = Some(value.into());
        self
    }

    pub fn build(self) -> Result<EverMemOSClient, EverMemOSError> {
        EverMemOSClient::new(self.config)
    }
}

#[derive(Clone)]
pub struct EverMemOSClient {
    http: Client,
    config: EverMemOSConfig,
}

impl EverMemOSClient {
    pub fn builder() -> EverMemOSClientBuilder {
        EverMemOSClientBuilder::default()
    }

    pub fn new(config: EverMemOSConfig) -> Result<Self, EverMemOSError> {
        if config.base_url.trim().is_empty() {
            return Err(EverMemOSError::InvalidConfig("base_url cannot be empty"));
        }
        if config.org_id.trim().is_empty() {
            return Err(EverMemOSError::InvalidConfig("org_id cannot be empty"));
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-Organization-Id",
            HeaderValue::from_str(&config.org_id)
                .map_err(|_| EverMemOSError::InvalidConfig("invalid org_id header"))?,
        );
        if let Some(api_key) = &config.api_key {
            let token = format!("Bearer {api_key}");
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&token)
                    .map_err(|_| EverMemOSError::InvalidConfig("invalid authorization header"))?,
            );
        }

        let http = Client::builder().default_headers(headers).build()?;
        Ok(Self { http, config })
    }

    pub async fn health(&self) -> Result<Value, EverMemOSError> {
        self.get_raw("/health").await
    }

    pub async fn memorize(&self, payload: MemorizePayload) -> Result<GenericMapResult, EverMemOSError> {
        let body = json!({
            "message_id": payload.message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            "create_time": payload.create_time.unwrap_or_else(|| Utc::now().to_rfc3339()),
            "sender": payload.sender.clone().unwrap_or_else(|| "User".to_string()),
            "sender_name": payload.sender_name.or_else(|| payload.sender).unwrap_or_else(|| "User".to_string()),
            "content": payload.content,
            "role": payload.role.unwrap_or_else(|| "user".to_string()),
            "user_id": payload.user_id.or_else(|| self.config.user_id.clone()),
            "group_id": payload.group_id.or_else(|| self.config.group_id.clone()),
            "history": payload.history,
        });

        self.request_json("POST", "/api/v1/memories", Some(body), true).await
    }

    pub async fn add_conversation(
        &self,
        user_message: impl Into<String>,
        assistant_message: impl Into<String>,
        user_name: Option<&str>,
        assistant_name: Option<&str>,
        user_id: Option<&str>,
        group_id: Option<&str>,
    ) -> Result<(GenericMapResult, GenericMapResult), EverMemOSError> {
        let user_message = user_message.into();
        let assistant_message = assistant_message.into();
        let user_res = self
            .memorize(
                MemorizePayload::new(user_message.clone())
                    .sender(user_name.unwrap_or("User"))
                    .sender_name(user_name.unwrap_or("User"))
                    .role("user")
                    .user_id_opt(user_id)
                    .group_id_opt(group_id),
            )
            .await?;

        let assistant_res = self
            .memorize(
                MemorizePayload::new(assistant_message)
                    .sender(assistant_name.unwrap_or("Assistant"))
                    .sender_name(assistant_name.unwrap_or("Assistant"))
                    .role("assistant")
                    .user_id_opt(user_id)
                    .group_id_opt(group_id)
                    .history(vec![json!({"role": "user", "content": user_message})]),
            )
            .await?;

        Ok((user_res, assistant_res))
    }

    pub async fn search(
        &self,
        query: impl AsRef<str>,
        options: SearchOptions,
    ) -> Result<Vec<crate::types::MemoryItem>, EverMemOSError> {
        let mut params: Vec<(String, String)> = vec![
            ("query".to_string(), query.as_ref().to_string()),
            (
                "retrieve_method".to_string(),
                options.retrieve_method.unwrap_or_else(|| "HYBRID".to_string()),
            ),
            ("top_k".to_string(), options.top_k.unwrap_or(5).to_string()),
        ];

        if let Some(user_id) = options.user_id.or_else(|| self.config.user_id.clone()) {
            params.push(("user_id".to_string(), user_id));
        }
        if let Some(group_id) = options.group_id.or_else(|| self.config.group_id.clone()) {
            params.push(("group_id".to_string(), group_id));
        }
        if let Some(memory_types) = options.memory_types {
            if !memory_types.is_empty() {
                params.push(("memory_types".to_string(), memory_types.join(",")));
            }
        }
        if let Some(radius) = options.radius {
            params.push(("radius".to_string(), radius.to_string()));
        }

        let path = format!("/api/v1/memories/search?{}", serde_urlencoded::to_string(params).map_err(|_| EverMemOSError::InvalidConfig("invalid query params"))?);
        let result: SearchResult = self.request_json("GET", &path, None, true).await?;
        Ok(result.memories)
    }

    pub async fn fetch_memories(&self, options: FetchOptions) -> Result<FetchMemoriesResult, EverMemOSError> {
        let mut params: Vec<(String, String)> = vec![
            ("limit".to_string(), options.limit.unwrap_or(20).to_string()),
            ("offset".to_string(), options.offset.unwrap_or(0).to_string()),
        ];
        if let Some(user_id) = options.user_id.or_else(|| self.config.user_id.clone()) {
            params.push(("user_id".to_string(), user_id));
        }
        if let Some(group_id) = options.group_id.or_else(|| self.config.group_id.clone()) {
            params.push(("group_id".to_string(), group_id));
        }
        if let Some(memory_type) = options.memory_type {
            params.push(("memory_type".to_string(), memory_type));
        }
        let path = format!("/api/v1/memories?{}", serde_urlencoded::to_string(params).map_err(|_| EverMemOSError::InvalidConfig("invalid query params"))?);
        self.request_json("GET", &path, None, true).await
    }

    pub async fn get_profile(&self, memory_type: &str, options: FetchOptions) -> Result<Vec<crate::types::MemoryItem>, EverMemOSError> {
        let result = self.fetch_memories(FetchOptions {
            memory_type: Some(memory_type.to_string()),
            limit: options.limit.or(Some(20)),
            offset: options.offset,
            user_id: options.user_id,
            group_id: options.group_id,
        }).await?;
        Ok(result.memories)
    }

    pub async fn delete_memories(&self, options: DeleteOptions) -> Result<GenericMapResult, EverMemOSError> {
        let body = json!({
            "user_id": options.user_id.or_else(|| self.config.user_id.clone()),
            "group_id": options.group_id.or_else(|| self.config.group_id.clone()),
            "memory_id": options.memory_id,
        });
        self.request_json("DELETE", "/api/v1/memories", Some(body), true).await
    }

    async fn get_raw(&self, path: &str) -> Result<Value, EverMemOSError> {
        let response = self.http.get(format!("{}{}", self.config.base_url, path)).send().await?;
        Ok(response.json::<Value>().await?)
    }

    async fn request_json<T: DeserializeOwned + Default>(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        unwrap_envelope: bool,
    ) -> Result<T, EverMemOSError> {
        let url = format!("{}{}", self.config.base_url, path);
        let builder = match method {
            "POST" => self.http.post(url),
            "DELETE" => self.http.delete(url),
            _ => self.http.get(url),
        };
        let response = match body {
            Some(body) => builder.json(&body).send().await?,
            None => builder.send().await?,
        };
        let response = response.error_for_status()?;
        if unwrap_envelope {
            let envelope = response.json::<ApiEnvelope<T>>().await?;
            if envelope.status != "success" && envelope.status != "ok" {
                return Err(EverMemOSError::ApiStatus {
                    status: envelope.status,
                    message: envelope.message,
                });
            }
            Ok(envelope.result.unwrap_or_default())
        } else {
            Ok(response.json::<T>().await?)
        }
    }
}

trait OptionalScopeExt {
    fn user_id_opt(self, value: Option<&str>) -> Self;
    fn group_id_opt(self, value: Option<&str>) -> Self;
}

impl OptionalScopeExt for MemorizePayload {
    fn user_id_opt(mut self, value: Option<&str>) -> Self {
        if let Some(value) = value {
            self.user_id = Some(value.to_string());
        }
        self
    }

    fn group_id_opt(mut self, value: Option<&str>) -> Self {
        if let Some(value) = value {
            self.group_id = Some(value.to_string());
        }
        self
    }
}
