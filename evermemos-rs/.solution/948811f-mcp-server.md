# Solution: MCP Server (evermemos-mcp)

**Commit**: `948811f`  
**Date**: 2026-02-26

## Problem

MCP tool-call 模式下模型懒惰，几乎不主动调用记忆工具。

根本原因：tool-call 让模型"自主决定"，模型认为自己够聪明不需要查。

## Solution

实现 `evermemos-mcp` 二进制：stdio JSON-RPC 2.0 MCP server，
让 Claude Desktop / Cursor 等客户端"知道"记忆工具存在，
配合 system prompt 说明（让模型知道应该主动调用）解决懒惰问题。

真正解决懒惰的方案见 `/chat/completions` 代理（待实现）。

## Files Changed

| File | Action |
|------|--------|
| `src/mcp/mod.rs` | NEW — stdio server 实现 (≈370 行) |
| `src/mcp_main.rs` | NEW — binary 入口 + 文档 |
| `src/lib.rs` | `pub mod mcp;` |
| `Cargo.toml` | `[[bin]] evermemos-mcp` |
| `justfile` | `build-mcp`, `build-mcp-release` recipes |

## Architecture

```
Claude Desktop ──stdio──▶ evermemos-mcp
                    JSON-RPC 2.0
                                │
                        reqwest HTTP
                                │
                   evermemos-rs :8080
```

## Tools Exposed

| Tool | Description |
|------|-------------|
| `search_memory` | keyword/vector/hybrid/rrf/agentic 检索 |
| `add_memory` | 存储单条消息 |
| `add_conversation` | 存储完整一轮对话 (user + assistant) |
| `get_profile` | 获取用户画像 / core_memory |

## Config (env vars)

| Var | Default | 说明 |
|-----|---------|------|
| `EVERMEMOS_BASE_URL` | `http://localhost:8080` | evermemos-rs 地址 |
| `EVERMEMOS_GROUP_ID` | `default_group` | 会话/群组 ID |
| `EVERMEMOS_USER_ID` | `default_user` | 用户 ID |
| `EVERMEMOS_API_KEY` | *(empty)* | Bearer token |
| `EVERMEMOS_RETRIEVE_METHOD` | `hybrid` | 默认检索方法 |

## Usage — Claude Desktop

Build:
```bash
just build-mcp
# binary at: evermemos-rs/target/debug/evermemos-mcp
```

`~/Library/Application Support/Claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "evermemos": {
      "command": "/absolute/path/to/evermemos-mcp",
      "env": {
        "EVERMEMOS_BASE_URL": "http://localhost:8080",
        "EVERMEMOS_GROUP_ID": "alice_chat",
        "EVERMEMOS_USER_ID": "alice"
      }
    }
  }
}
```

## Known Limitation — tool-call laziness

MCP tool-call 模式本质上依赖模型主动调用。
解决懒惰的最终方案是 `POST /v1/chat/completions` 代理（P 待实现）：
- evermemos-rs 提供 OpenAI 兼容的 chat 端点
- 每次请求自动 search_memory → 注入 system prompt → 转发给真实 LLM
- 模型完全看不到这一层，100% 可靠
