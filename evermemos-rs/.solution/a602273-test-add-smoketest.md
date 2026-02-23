# a602273 — test: add smoketest

> **Branch**: `evermemos-rs`  
> **Date**: 2026-02-23  
> **Scope**: `demo/smoke_test.sh`, `demo/simple_demo.py`, `src/lib.rs`

---

## 背景

Rust 重写后缺少端对端验证手段。Python 原版有 `demo/simple_demo.py` 可手动执行，
但 Rust 版没有对应工具，也没有办法快速确认各个 API 接口是否联通。

---

## 为什么需要 smoke test

1. **回归保护**：每次修改 SurrealDB schema / repository 层后，可一键验证六个核心流程不退化。
2. **行为对齐基准**：与原版 `simple_demo.py` 使用相同的用户数据（soccer / Barcelona / Messi），
   便于对比两个版本的记忆提取结果。
3. **CI 入口**：shell 脚本返回非零退出码，可直接接入 GitHub Actions。

---

## 文件说明

### `demo/smoke_test.sh`

**设计原则**：只依赖 `bash` + `curl`，不依赖 Python / jq，开箱即用。

测试步骤（顺序）：

| Step | 接口 | 验证内容 |
|------|------|----------|
| 0 | `GET /health` | 服务存活 |
| 1 | `DELETE /api/v1/memories` | 清理上次测试数据 |
| 2 | `POST /api/v1/memories` × 7 | 消息写入成功（accumulating / extracted 均视为成功） |
| 3 | `sleep 12` | 等待后台异步 LLM extraction pipeline 完成 |
| 4 | `GET /api/v1/memories` | 检查 `"memories"` 字段存在（有没有值取决于 LLM 速率） |
| 5 | `GET /api/v1/memories/search?retrieve_method=KEYWORD` × 3 | BM25 接口不返回 500 |
| 6 | `GET /api/v1/memories/search?retrieve_method=VECTOR` | HNSW 接口返回合法 envelope |

**为什么 step 5 只验证接口存活而不验证结果数量**：
BM25 搜索结果依赖异步 LLM 提取完成后写入的 `episodic_memory` 记录，
7 条消息只触发一次 boundary（5 条时），提取耗时受 LLM 速率（40 req/min）影响不可预测。
因此断言"接口不报错"而非"返回 N 条结果"，避免 flaky test。

**为什么等 12 秒**：
boundary 触发后，`tokio::spawn` 进行 4 路并发提取（episode / foresight / event log / profile），
每路至少一个 LLM 调用（~1-2s），加上可能的速率限制回退，12s 足够覆盖正常场景。

---

### `demo/simple_demo.py`

Python 版本的端对端演示，镜像 `smoke_test.sh` 的步骤，使用 `httpx` 库。
用途：
- 在有 Python 环境时给开发者提供更友好的彩色输出和详细内存内容展示
- 作为 Rust 版与原版行为对比的基准脚本

与原版 `src/demo/simple_demo.py` 的区别：
- 原版直接调用 Python SDK；此版本通过 HTTP API 调用 Rust 服务
- 新增 hybrid search 测试（`RRF` 方法）

---

### `src/lib.rs`

将 `main.rs` 中的模块声明提取到 `lib.rs`，使单元测试可以不依赖 `main()` 入口编译整个 crate。

```rust
// 之前：所有 pub mod 在 main.rs 顶部
// 之后：src/lib.rs 持有模块树，main.rs 只保留 main()
```

这是 Rust 惯用的 binary crate 拆分模式，允许 `cargo test` 在不启动服务的情况下
独立运行各层的单元测试。

---

## 使用方式

```bash
# 终端 A：启动服务
cd evermemos-rs && cargo run --bin evermemos

# 终端 B：运行 smoke test
bash demo/smoke_test.sh

# 或 Python 版
pip install httpx
python demo/simple_demo.py
```

退出码：`0` = 全部通过，非零 = 有失败项。
