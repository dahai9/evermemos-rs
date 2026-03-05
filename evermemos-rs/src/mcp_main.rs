//! `evermemos-mcp` — MCP server binary for EverMemOS
//!
//! Bridges any MCP-compatible AI client (Claude Desktop, Cursor, Continue …)
//! to an EverMemOS HTTP API server via stdio / JSON-RPC 2.0.
//!
//! # Quick start
//!
//! 1. Start the main server:
//!    ```bash
//!    just start          # or: cargo run --bin evermemos
//!    ```
//!
//! 2. Build the MCP binary:
//!    ```bash
//!    just build-mcp
//!    ```
//!
//! 3. Register in Claude Desktop (`~/Library/Application Support/Claude/claude_desktop_config.json`):
//!    ```json
//!    {
//!      "mcpServers": {
//!        "evermemos": {
//!          "command": "/absolute/path/to/evermemos-mcp",
//!          "env": {
//!            "EVERMEMOS_BASE_URL": "http://localhost:8080",
//!            "EVERMEMOS_GROUP_ID": "my_chat",
//!            "EVERMEMOS_USER_ID": "alice"
//!          }
//!        }
//!      }
//!    }
//!    ```
//!
//! 4. Available tools exposed to the model:
//!    - `search_memory`    — search long-term memory before answering
//!    - `add_memory`       — store a single message
//!    - `add_conversation` — store a full user+assistant turn at once
//!    - `get_profile`      — retrieve user profile / core memory
//!    - `add_behavior_history` — store one behavior-history record (feature: `behavior-history`)
//!    - `list_behavior_history` — list behavior-history records (feature: `behavior-history`)
//!    - `get_behavior_history_stats` — get behavior-history stats (feature: `behavior-history`)

use evermemos_rs::mcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (useful for local dev)
    let _ = dotenvy::dotenv();
    mcp::run_stdio_server().await
}
