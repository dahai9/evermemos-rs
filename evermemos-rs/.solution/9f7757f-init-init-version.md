# 9f7757f — init: init version

> **Branch**: `evermemos-rs`  
> **Date**: 2026-02-23  
> **Scope**: 67 files, 11 981 行新增 — EverMemOS 的完整 Rust 重写

---

## 背景

原版 EverMemOS 使用 Python + FastAPI + MongoDB + Elasticsearch + Milvus + Redis 五组件栈。
目标是用 **SurrealDB v2（嵌入式 RocksDB）+ NATS + moka** 将三个存储层（文档、向量、全文）合并为一个数据库，消除跨服务的网络 round-trip，以单二进制方式部署。

---

## 主要技术决策 & 为什么这样做

### 1. `TYPE any` 替代 `TYPE datetime` — 存储模型时间字段

**问题**：SurrealDB v2 SCHEMAFULL 模式下，`TYPE datetime` 字段会对插入值进行强校验。
Rust 端的数据结构使用 `chrono::DateTime<Utc>`，序列化后得到的是 JSON 字符串
（例如 `"2026-02-23T01:05:43.711303706Z"`），SurrealDB 不接受字符串作为 `datetime` 类型，
直接报错：

```
Found '2026-02-23T01:05:43.711303706Z' for field `created_at`, but expected a datetime
```

**选择方案**：将所有 datetime 字段改为 `TYPE any`，接受任何值（字符串 / 原生 datetime / 空值），
同时保留 `DEFAULT time::now()` 让数据库在未传入时自动填充。

```sql
-- Before
DEFINE FIELD IF NOT EXISTS created_at ON memcell TYPE datetime DEFAULT time::now();

-- After
DEFINE FIELD IF NOT EXISTS created_at ON memcell TYPE any DEFAULT time::now();
```

**影响文件**：`src/storage/schema.rs`（所有 8 张表 × 多个 datetime 字段）

**为什么不换 Rust 类型**：改用 `surrealdb::sql::Datetime` 需要修改所有 model 构造点、
extractor 构造点和所有测试，成本远高于在 schema 层放宽校验。

---

### 2. `create(("table", uuid))` 替代 `create("table")` — SCHEMAFULL 表插入

**问题**：SurrealDB v2 中，对 SCHEMAFULL 表调用 `db.create("table_name")` 不指定 ID，
SDK 内部会生成一个临时 record ID 但无法通过 schema 校验 —— 返回 `None` 或 silently 失败。

**解法**：在 Rust 端用 `uuid::Uuid::new_v4()` 预先生成 ID，传入 tuple 形式：

```rust
// Before
let created: Option<MemCell> = self.db.create("memcell").content(cell).await?;

// After
let rid = Uuid::new_v4().to_string();
let created: Option<MemCell> = self.db.create(("memcell", rid)).content(cell).await?;
```

**影响文件**：
- `src/storage/repository/memcell.rs`
- `src/storage/repository/episodic_memory.rs`
- `src/storage/repository/foresight.rs`
- `src/storage/repository/event_log.rs`
- `src/storage/repository/user_profile.rs`
- `src/storage/repository/cluster_state.rs`

---

### 3. BM25 `@0@` 运算符替代 `search::matches()` — 全文检索

**问题**：SurrealDB 文档中同时出现了 `search::matches(field, tokens)` 函数式写法和
`field @N@ tokens` 运算符写法，但当前部署的 v2 版本（`surrealdb = "2"`）仅支持后者，
调用 `search::matches()` 会直接返回 Parse error。

**解法**：改用 `@0@` 运算符，其中 `0` 是该字段上第一个（零索引）BM25 谓词的编号：

```sql
-- Before（报错）
WHERE search::matches(search_content, $tokens)

-- After（正确）
WHERE search_content @0@ $tokens
```

同时 `search::score(N)` 中的 `N` 也是零索引，需与 `@N@` 保持一致（均为 `0`）：

```sql
SELECT *, search::score(0) AS _score FROM episodic_memory
WHERE search_content @0@ $tokens
```

**影响文件**：
- `src/storage/repository/episodic_memory.rs`
- `src/storage/repository/foresight.rs`
- `src/storage/repository/event_log.rs`

---

### 4. 嵌入式模式跳过 `signin()` — SurrealDB 连接

**问题**：`db.signin(Root { username, password })` 仅对远程 server 模式有效。
对 `rocksdb://`、`mem://`、`file://` 等嵌入式 endpoint 调用 `signin()` 会直接报错并崩溃。

**解法**：连接后检测 endpoint 前缀，仅在非嵌入式时发起鉴权：

```rust
let is_embedded = cfg.endpoint.starts_with("rocksdb://")
    || cfg.endpoint.starts_with("mem://")
    || cfg.endpoint.starts_with("file://");

if !is_embedded && !cfg.user.is_empty() {
    db.signin(Root { username: &cfg.user, password: &cfg.pass }).await?;
}
```

**影响文件**：`src/storage/db.rs`

---

### 5. HNSW 语法 `DIMENSION`（单数）— 向量索引

**问题**：SurrealDB v2 使用 `HNSW DIMENSION N`（单数），但部分文档和早期版本使用
`HNSW DIMENSIONS N`（复数），后者在当前版本触发 parse error。

**解法**：

```sql
-- Wrong
DEFINE INDEX idx_ep_vec ON episodic_memory FIELDS vector
    HNSW DIMENSIONS 1024 DIST COSINE EFC 200 M 16;

-- Correct
DEFINE INDEX idx_ep_vec ON episodic_memory FIELDS vector
    HNSW DIMENSION 1024 DIST COSINE EFC 200 M 16;
```

---

### 6. 配置环境变量双下划线分隔符

**问题**：`config-rs` crate 默认使用 `__` 作为嵌套字段分隔符（对应 `.` ），
单下划线 `_` 是字段名的一部分。若 `.env` 中写 `LLM_BASE_URL`，config-rs 无法映射到
`config.llm.base_url`。

**解法**：`.env` / `.env.template` 统一使用双下划线：

```env
LLM__BASE_URL=http://127.0.0.1:4000
LLM__API_KEY=sk-...
LLM__MODEL=gpt-4.1
SURREAL__ENDPOINT=rocksdb://./data/surreal
```

---

## 架构映射（Python → Rust）

| Python 组件 | Rust 替代 |
|---|---|
| MongoDB | SurrealDB SCHEMAFULL tables |
| Elasticsearch | SurrealDB BM25 `SEARCH ANALYZER … BM25` |
| Milvus | SurrealDB HNSW `DIMENSION 1024 DIST COSINE` |
| Redis / mq buffer | `moka` 内存缓存（LRU）|
| Kafka / ARQ | NATS JetStream |
| FastAPI | Axum |
| Beanie ODM | surrealdb Rust SDK |
| LangChain | 自实现 `LlmProvider` trait |

---

## 验证

所有决策在开发过程中通过实际运行报错发现，逐一修复后 smoke test 全部通过：

```
PASS: 7   FAIL: 0
All tests passed!
```
