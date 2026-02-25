# 885ec2a — feat(evermemos-rs): GroupProfile extraction for group-chat scenes

## 限制
Group 场景下的群画像抽取（主题、摘要、定位），单次 LLM 调用，并发执行。

## 背景
Python 实现中，Group 场景会维护 `GroupProfile`（含话题列表、群摘要、群定位）。
Rust 侧之前只做了 Group Episode 抽取，缺少这个聚合画像层。需要补齐，使 Group 场景
的记忆管道与 Python baseline 对齐。

## 问题根因
Python `group_profile_memory_extractor.py` 结构复杂（topic_processor、role_processor、
data_processor、llm_handler 四个子模块），直接对译工作量巨大且当前无对应测试用例。
Rust 侧也没有 `group_profile` 表和对应 model/repo。

## 解法
采用简化单次 LLM 调用方案，而非照搬 Python 的多阶段增量更新：

1. **DDL**（`schema.rs`）：新增 `group_profile` 表，`group_id` 唯一索引，
   `topics` 字段存储 JSON 数组（`option<any>`），无需向量索引（非检索型数据）。

2. **Model**（`models/group_profile.rs`）：`GroupProfile` struct，`topics` 用
   `Option<Value>` 存储（避免复杂的嵌套泛型），`TopicInfo` 仅保留 name/summary/status。

3. **Repo**（`repository/group_profile.rs`）：`get_by_group_id` + `upsert`，
   upsert 逻辑与其他 repo 一致（查存在则 merge update，否则 create）。

4. **Prompts**（`prompts/en.rs`）：新增 `GROUP_PROFILE_SYSTEM` + `GROUP_PROFILE_USER`，
   参考 Python `CONTENT_ANALYSIS_PROMPT`，要求输出 `{topics, summary, subject}` JSON。
   支持中英文内容，status 枚举保持英文。

5. **Extractor**（`memory/group_profile_extractor.rs`）：单次 `complete_json`，
   将结果合并进 `GroupProfile`（保留 `created_at` from existing）。

6. **Manager**（`memory/manager.rs`）：新增 `group_profile_ex` + `gp_repo` 字段；
   `gp_fut` 在 `SceneType::Group` 时触发，与 `ep_fut` 并发执行（`tokio::join!`）。
   `MemoryManager::new` 新增 `gp_repo: GroupProfileRepo` 参数。

7. **main.rs / worker_main.rs**：实例化 `GroupProfileRepo` 并传入 `MemoryManager`。

关键 diff（manager.rs 核心新增）：
```rust
// before: tokio::join!(ep_fut, fs_fut, el_fut, profile_fut);

// after:
let gp_fut = async {
    if scene == SceneType::Group {
        if let (Some(gid), Some(gname)) = (group_id, group_name) {
            let existing = self.gp_repo.get_by_group_id(gid).await.ok().flatten();
            match self.group_profile_ex.extract(messages, gid, gname, existing.as_ref()).await {
                Ok(gp) => { let _ = self.gp_repo.upsert(gp).await; }
                Err(e) => warn!("GroupProfile extraction failed: {e}"),
            }
        }
    }
};
tokio::join!(ep_fut, fs_fut, el_fut, profile_fut, gp_fut);
```

## 影响文件

| 文件 | 改动 |
|------|------|
| `src/storage/schema.rs` | 新增 `group_profile` 表 DDL + UNIQUE 索引 |
| `src/storage/models/group_profile.rs` | 新文件：`GroupProfile` + `TopicInfo` |
| `src/storage/models/mod.rs` | 导出 `GroupProfile` |
| `src/storage/repository/group_profile.rs` | 新文件：`GroupProfileRepo` |
| `src/storage/repository/mod.rs` | 导出 `GroupProfileRepo` |
| `src/memory/group_profile_extractor.rs` | 新文件：`GroupProfileExtractor` |
| `src/memory/mod.rs` | 导出 `group_profile_extractor` |
| `src/memory/prompts/en.rs` | 新增 `GROUP_PROFILE_SYSTEM` / `GROUP_PROFILE_USER` |
| `src/memory/manager.rs` | 新增字段 + `gp_fut` + `new()` 参数 |
| `src/main.rs` | 实例化 `GroupProfileRepo`，传入 `MemoryManager` |
| `src/worker_main.rs` | 同上 |

## 验证

```
cargo build          # 0 errors, 0 warnings
uv run python evermemos-rs/demo/parity_test.py
# Parity: 21/21 (100%) — Full parity maintained
```
