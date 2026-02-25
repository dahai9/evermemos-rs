# dfdcdd9 — feat(evermemos-rs): PATCH conv-meta + global-user-profile/custom

## 限制
在保持 parity 21/21 的前提下，新增两个 Python 有、Rust 缺失的 API。

## 背景
上一轮进度回顾发现以下 gap 未实现：
1. `PATCH /api/v1/memories/conversation-meta` — 部分字段更新（Python 的 `patch_conversation_meta`）
2. `POST /api/v1/global-user-profile/custom` — 注入用户自定义画像（Python 的 `GlobalUserProfileController.upsert_custom_profile`）

## 问题根因
两个端点在 Rust 版中完全缺失：路由未注册、handler 未实现、相关 model/repo 方法不存在。

## 解法

### 1. 扩展 `ConversationMeta` 模型
**`storage/models/conversation_meta.rs`**：新增对齐 Python 的字段：
`name`, `description`, `scene`, `scene_desc`, `tags`, `user_details`, `default_timezone`，
同时新增内嵌 `UserDetail` struct。

### 2. `patch_by_group_id` 方法
**`storage/repository/conversation_meta.rs`**：
```rust
pub async fn patch_by_group_id(
    &self, group_id: &str, patch: Value,
) -> Result<Option<ConversationMeta>>
```
通过 `get_by_group_id` 查到现有记录，用 `.merge(patch)` 做部分更新；找不到时返回 `None`（对应 HTTP 404）。

### 3. 扩展 `UserProfile` 模型
**`storage/models/user_profile.rs`**：新增 `custom_profile_data: Option<Value>` 字段。

### 4. `upsert_custom_profile` 方法
**`storage/repository/user_profile.rs`**：
```rust
pub async fn upsert_custom_profile(
    &self, user_id: &str, custom_profile_data: Value,
) -> Result<UserProfile>
```
找到已有 profile → `merge` 更新 `custom_profile_data`；不存在 → 创建新记录。

### 5. 新增 DTO
**`api/dto.rs`** 追加：
- `ConversationMetaPatchBody` / `ConversationMetaPatchResponse`
- `CustomProfileData` / `UpsertCustomProfileRequest` / `UpsertCustomProfileResponse`

### 6. PATCH handler
**`api/memory_router.rs`**：
```
PATCH /api/v1/memories/conversation-meta
```
解析 `ConversationMetaPatchBody`，将非 None 字段收集为 `serde_json::Value` patch 对象，
调用 `conv_meta_repo.patch_by_group_id()`，返回 `updated_fields` 列表。
找不到 group_id 时返回 404。

### 7. `global_profile_router.rs`（新文件）
注册路由：
```
POST /api/v1/global-user-profile/custom
```
验证 `user_id` 和 `initial_profile` 非空，将 `CustomProfileData` 序列化后调用 `up_repo.upsert_custom_profile()`。

### 8. 集成
- **`api/mod.rs`**：`pub mod global_profile_router;` 导出
- **`main.rs`**：将 `global_profile_router()` merge 进 Axum Router，传入 `up_repo`

## 影响文件

| 文件 | 改动原因 |
|------|----------|
| `storage/models/conversation_meta.rs` | 新增 name/description/scene/tags 等字段 + UserDetail |
| `storage/models/user_profile.rs` | 新增 custom_profile_data 字段 |
| `storage/repository/conversation_meta.rs` | 新增 patch_by_group_id |
| `storage/repository/user_profile.rs` | 新增 upsert_custom_profile |
| `api/dto.rs` | 新增 PATCH + global-profile DTO |
| `api/memory_router.rs` | 新增 PATCH 路由 + handler |
| `api/global_profile_router.rs` | 新文件：POST /custom 路由 + handler |
| `api/mod.rs` | 导出 global_profile_router |
| `main.rs` | 集成 global_profile_router |

## 验证

```
# PATCH conv-meta
curl -X PATCH .../conversation-meta -d '{"group_id":"patch-grp","name":"New Name","tags":["updated"]}'
→ {"updated_fields":["name","tags"]}

# POST global-user-profile/custom
curl -X POST .../global-user-profile/custom -d '{"user_id":"u1","custom_profile_data":{"initial_profile":["User is a Rust developer"]}}'
→ {"success":true}

# 回归
parity  21/21 ✅  (100%)
completeness 19/19 ✅, 15/15 feature checks ✓ OK
```
