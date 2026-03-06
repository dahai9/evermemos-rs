# File Guide (evermemos-rs)

This guide explains what each major folder/file is for, and where to edit for common tasks.

## Top-Level Layout

- `Cargo.toml`: dependencies, binaries, features, release profile
- `.env.template`: environment variable template
- `justfile`: common build/run/test/dev recipes
- `docker-compose.otel.yaml`: local observability stack (collector + backends)
- `src/`: Rust source code
- `docs/`: project documentation
- `static/`: embedded UI/static pages
- `data/`: local runtime data (including embedded SurrealDB files)
- `demo/`: Python scripts for parity/completeness validation

## `src/` Module Map

- `main.rs`: API server bootstrap and middleware wiring
- `mcp_main.rs`: MCP stdio binary entry
- `worker_main.rs`: standalone worker entry
- `lib.rs`: crate exports

### API layer (`src/api/`)

- `memory_router.rs`: `/api/v1/memories*`
- `global_profile_router.rs`: global profile endpoints
- `behavior_history_router.rs`: behavior-history endpoints (feature-gated)
- `health_router.rs`: health endpoint
- `ui_router.rs`: static UI routes (behavior-history page also feature-gated)
- `dto.rs`: API request query/body mapping
- `middleware.rs`: API middleware helpers

### Agentic layer (`src/agentic/`)

- `manager.rs`: retrieval strategy orchestration (`KEYWORD/VECTOR/HYBRID/RRF/AGENTIC`)
- `strategies/`: strategy implementations
- `retrieval_utils.rs`: retrieval helpers
- `prompts.rs`: strategy prompts

### Memory extraction (`src/memory/`)

- `manager.rs`: extraction pipeline coordinator
- `memcell_extractor.rs`: conversation boundary extraction
- `episode_extractor.rs`, `profile_extractor.rs`, `group_profile_extractor.rs`
- `foresight_extractor.rs`, `event_log_extractor.rs`
- `cluster_manager.rs`: memory clustering
- `prompts/`: extraction prompts (en/zh)

### Storage (`src/storage/`)

- `db.rs`: SurrealDB initialization
- `schema.rs`: schema/table/index setup
- `models/`: domain data models
- `repository/`: data access repositories

### Runtime core (`src/core/`)

- `error.rs`: unified app errors
- `tracing.rs`: tracing + OTEL init
- `metrics.rs`: HTTP metrics middleware
- `tenant.rs`: tenant context model
- `cache.rs`: in-process cache wrappers

### LLM adapters (`src/llm/`)

- `provider.rs`: LLM trait
- `openai.rs`: OpenAI-compatible implementation
- `vectorize.rs`: embeddings
- `rerank.rs`: rerank integration
- `cassette.rs`: record/replay logic

### Async / workers (`src/tasks/`)

- `nats_worker.rs`: NATS consume loop
- `task_types.rs`: task contracts

### MCP (`src/mcp/`)

- `mod.rs`: JSON-RPC handling, tool list/dispatch, API bridge

## Feature Gate

- Cargo feature: `behavior-history`
- Default: disabled (`default = []`)
- Affects API routes, DTO handling, agentic integration, MCP behavior tools

## Where to Change (Common Tasks)

- Add new API endpoint: `src/api/*_router.rs` + `src/main.rs` route wiring
- Add new memory type: `src/memory/` + `src/storage/models` + `src/storage/repository` + agentic manager
- Add MCP tool: `src/mcp/mod.rs` (`tools_list` + `dispatch_tool`)
- Add config key: `src/config.rs` + `.env.template` + `docs/CONFIG_REFERENCE.md`
- Add observability metric: `src/core/metrics.rs` / `src/core/tracing.rs`

## Runbook-Adjacent Files

- Runtime logs: `/tmp/evermemos.log` (when using `just start`)
- PID marker: `/tmp/evermemos.pid`
- Surreal embedded data: `evermemos-rs/data/surreal`
