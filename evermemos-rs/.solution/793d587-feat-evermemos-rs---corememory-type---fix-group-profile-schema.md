# 7d7f31c — feat(evermemos-rs): CoreMemory type + fix group_profile schema

## 限制
新增 `core_memory` 内存类型；修复 group_profile 表字段类型兼容问题。

## 背景
Python 实现中存在独立的 `core_memory` collection（`core_memories` MongoDB 表），
存储用户最高优先级的结构化画像（技能、OKR、性格、目标等）。
Rust 侧已有 `user_profile` 表存储同类数据，但 `MemoryType` 枚举和搜索 API 缺少
`core_memory` 类型，调用方无法按此类型过滤。同时发现上一个 commit 的 group_profile
DDL 使用了 `option<any>` 在 SurrealDB 中不合法，导致服务无法启动。

## 问题根因

**CoreMemory 缺失**：`MemoryType` 枚举仅有 `Profile`，没有 `CoreMemory`；
`parse_memory_types()` 不识别 `core_memory` 字符串；两个搜索分支（keyword/vector）
也没有对应处理。

**schema 报错**：
```
Parse error: Unexpected token `ANY`, expected a kind name
--> [183:69]
DEFINE FIELD ... TYPE option<any>;
```
SurrealDB 不支持 `option<any>` 写法，需改为 `option<array>`。

## 解法

### CoreMemory（`agentic/manager.rs`）
新增枚举变体并在两个搜索路径中各加一个 arm：

```rust
// before
pub enum MemoryType { EpisodicMemory, ForesightRecord, EventLogRecord, Profile, All }

// after
pub enum MemoryType { EpisodicMemory, ForesightRecord, EventLogRecord, Profile, CoreMemory, All }
```

新增独立转换函数 `cm_to_item()`，与 `up_to_item()` 数据来源相同（`user_profile` 表），
但 `memory_type` 字段返回 `"core_memory"`，metadata 增加 `"is_latest": true`，
精确对应 Python 侧语义。

### 字符串解析（`api/dto.rs`）
```rust
"core_memory" | "CORE_MEMORY" | "CORE" => MemoryType::CoreMemory,
```

### Schema 修复（`storage/schema.rs`）
```sql
-- before (invalid)
DEFINE FIELD IF NOT EXISTS topics ON group_profile TYPE option<any>;

-- after
DEFINE FIELD IF NOT EXISTS topics ON group_profile TYPE option<array>;
```

## 影响文件

| 文件 | 改动 |
|------|------|
| `src/agentic/manager.rs` | 新增 `CoreMemory` 枚举变体；`keyword_search` + `vector_search` 各加一个 arm；新增 `cm_to_item()` |
| `src/api/dto.rs` | `parse_memory_types()` 新增 `core_memory` 匹配分支 |
| `src/storage/schema.rs` | `group_profile.topics` 从 `option<any>` 改为 `option<array>` |

## 验证

```
cargo build          # 0 errors
just restart         # 服务正常启动（schema DDL 无报错）
just test-parity     # 21/21 (100%) ✅
curl ".../search?memory_types=core_memory"  # HTTP 200，结构正确
```
