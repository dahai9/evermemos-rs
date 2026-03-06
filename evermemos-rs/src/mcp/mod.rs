//! MCP (Model Context Protocol) server — stdio transport
//!
//! Implements JSON-RPC 2.0 over stdin/stdout so that any MCP-compatible
//! client (Claude Desktop, Cursor, Continue, etc.) can use EverMemOS as
//! a persistent-memory backend without code changes.
//!
//! # Architecture
//!
//! ```
//! Claude Desktop ──stdio──▶ evermemos-mcp binary
//!                                  │
//!                          HTTP API calls
//!                                  │
//!                       evermemos-rs server
//!                      (http://localhost:8080)
//! ```
//!
//! # Environment variables
//! | Var | Default | Description |
//! |-----|---------|-------------|
//! | `EVERMEMOS_BASE_URL` | `http://localhost:8080` | evermemos-rs server URL |
//! | `EVERMEMOS_GROUP_ID` | `default_group` | Conversation / group scope |
//! | `EVERMEMOS_USER_ID` | `default_user` | User scope |
//! | `EVERMEMOS_API_KEY` | *(empty)* | Bearer token if server has auth |
//! | `EVERMEMOS_RETRIEVE_METHOD` | `hybrid` | Default retrieval method |
//!
//! # Claude Desktop claude_desktop_config.json example
//! ```json
//! {
//!   "mcpServers": {
//!     "evermemos": {
//!       "command": "/path/to/evermemos-mcp",
//!       "env": {
//!         "EVERMEMOS_BASE_URL": "http://localhost:8080",
//!         "EVERMEMOS_GROUP_ID": "alice_chat",
//!         "EVERMEMOS_USER_ID": "alice"
//!       }
//!     }
//!   }
//! }
//! ```

use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use uuid::Uuid;

static PENDING_HISTORY: Lazy<Mutex<HashMap<String, Vec<Value>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ─────────────────────────────────────────────────────────────────────────────
// JSON-RPC 2.0 wire types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime configuration (from env)
// ─────────────────────────────────────────────────────────────────────────────

struct McpConfig {
    base_url: String,
    group_id: String,
    user_id: String,
    api_key: String,
    retrieve_method: String,
}

impl McpConfig {
    fn from_env() -> Self {
        Self {
            base_url: env::var("EVERMEMOS_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            group_id: env::var("EVERMEMOS_GROUP_ID")
                .unwrap_or_else(|_| "default_group".to_string()),
            user_id: env::var("EVERMEMOS_USER_ID").unwrap_or_else(|_| "default_user".to_string()),
            api_key: env::var("EVERMEMOS_API_KEY").unwrap_or_default(),
            retrieve_method: env::var("EVERMEMOS_RETRIEVE_METHOD")
                .unwrap_or_else(|_| "hybrid".to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool schema definitions
// ─────────────────────────────────────────────────────────────────────────────

fn tools_list() -> Value {
    #[cfg(feature = "behavior-history")]
    let mut tools = vec![
        json!({
            "name": "search_memory",
            "description": concat!(
                "Search the user's long-term memory for relevant information from past ",
                "conversations — including preferences, facts, experiences, and profiles. ",
                "Call this BEFORE answering questions that involve personal context, history, ",
                "or anything the user may have mentioned before."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What to search for — phrase it as a question or topic"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "How many memory items to return (default: 5, max: 20)",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    },
                    "retrieve_method": {
                        "type": "string",
                        "enum": ["keyword", "vector", "hybrid", "rrf", "agentic"],
                        "description": "Retrieval algorithm. 'hybrid' balances recall and precision. 'agentic' uses LLM-guided multi-round search (slower but deeper).",
                        "default": "hybrid"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "add_memory",
            "description": concat!(
                "Store an important message or fact into the user's long-term memory. ",
                "Use this to save key things the user shares about themselves, their goals, ",
                "preferences, or decisions made during the conversation."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message or fact to store"
                    },
                    "sender": {
                        "type": "string",
                        "description": "Name of the sender (e.g. 'Alice', 'User', 'Assistant'). Default: 'User'"
                    },
                    "role": {
                        "type": "string",
                        "enum": ["user", "assistant"],
                        "description": "Message role — 'user' for things the human said, 'assistant' for AI responses",
                        "default": "user"
                    }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "get_profile",
            "description": concat!(
                "Retrieve the user's stored profile including their characteristics, ",
                "interests, habits, and personal background collected from past conversations. ",
                "Useful for personalizing responses."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memory_type": {
                        "type": "string",
                        "enum": ["user_profile", "core_memory", "episodic_memory"],
                        "description": "Which type of memory to retrieve (default: user_profile)",
                        "default": "user_profile"
                    }
                }
            }
        }),
        json!({
            "name": "add_conversation",
            "description": concat!(
                "Store an entire conversation turn (user message + assistant response) at once. ",
                "More efficient than calling add_memory twice when you want to persist a full exchange."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "user_message": {
                        "type": "string",
                        "description": "The user's message"
                    },
                    "assistant_message": {
                        "type": "string",
                        "description": "The assistant's response"
                    },
                    "user_name": {
                        "type": "string",
                        "description": "User's display name (default: 'User')"
                    }
                },
                "required": ["user_message", "assistant_message"]
            }
        }),
    ];

    #[cfg(not(feature = "behavior-history"))]
    let tools = vec![
        json!({
            "name": "search_memory",
            "description": concat!(
                "Search the user's long-term memory for relevant information from past ",
                "conversations — including preferences, facts, experiences, and profiles. ",
                "Call this BEFORE answering questions that involve personal context, history, ",
                "or anything the user may have mentioned before."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What to search for — phrase it as a question or topic"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "How many memory items to return (default: 5, max: 20)",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    },
                    "retrieve_method": {
                        "type": "string",
                        "enum": ["keyword", "vector", "hybrid", "rrf", "agentic"],
                        "description": "Retrieval algorithm. 'hybrid' balances recall and precision. 'agentic' uses LLM-guided multi-round search (slower but deeper).",
                        "default": "hybrid"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "add_memory",
            "description": concat!(
                "Store an important message or fact into the user's long-term memory. ",
                "Use this to save key things the user shares about themselves, their goals, ",
                "preferences, or decisions made during the conversation."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message or fact to store"
                    },
                    "sender": {
                        "type": "string",
                        "description": "Name of the sender (e.g. 'Alice', 'User', 'Assistant'). Default: 'User'"
                    },
                    "role": {
                        "type": "string",
                        "enum": ["user", "assistant"],
                        "description": "Message role — 'user' for things the human said, 'assistant' for AI responses",
                        "default": "user"
                    }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "get_profile",
            "description": concat!(
                "Retrieve the user's stored profile including their characteristics, ",
                "interests, habits, and personal background collected from past conversations. ",
                "Useful for personalizing responses."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "memory_type": {
                        "type": "string",
                        "enum": ["user_profile", "core_memory", "episodic_memory"],
                        "description": "Which type of memory to retrieve (default: user_profile)",
                        "default": "user_profile"
                    }
                }
            }
        }),
        json!({
            "name": "add_conversation",
            "description": concat!(
                "Store an entire conversation turn (user message + assistant response) at once. ",
                "More efficient than calling add_memory twice when you want to persist a full exchange."
            ),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "user_message": {
                        "type": "string",
                        "description": "The user's message"
                    },
                    "assistant_message": {
                        "type": "string",
                        "description": "The assistant's response"
                    },
                    "user_name": {
                        "type": "string",
                        "description": "User's display name (default: 'User')"
                    }
                },
                "required": ["user_message", "assistant_message"]
            }
        }),
    ];

    #[cfg(feature = "behavior-history")]
    {
        tools.push(json!({
            "name": "add_behavior_history",
            "description": "Store one behavior-history record for the current MCP user.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "behavior_type": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Behavior tags, e.g. ['chat', 'follow-up']"
                    },
                    "event_id": {
                        "type": "string",
                        "description": "Optional related event/memory id"
                    },
                    "meta": {
                        "type": "object",
                        "description": "Optional metadata payload"
                    },
                    "timestamp": {
                        "type": "string",
                        "description": "Optional RFC3339 timestamp; defaults to now"
                    }
                },
                "required": ["behavior_type"]
            }
        }));

        tools.push(json!({
            "name": "list_behavior_history",
            "description": "List behavior-history records for the current MCP user.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "behavior_type": {
                        "type": "string",
                        "description": "Optional behavior tag filter"
                    },
                    "start_time": {
                        "type": "string",
                        "description": "Optional RFC3339 start time"
                    },
                    "end_time": {
                        "type": "string",
                        "description": "Optional RFC3339 end time"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional max records (default 50, max 500)",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 500
                    }
                }
            }
        }));

        tools.push(json!({
            "name": "get_behavior_history_stats",
            "description": "Get behavior-history statistics for the current MCP user.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }));
    }

    json!({ "tools": tools })
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP API call helpers
// ─────────────────────────────────────────────────────────────────────────────

fn add_auth(req: reqwest::RequestBuilder, cfg: &McpConfig) -> reqwest::RequestBuilder {
    if cfg.api_key.is_empty() {
        req
    } else {
        req.bearer_auth(&cfg.api_key)
    }
}

fn pending_key(cfg: &McpConfig) -> String {
    format!("{}::{}", cfg.group_id, cfg.user_id)
}

/// POST /api/v1/memories — store a single message
async fn api_add_single(
    client: &reqwest::Client,
    cfg: &McpConfig,
    content: impl Into<String>,
    sender: impl Into<String>,
    role: impl Into<String>,
) -> anyhow::Result<Value> {
    let url = format!("{}/api/v1/memories", cfg.base_url);
    let content = content.into();
    let sender = sender.into();
    let role = role.into();
    let message_id = Uuid::new_v4().to_string();
    let create_time = Utc::now().to_rfc3339();
    let current_message = json!({
        "message_id": message_id.clone(),
        "create_time": create_time.clone(),
        "sender": sender.clone(),
        "content": content.clone(),
        "role": role.clone(),
    });

    let key = pending_key(cfg);
    let history = {
        let guard = PENDING_HISTORY.lock().await;
        guard.get(&key).cloned().unwrap_or_default()
    };

    let body = json!({
        "message_id": message_id,
        "create_time": create_time,
        "sender": sender,
        "content": content,
        "role": role,
        "group_id": cfg.group_id,
        "user_id": cfg.user_id,
        "history": history,
    });
    let resp = add_auth(client.post(&url).json(&body), cfg)
        .send()
        .await?
        .json::<Value>()
        .await?;

    let status = resp
        .get("result")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str);

    let mut guard = PENDING_HISTORY.lock().await;
    match status {
        Some("accumulating") => {
            guard.entry(key).or_default().push(current_message);
        }
        Some("extracted") => {
            guard.remove(&key);
        }
        _ => {}
    }

    Ok(resp)
}

/// Tool: search_memory
async fn tool_search(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let query = args["query"].as_str().unwrap_or("").to_string();
    if query.is_empty() {
        return Ok("Error: 'query' is required".to_string());
    }
    let top_k = args["top_k"].as_u64().unwrap_or(5).to_string();
    let method = args["retrieve_method"]
        .as_str()
        .unwrap_or(&cfg.retrieve_method)
        .to_uppercase();

    let url = format!("{}/api/v1/memories/search", cfg.base_url);
    let resp = add_auth(
        client.get(&url).query(&[
            ("query", query.as_str()),
            ("group_id", cfg.group_id.as_str()),
            ("user_id", cfg.user_id.as_str()),
            ("top_k", top_k.as_str()),
            ("retrieve_method", method.as_str()),
        ]),
        cfg,
    )
    .send()
    .await?
    .json::<Value>()
    .await?;

    // Extract and format memories for LLM readability
    let memories = resp
        .get("result")
        .and_then(|r| r.get("memories"))
        .or_else(|| resp.get("memories"))
        .and_then(|m| m.as_array());

    match memories {
        Some(items) if items.is_empty() => Ok("No relevant memories found.".to_string()),
        Some(items) => {
            let mut out = format!("Found {} memory item(s):\n\n", items.len());
            for (i, item) in items.iter().enumerate() {
                let mem_type = item
                    .get("memory_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let subject = item.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                let content = item
                    .get("episode")
                    .or_else(|| item.get("content"))
                    .or_else(|| item.get("profile"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let score = item
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .map(|s| format!(" (score: {s:.3})"))
                    .unwrap_or_default();

                out.push_str(&format!("[{idx}] [{mem_type}]{score}\n", idx = i + 1));
                if !subject.is_empty() {
                    out.push_str(&format!("  Subject: {subject}\n"));
                }
                if !content.is_empty() {
                    out.push_str(&format!("  Content: {content}\n"));
                }
                out.push('\n');
            }
            Ok(out)
        }
        None => Ok(serde_json::to_string_pretty(&resp).unwrap_or_default()),
    }
}

/// Tool: add_memory
async fn tool_add_memory(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let content = args["content"].as_str().unwrap_or("").to_string();
    if content.is_empty() {
        return Ok("Error: 'content' is required".to_string());
    }
    let sender = args["sender"].as_str().unwrap_or("User");
    let role = args["role"].as_str().unwrap_or("user");

    api_add_single(client, cfg, &content, sender, role).await?;
    Ok(format!("Stored: \"{content}\""))
}

/// Tool: get_profile
async fn tool_get_profile(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let mem_type = args["memory_type"].as_str().unwrap_or("user_profile");
    let url = format!("{}/api/v1/memories", cfg.base_url);
    let resp = add_auth(
        client.get(&url).query(&[
            ("user_id", cfg.user_id.as_str()),
            ("group_id", cfg.group_id.as_str()),
            ("memory_type", mem_type),
            ("limit", "20"),
        ]),
        cfg,
    )
    .send()
    .await?
    .json::<Value>()
    .await?;

    let memories = resp
        .get("result")
        .and_then(|r| r.get("memories"))
        .or_else(|| resp.get("memories"))
        .and_then(|m| m.as_array());

    match memories {
        Some(items) if items.is_empty() => Ok(format!("No {mem_type} stored yet.")),
        Some(items) => {
            let mut out = format!("User profile ({} entries):\n\n", items.len());
            for item in items {
                let content = item
                    .get("profile")
                    .or_else(|| item.get("content"))
                    .or_else(|| item.get("episode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !content.is_empty() {
                    out.push_str(&format!("• {content}\n"));
                }
            }
            Ok(out)
        }
        None => Ok(serde_json::to_string_pretty(&resp).unwrap_or_default()),
    }
}

/// Tool: add_conversation (user + assistant in one call)
async fn tool_add_conversation(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let user_msg = args["user_message"].as_str().unwrap_or("").to_string();
    let asst_msg = args["assistant_message"].as_str().unwrap_or("").to_string();
    let user_name = args["user_name"].as_str().unwrap_or("User");

    if user_msg.is_empty() || asst_msg.is_empty() {
        return Ok("Error: 'user_message' and 'assistant_message' are required".to_string());
    }

    // Store sequentially to maintain order
    api_add_single(client, cfg, &user_msg, user_name, "user").await?;
    api_add_single(client, cfg, &asst_msg, "Assistant", "assistant").await?;

    Ok(format!(
        "Stored conversation turn ({} + {} chars).",
        user_msg.len(),
        asst_msg.len()
    ))
}

#[cfg(feature = "behavior-history")]
/// Tool: add_behavior_history
async fn tool_add_behavior_history(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let behavior_type = args["behavior_type"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if behavior_type.is_empty() {
        return Ok("Error: 'behavior_type' must be a non-empty string array".to_string());
    }

    let mut body = json!({
        "user_id": cfg.user_id,
        "behavior_type": behavior_type,
    });

    if let Some(event_id) = args["event_id"].as_str() {
        body["event_id"] = json!(event_id);
    }
    if args["meta"].is_object() {
        body["meta"] = args["meta"].clone();
    }
    if let Some(timestamp) = args["timestamp"].as_str() {
        body["timestamp"] = json!(timestamp);
    }

    let url = format!("{}/api/v1/behavior-history", cfg.base_url);
    let resp = add_auth(client.post(&url).json(&body), cfg)
        .send()
        .await?
        .json::<Value>()
        .await?;

    let id = resp
        .get("result")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    Ok(format!("Stored behavior history record: {id}"))
}

#[cfg(feature = "behavior-history")]
/// Tool: list_behavior_history
async fn tool_list_behavior_history(
    client: &reqwest::Client,
    cfg: &McpConfig,
    args: &Value,
) -> anyhow::Result<String> {
    let limit = args["limit"].as_u64().unwrap_or(50).min(500).to_string();
    let mut query: Vec<(String, String)> = vec![
        ("user_id".to_string(), cfg.user_id.clone()),
        ("limit".to_string(), limit),
    ];

    if let Some(v) = args["behavior_type"].as_str() {
        query.push(("behavior_type".to_string(), v.to_string()));
    }
    if let Some(v) = args["start_time"].as_str() {
        query.push(("start_time".to_string(), v.to_string()));
    }
    if let Some(v) = args["end_time"].as_str() {
        query.push(("end_time".to_string(), v.to_string()));
    }

    let url = format!("{}/api/v1/behavior-history", cfg.base_url);
    let resp = add_auth(client.get(&url).query(&query), cfg)
        .send()
        .await?
        .json::<Value>()
        .await?;

    let records = resp
        .get("result")
        .and_then(|v| v.get("records"))
        .and_then(|v| v.as_array());

    match records {
        Some(items) if items.is_empty() => Ok("No behavior history records found.".to_string()),
        Some(items) => {
            let mut out = format!("Behavior history ({} records):\n\n", items.len());
            for (idx, item) in items.iter().enumerate() {
                let ts = item
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let tags = item
                    .get("behavior_type")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "-".to_string());
                let event_id = item.get("event_id").and_then(|v| v.as_str()).unwrap_or("-");

                out.push_str(&format!(
                    "[{i}] {ts}\n  behavior_type: {tags}\n  event_id: {event_id}\n\n",
                    i = idx + 1
                ));
            }
            Ok(out)
        }
        None => Ok(serde_json::to_string_pretty(&resp).unwrap_or_default()),
    }
}

#[cfg(feature = "behavior-history")]
/// Tool: get_behavior_history_stats
async fn tool_get_behavior_history_stats(
    client: &reqwest::Client,
    cfg: &McpConfig,
    _args: &Value,
) -> anyhow::Result<String> {
    let url = format!("{}/api/v1/behavior-history/stats", cfg.base_url);
    let resp = add_auth(
        client.get(&url).query(&[("user_id", cfg.user_id.as_str())]),
        cfg,
    )
    .send()
    .await?
    .json::<Value>()
    .await?;

    let total = resp
        .get("result")
        .and_then(|v| v.get("total_records"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let breakdown = resp
        .get("result")
        .and_then(|v| v.get("type_breakdown"))
        .cloned()
        .unwrap_or(json!({}));

    Ok(format!(
        "Behavior history stats for user={}:\n- total_records: {}\n- type_breakdown: {}",
        cfg.user_id,
        total,
        serde_json::to_string(&breakdown).unwrap_or_else(|_| "{}".to_string())
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool dispatcher
// ─────────────────────────────────────────────────────────────────────────────

async fn dispatch_tool(
    client: &reqwest::Client,
    cfg: &McpConfig,
    name: &str,
    args: &Value,
) -> Result<String, String> {
    let result = match name {
        "search_memory" => tool_search(client, cfg, args).await,
        "add_memory" => tool_add_memory(client, cfg, args).await,
        "get_profile" => tool_get_profile(client, cfg, args).await,
        "add_conversation" => tool_add_conversation(client, cfg, args).await,
        #[cfg(feature = "behavior-history")]
        "add_behavior_history" => tool_add_behavior_history(client, cfg, args).await,
        #[cfg(feature = "behavior-history")]
        "list_behavior_history" => tool_list_behavior_history(client, cfg, args).await,
        #[cfg(feature = "behavior-history")]
        "get_behavior_history_stats" => tool_get_behavior_history_stats(client, cfg, args).await,
        #[cfg(not(feature = "behavior-history"))]
        "add_behavior_history" | "list_behavior_history" | "get_behavior_history_stats" => Ok(
            "BehaviorHistory feature is disabled. Rebuild with --features behavior-history"
                .to_string(),
        ),
        _ => return Err(format!("Unknown tool: {name}")),
    };

    match result {
        Ok(text) => Ok(text),
        Err(e) => Err(format!("Tool error: {e}")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// stdio server loop
// ─────────────────────────────────────────────────────────────────────────────

/// Run the MCP server on stdin/stdout.
/// All logs go to stderr so stdout stays clean for JSON-RPC.
pub async fn run_stdio_server() -> anyhow::Result<()> {
    let cfg = McpConfig::from_env();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader = BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    eprintln!(
        "[evermemos-mcp] started  base_url={}  group={}  user={}",
        cfg.base_url, cfg.group_id, cfg.user_id
    );

    while let Some(line) = reader.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        eprintln!("[evermemos-mcp] ← {line}");

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[evermemos-mcp] parse error: {e}");
                // Send parse error response (id unknown → null)
                let resp = JsonRpcResponse::err(Value::Null, -32700, format!("Parse error: {e}"));
                write_response(&mut writer, &resp).await?;
                continue;
            }
        };

        let id = request.id.clone().unwrap_or(Value::Null);
        let is_notification = request.id.is_none();

        let response: Option<JsonRpcResponse> = match request.method.as_str() {
            // ── Handshake ──────────────────────────────────────────────────────
            "initialize" => Some(JsonRpcResponse::ok(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "evermemos-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )),

            // ── Notifications (fire-and-forget — no response) ─────────────────
            m if m.starts_with("notifications/") => {
                eprintln!("[evermemos-mcp] notification: {m}");
                None
            }

            // ── Tool list ─────────────────────────────────────────────────────
            "tools/list" => Some(JsonRpcResponse::ok(id, tools_list())),

            // ── Tool call ─────────────────────────────────────────────────────
            "tools/call" => {
                let params = request.params.as_ref().unwrap_or(&Value::Null);
                let tool_name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                eprintln!("[evermemos-mcp] tool={tool_name} args={args}");

                let resp_body = match dispatch_tool(&client, &cfg, tool_name, args).await {
                    Ok(text) => json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                    Err(e) => json!({
                        "content": [{ "type": "text", "text": e }],
                        "isError": true
                    }),
                };

                Some(JsonRpcResponse::ok(id, resp_body))
            }

            // ── Unknown method ────────────────────────────────────────────────
            other => {
                eprintln!("[evermemos-mcp] unknown method: {other}");
                if is_notification {
                    None // no response for unknown notifications
                } else {
                    Some(JsonRpcResponse::err(
                        id,
                        -32601,
                        format!("Method not found: {other}"),
                    ))
                }
            }
        };

        if let Some(r) = response {
            write_response(&mut writer, &r).await?;
        }
    }

    eprintln!("[evermemos-mcp] stdin closed, exiting");
    Ok(())
}

async fn write_response(
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    resp: &JsonRpcResponse,
) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(resp)?;
    eprintln!("[evermemos-mcp] → {line}");
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}
