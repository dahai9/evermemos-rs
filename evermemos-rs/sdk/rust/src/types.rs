use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    pub status: String,
    pub message: String,
    pub result: Option<T>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: Option<String>,
    pub content: Option<String>,
    pub score: Option<f64>,
    pub timestamp: Option<String>,
    #[serde(alias = "memoryType")]
    pub memory_type: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FetchMemoriesResult {
    pub memories: Vec<MemoryItem>,
    pub total_count: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResult {
    pub memories: Vec<MemoryItem>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenericMapResult {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorizePayload {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<Value>>,
}

impl MemorizePayload {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    pub fn sender(mut self, sender: impl Into<String>) -> Self {
        self.sender = Some(sender.into());
        self
    }

    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn sender_name(mut self, sender_name: impl Into<String>) -> Self {
        self.sender_name = Some(sender_name.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    pub fn history(mut self, history: Vec<Value>) -> Self {
        self.history = Some(history);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub retrieve_method: Option<String>,
    pub memory_types: Option<Vec<String>>,
    pub top_k: Option<u32>,
    pub radius: Option<f32>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
}

impl SearchOptions {
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.retrieve_method = Some(method.into());
        self
    }

    pub fn with_memory_types(mut self, memory_types: Vec<String>) -> Self {
        self.memory_types = Some(memory_types);
        self
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    pub memory_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
}

impl FetchOptions {
    pub fn with_memory_type(mut self, memory_type: impl Into<String>) -> Self {
        self.memory_type = Some(memory_type.into());
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeleteOptions {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub memory_id: Option<String>,
}

impl DeleteOptions {
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    pub fn with_memory_id(mut self, memory_id: impl Into<String>) -> Self {
        self.memory_id = Some(memory_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleContentMessage {
    pub role: String,
    pub content: String,
}
