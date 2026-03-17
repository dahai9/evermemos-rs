# BehaviorHistory — 行为历史记录表 (Full Stack)

**Commit**: `488fec6`  
**Branch**: `evermemos-rs`  
**Date**: 2026-02-26  

## Summary

Implemented the full BehaviorHistory feature: Rust model → SurrealDB schema → repository CRUD → agentic wiring → REST API → embedded HTML management dashboard.

---

## Files Changed

### New Files

| File | Description |
|------|-------------|
| `src/storage/models/behavior_history.rs` | `BehaviorHistory` struct (user_id, timestamp, behavior_type Vec\<String\>, event_id, meta, extend, is_deleted, created_at, updated_at) + `primary_type()` helper |
| `src/storage/repository/behavior_history.rs` | CRUD: `insert`, `soft_delete`, `soft_delete_by_user`, `get_by_user_id`, `get_by_time_range`, `get_by_type`, `count_by_user` |
| `src/api/behavior_history_router.rs` | REST API router — 4 endpoints |
| `src/api/ui_router.rs` | Serves embedded HTML dashboard at `GET /ui/behavior-history` |
| `static/behavior_history.html` | Dark-theme management SPA (420 lines) |
| `docs/MCP_AGENT_RULES.md` | Agent system prompt guide (CN+EN) for Claude Desktop / Cursor |

### Modified Files

| File | Change |
|------|--------|
| `src/storage/schema.rs` | Added SCHEMAFULL DDL + 5 indexes for `behavior_history` table (no BM25/HNSW — matches Python behavior) |
| `src/storage/models/mod.rs` | `pub mod behavior_history` + `pub use behavior_history::BehaviorHistory` |
| `src/storage/repository/mod.rs` | `pub mod behavior_history` + `pub use behavior_history::BehaviorHistoryRepo` |
| `src/agentic/manager.rs` | `MemoryType::BehaviorHistory` variant, `bh_repo: BehaviorHistoryRepo` field, constructor param, keyword/vector search arms (time-sorted fallback), `bh_to_item()` converter |
| `src/api/dto.rs` | `"behavior_history" \| "BEHAVIOR" \| "BEHAVIOR_HISTORY" => MemoryType::BehaviorHistory` |
| `src/api/mod.rs` | `pub mod behavior_history_router`, `pub mod ui_router`, re-exports |
| `src/main.rs` | `bh_repo` wiring throughout + `ui_routes()` merged into axum router |

---

## REST API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/behavior-history` | List records (query: user_id, behavior_type, start_time, end_time, limit) |
| `POST` | `/api/v1/behavior-history` | Create record |
| `DELETE` | `/api/v1/behavior-history/{id}` | Soft-delete record |
| `GET` | `/api/v1/behavior-history/stats` | User stats + type breakdown |
| `GET` | `/ui/behavior-history` | Management dashboard HTML |

## Example

```bash
# Create
curl -X POST http://localhost:8080/api/v1/behavior-history \
  -H "Content-Type: application/json" \
  -d '{"user_id":"alice","behavior_type":["chat","follow-up"],"event_id":"conv_001","meta":{"score":5}}'

# List
curl "http://localhost:8080/api/v1/behavior-history?user_id=alice&limit=20"

# Stats
curl "http://localhost:8080/api/v1/behavior-history/stats?user_id=alice"
# → {"user_id":"alice","total_records":1,"type_breakdown":{"chat":1,"follow-up":1}}

# Delete
curl -X DELETE "http://localhost:8080/api/v1/behavior-history/b55227b3-fa25-4f54-839e-6c3f289a9997"

# Dashboard
open http://localhost:8080/ui/behavior-history
```

---

## Design Notes

- **No BM25/HNSW index**: Intentional. Python reference implementation also has no vector/BM25 index for BehaviorHistory — retrieval is always by `user_id` sorted by `timestamp` DESC. Keyword/vector search arms return time-sorted list via `get_by_user_id()`.
- **Soft delete**: `is_deleted = true` flag, queries always filter `is_deleted = false`.
- **HTML embedded via `include_str!`**: No runtime file I/O — the HTML is baked into the binary at compile time.
- **Axum route note**: Route param syntax is `{id}` (Axum 0.8), not `:id`.

---

## Smoke Test Results

```
✅ POST /api/v1/behavior-history     → 200 {"status":"success","result":{...}}
✅ GET  /api/v1/behavior-history     → 200 {"records":[...],"total":1}
✅ GET  /api/v1/behavior-history/stats → 200 {"total_records":1,"type_breakdown":{"chat":1,"follow-up":1}}
✅ GET  /ui/behavior-history         → 200 HTML dashboard
✅ cargo check                       → Finished (0 errors, 0 warnings)
```

---

## Priority Status

| Feature | Priority | Status |
|---------|----------|--------|
| BehaviorHistory | P1 | ✅ Done |
| Prometheus metrics | P2 | 🔴 Next |
| Rate Limiting | P3 | 🔴 Pending |
| `/v1/chat/completions` proxy | P4 | 🔴 Pending |
| Kafka Consumer | P5 | 🔴 Pending |
