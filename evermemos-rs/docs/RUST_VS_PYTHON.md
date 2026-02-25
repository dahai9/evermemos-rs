# evermemos-rs vs Python EverMemOS — 功能对比

> 更新日期：2026-02-25  
> Rust HEAD：`76c8747`  
> 基准：Python `src/` 目录（`main` 分支）

---

## 代码规模

| 指标 | Python (`src/`) | Rust (`evermemos-rs/src/`) | 比例 |
|------|----------------|---------------------------|------|
| 文件数 | 434 | 65 | 15% |
| 代码行数 | 87,323 | ~7,000 | **8%** |

Rust 用 **8% 的代码量**覆盖了核心业务功能，体现了类型系统和零抽象成本带来的表达效率。

---

## 记忆提取层

| 功能 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| MemCell 边界检测 | ✅ | ✅ | 基于 LLM 的对话分段 |
| 剧集记忆 (EpisodicMemory) | ✅ | ✅ | 个人 & 群组双场景 |
| 用户画像 (UserProfile) | ✅ | ✅ | 含 LifeSummary 双阶段提取 |
| 预见记忆 (Foresight) | ✅ | ✅ | 仅 Assistant 场景 |
| 事件日志 (EventLog) | ✅ | ✅ | 仅 Assistant 场景 |
| 群组画像 (GroupProfile) | ✅ | ✅ | 群聊场景并发提取 |
| 行为历史 (BehaviorHistory) | ✅ | ❌ | 待实现 |

---

## 检索层

| 策略 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| Keyword (BM25) | ✅ | ✅ | SurrealDB 内置全文索引 |
| Vector (HNSW) | ✅ | ✅ | SurrealDB 内置向量索引 |
| Hybrid | ✅ | ✅ | Keyword + Vector 并行 |
| RRF (Reciprocal Rank Fusion) | ✅ | ✅ | |
| Agentic (多轮 LLM 引导检索) | ✅ | ✅ | 含充分性判断 |
| Rerank (Cross-encoder) | ✅ | ✅ | 可选，OpenAI-compatible |

---

## 记忆类型（检索侧）

| 类型 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| `episodic_memory` | ✅ | ✅ | |
| `foresight_record` | ✅ | ✅ | |
| `event_log_record` | ✅ | ✅ | |
| `profile` | ✅ | ✅ | |
| `core_memory` | ✅ | ✅ | 复用 user_profile 表 |
| `group_profile` | ✅ | — | 写入有，检索未暴露 |
| `entity` / `relationship` | 占位(未实现) | — | Python 侧亦未落地 |

---

## API 端点

| 端点 | 方法 | Python | Rust | 备注 |
|------|------|:------:|:----:|------|
| `/api/v1/memories` | POST | ✅ | ✅ | 单条消息写入 |
| `/api/v1/memories` | GET | ✅ | ✅ | 按 user_id/group_id 拉取 |
| `/api/v1/memories` | DELETE | ✅ | ✅ | 软删除，支持多条件 |
| `/api/v1/memories/search` | GET | ✅ | ✅ | 全策略检索 |
| `/api/v1/memories/conversation-meta` | GET | ✅ | ✅ | |
| `/api/v1/memories/conversation-meta` | POST | ✅ | ✅ | |
| `/api/v1/memories/conversation-meta` | PATCH | ✅ | ✅ | |
| `/api/v1/memories/status` | GET | ✅ | ✅ | 请求状态查询 |
| `/api/v1/global-user-profile` | GET | ✅ | ✅ | |
| `/api/v1/global-user-profile/custom` | POST | ✅ | ✅ | |
| `/health` | GET | ✅ | ✅ | |

---

## 消息队列 / 异步任务

| 功能 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| Kafka Consumer | ✅ | ❌ | Python 用 aiokafka |
| Kafka Producer | ✅ | ❌ | |
| NATS Consumer | ❌ | ✅ | Rust 用 NATS 替代 |
| ARQ (Redis 任务队列) | ✅ | ❌ | |

> Rust 选择 NATS 代替 Kafka，在嵌入式/移动场景更轻量。

---

## 存储层

| 组件 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| 文档存储 | MongoDB + Beanie ODM | SurrealDB (RocksDB) | Rust 嵌入式，零外部依赖 |
| 向量搜索 | Milvus 2.5 | SurrealDB HNSW | 已内置 |
| 全文搜索 | Elasticsearch 8.x | SurrealDB BM25 | 已内置 |
| 缓存 | Redis | Moka (in-process) | |
| 分布式锁 | Redis Redlock | ❌ | Rust 待实现 |

---

## 生产基础设施

| 功能 | Python | Rust | 备注 |
|------|:------:|:----:|------|
| Prometheus 监控 | ✅ | ❌ | 待实现 |
| Rate Limiting | ✅ | ❌ | 待实现 |
| 授权层 (Auth) | ✅ | ❌ | 待实现 |
| 多租户隔离 | ✅ | 部分 | Rust 有 tenant context，未完整隔离 |
| MCP 协议适配器 | ✅ stub | ❌ | 待实现 |
| 插件系统 (Addon) | ✅ | ❌ | Python 专有架构 |
| 国际化 / i18n | ✅ | ✅ | Rust 有 en/zh 双语 prompt |

---

## Rust 独有功能

| 功能 | 说明 |
|------|------|
| **LLM Cassette 录制/回放** | `LLM_CASSETTE_MODE=record/replay/auto`，开发测试零 token 消耗 |
| **零外部依赖部署** | SurrealDB 嵌入式替代 MongoDB + Elasticsearch + Milvus 三组件 |
| **嵌入式友好** | 适合移动端 / 边缘设备，二进制体积最小化 (`opt-level=z`) |

---

## Parity 测试结果

```
Parity Test Results — evermemos-rs vs Python baseline
══════════════════════════════════════════════════════
  Total : 21
  ✅ Pass: 21
  ❌ Fail: 0
  Score : 100.0%
  Core pipeline (P1–P15) : 17/17  (100%)
  Extended features (P16): 4/4    (100%)
🟢  FULL PARITY
```

---

## 总体评估

| 维度 | 评级 | 说明 |
|------|------|------|
| **核心记忆管道** | 🟢 可比肩 | 提取 + 检索 + API 全覆盖，parity 21/21 |
| **生产基础设施** | 🟡 差距明显 | 缺 Prometheus / RateLimit / Auth / 多租户 |
| **运维复杂度** | 🟢 Rust 更优 | 单二进制，无外部数据库依赖 |
| **开发工具链** | 🟢 Rust 更优 | Cassette 录播，Python 侧无此功能 |
| **功能完整性** | 🟡 约 85% | BehaviorHistory / Kafka / MCP / Prometheus 待补 |

---

## 待实现优先级

| 优先级 | 功能 | 工作量 |
|--------|------|--------|
| P1 | BehaviorHistory 记忆类型 | 小 |
| P2 | Prometheus metrics 中间件 | 中 |
| P3 | Rate Limiting 中间件 | 中 |
| P4 | MCP 协议适配器 | 中 |
| P5 | Kafka Consumer | 大（或维持 NATS 替代方案） |
| P6 | 多租户完整隔离 | 大 |
