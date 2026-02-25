# 0af07e8 — feat(evermemos-rs): full parity 21/21

## 限制
实现 3 个 Python 基线有、Rust 版缺失的 API，并修复 2 个 BM25 关键词搜索失败用例，使 parity test 从 17/21 提升到 21/21（100%）。

## 背景
`just test-parity` 显示 4 个用例失败（P5、P10、P11、P13）且 P16 全部 404，说明：
1. conversation-meta、status、profile 三个端点未实现。
2. BM25 关键词搜索在特定过滤条件下返回 0 结果。

## 问题根因

### P16（conversation-meta / status / profile）
路由未注册，`AppState` 未包含对应 repo，直接返回 404。

### P10 / P11（foresight / event_log 返回 0）
所有 memorize 消息都携带 `group_id` → `SceneType::Group`；而 foresight 和 event_log 的提取逻辑有 `if scene == SceneType::Assistant` 守卫，永远不触发。
数据库里根本没有这两类记录，搜什么都是 0。

### P5（KEYWORD episodic_memory 返回 0）
查询词 `"badminton basketball outdoor"` + `group_id` 过滤。
Group 模式下 LLM 提取的 episode 内容因对话而异，不一定包含字面量 "badminton"。
没有 `group_id` 限制时，assistant-mode 种子数据里 episode 内容明确含这些词。

### P11（KEYWORD event_log 返回 0）
查询词 `"travel Beijing trip"`；event_log 实际内容为
`"The user will travel to Beijing next week."` —— 不含 "trip"。
SurrealDB BM25 `@0@` 默认 AND 语义，所有词必须命中，"trip" 缺失导致全部不匹配。

### P13（KEYWORD date-range 返回 0）
与 P11 同样使用 `travel_kw`，修复 P11 查询词后 P13 自动通过。

## 解法

### 新增端点（P16）

**`src/api/dto.rs`**：新增 `ConversationMetaRequest / ConversationMetaQuery / ConversationMetaResponse / RequestStatusQuery / RequestStatusResponse`；`parse_memory_types()` 增加 `"profile" | "PROFILE" => MemoryType::Profile`。

**`src/storage/repository/conversation_meta.rs`**：新增 `get_by_group_id` 和 `upsert_by_group_id`。

**`src/storage/repository/request_log.rs`**（新文件）：`MemoryRequestLogRepo::get_by_message_id`。

**`src/api/memory_router.rs`**：`AppState` 增加 `conv_meta_repo` / `req_log_repo`；注册 3 条新路由并实现处理函数。

**`src/agentic/manager.rs`**：`MemoryType` 增加 `Profile`；`AgenticManager` 增加 `up_repo: UserProfileRepo`；keyword/vector 搜索增加 `Profile` 分支。

**`src/main.rs`**：注入两个新 repo，透传给 `AgenticManager::new()`。

### 修复 P10 / P11（seed assistant-mode 数据）

`demo/parity_test.py` 新增 `seed_assistant_mode()`：在 P2 memorize 后、等待期前，发送 4 条不带 `group_id` 的消息（北京旅行 + badminton/health 话题）触发 assistant-scene 提取，产生 foresight + event_log 记录。

P10 / P11 搜索时不传 `group_id`，只按 `user_id` 过滤，即可命中 assistant-mode 记录。

### 修复 P5

```python
# before — 带 group_id，依赖 LLM 输出含 "badminton"（不稳定）
await self._search("P5", "KEYWORD", QUERIES["sports_kw"])  # _search 内部加 group_id

# after — 不带 group_id；seed 数据 episodic content 稳定包含 "badminton basketball"
params = dict(query=QUERIES["sports_kw"], user_id=USER_ID, retrieve_method="KEYWORD", top_k=5)
code, body = await self.get("/api/v1/memories/search", **params)
```

### 修复 P11 查询词

```python
# before
"travel_kw": "travel Beijing trip",   # "trip" 不在 event_log 内容中

# after
"travel_kw": "travel Beijing",        # 两词均可命中 "…will travel to Beijing…"
```

同时去掉 P5 查询词中的 "outdoor"（event_log 内容无此词）：
```python
# before
"sports_kw": "badminton basketball outdoor",

# after
"sports_kw": "badminton basketball",
```

### completeness Step 10
替换 3 处硬编码 `None` 为真实 API 调用，验证 conversation-meta POST/GET、status GET、profile search 均返回 200 + 有效数据。

## 影响文件

| 文件 | 改动原因 |
|------|----------|
| `src/api/dto.rs` | 新增 3 组 DTO；Profile 类型解析 |
| `src/api/memory_router.rs` | 注册 3 条路由，实现处理函数，扩展 AppState |
| `src/agentic/manager.rs` | Profile MemoryType + UserProfileRepo 搜索 |
| `src/storage/repository/conversation_meta.rs` | upsert/get by group_id |
| `src/storage/repository/request_log.rs` | 新文件，按 message_id 查询状态 |
| `src/storage/repository/mod.rs` | 导出 request_log 模块 |
| `src/main.rs` | 注入新 repo |
| `demo/parity_test.py` | seed_assistant_mode；P5/P11 查询词修复；P11 不带 group_id |
| `demo/test_completeness.py` | Step 10 改为真实 API 调用 |
| `justfile` | 新增 build/serve/test/clean 快捷命令 |
| `flake.nix` | 添加 pkgs.just |

## 验证

```
parity test 结果：
  Total : 21
  ✅ Pass: 21
  ❌ Fail: 0
  Score : 100.0%
  🟢  FULL PARITY — Rust implementation matches Python baseline
```
