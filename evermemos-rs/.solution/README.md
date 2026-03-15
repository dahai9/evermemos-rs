# .solution/ — 版本改动回溯文档

每次向 `evermemos-rs` 分支提交后，在此目录创建对应文档。

## 命名规则

```
.solution/{short_hash}-{commit_message_slugified}.md
```

- `short_hash`：`git rev-parse --short HEAD`（7位）
- `commit_message_slugified`：commit message 转小写，空格和冒号替换为 `-`

示例：commit `a602273 test: add smoketest` → `a602273-test-add-smoketest.md`

## 文档必须包含的内容

```markdown
# {hash} — {commit message}

## 限制
commit message 简短

## 背景
为什么需要这次改动？解决什么问题？

## 问题根因
（如果是 bug fix）具体报错或行为异常是什么？为什么会出现？

## 解法
改了什么？为什么选择这种改法而不是其他方案？
给出关键的 before/after diff 片段。

## 影响文件
列出修改的文件和改动原因。

## 验证
如何确认改动正确？跑了什么测试？
```

## 创建流程

```bash
# 1. 完成改动并提交
git add -A && git commit -m "fix: xxx"

# 2. 获取 hash 和 message
HASH=$(git rev-parse --short HEAD)
MSG=$(git log -1 --format="%s" | tr ' :' '--' | tr '[:upper:]' '[:lower:]')

# 3. 创建文档（由 AI 填写内容）
touch .solution/${HASH}-${MSG}.md

# 4. 把文档 amend 进同一个 commit —— 不产生新 commit
git add .solution/ && git commit --amend --no-edit
```

## 重要约束：避免递归

solution doc **通过 `--amend` 合并进它所记录的 commit**，永远不单独产生新的提交。

- ✅ 做法：`commit` → 写 `.solution/HASH.md` → `commit --amend --no-edit`
- ❌ 禁止：用独立的 `docs:` commit 提交 solution doc（会触发对 docs commit 本身的记录需求，无限循环）

**纯文档类 commit（如 README、CHANGELOG、本目录的补档）不创建 solution doc。**

## 现有文档

| 文件 | 内容摘要 |
|------|----------|
| [9f7757f-init-init-version.md](./9f7757f-init-init-version.md) | Rust 重写全量初始化；6 个 SurrealDB runtime 坑的解法 |
| [a602273-test-add-smoketest.md](./a602273-test-add-smoketest.md) | smoke_test.sh + simple_demo.py + lib.rs 拆分 |
| [6743f7c-fix-evermemos-rs---search-sql-alias--extraction-original_data--test-history.md](./6743f7c-fix-evermemos-rs---search-sql-alias--extraction-original_data--test-history.md) | SurrealDB SQL alias 修复；original_data CBOR 保留；测试 history 累积 |
| [45fe47b-feat-evermemos-rs---auto-start-server-in-test-scripts.md](./45fe47b-feat-evermemos-rs---auto-start-server-in-test-scripts.md) | 新增 server_utils.py；测试脚本自动启动/停止 Rust 服务，一条命令搞定 |
| [0af07e8-feat-evermemos-rs--full-parity-21-21.md](./0af07e8-feat-evermemos-rs--full-parity-21-21.md) | conversation-meta/status/profile 三端点；BM25 查询词修复；seed assistant-mode；parity 21/21 |
| [dfdcdd9-feat-evermemos-rs--patch-conv-meta---global-user-profile-custom.md](./dfdcdd9-feat-evermemos-rs--patch-conv-meta---global-user-profile-custom.md) | PATCH conversation-meta（部分更新）；POST global-user-profile/custom（注入初始画像）；新增 UserDetail/custom_profile_data 字段 |
| [885ec2a-feat-evermemos-rs---groupprofile-extraction-for-group-chat-scenes.md](./885ec2a-feat-evermemos-rs---groupprofile-extraction-for-group-chat-scenes.md) | Group 场景 GroupProfile 抽取（topics/summary/subject）；单次 LLM 调用并发执行；group_profile 表 DDL |
| [7d7f31c-feat-evermemos-rs---corememory-type---fix-group-profile-schema.md](./7d7f31c-feat-evermemos-rs---corememory-type---fix-group-profile-schema.md) | 新增 CoreMemory 枚举类型（复用 user_profile 表，memory_type="core_memory"）；修复 group_profile DDL option\<any> → option\<array> |
| [948811f-mcp-server.md](./948811f-mcp-server.md) | MCP server 二进制（stdio JSON-RPC 2.0）；4 个工具：search_memory/add_memory/add_conversation/get_profile；Claude Desktop / Cursor 配置说明 |
| [215074d-feat-evermemos-rs--behavior-history-model-schema-repo-api-management-ui.md](./215074d-feat-evermemos-rs--behavior-history-model-schema-repo-api-management-ui.md) | BehaviorHistory 全栈实现：model/schema/repo/API (GET/POST/DELETE/stats) + 暗黑主题管理 SPA；无BM25/HNSW索引（与Python一致）；`GET /ui/behavior-history` 内嵌HTML |
| [ed65938-feat-evermemos-rs--opentelemetry-observability--traces---metrics-.md](./ed65938-feat-evermemos-rs--opentelemetry-observability--traces---metrics-.md) | OpenTelemetry 可观测性：OTLP HTTP/protobuf export；TelemetryGuard 优雅 flush；http.server.requests Counter + http.server.request.duration Histogram；docker-compose.otel.yaml (grafana/otel-lgtm 单容器) |
| [e4c7937-feat--implement-memory-metabolism-and-profile-update-conflict-resolution.md](./e4c7937-feat--implement-memory-metabolism-and-profile-update-conflict-resolution.md) | Memory Metabolism (冲突解决): 更新 `user_profile` schemas 将 objects 标记为 `FLEXIBLE` 避免被 SurrealDB `SCHEMAFULL` 过滤；重写了基于 JSON 的 MERGE 更新解决 untyped 序列化问题；加入初始的 spatiotemporal graph nodes。 |
