# AGENTS.md - AI Assistant Guide for EverMemOS

> This file provides project context for AI coding assistants (Claude Code, GitHub Copilot, Cursor, Codeium, etc.) to better understand and work with this repository.
>
> **Maintainer Note**: Keep this file updated when either Python (`src/`) or Rust (`evermemos-rs/`) structure changes.

## Project Summary

**EverMemOS** currently has **two active implementations** in this mono-repo:

| System | Location | Status | Primary Use |
|--------|----------|--------|-------------|
| Python EverMemOS | `src/` | Stable / feature-complete baseline | Production architecture reference |
| Rust evermemos-rs | `evermemos-rs/` | Active rewrite | Lightweight deployment, simpler infra |

## Architecture (Dual-System)

### Python Architecture (`src/`)

```
┌─────────────────────────────────────────────────────┐
│                  API Layer (FastAPI)                │
│            infra_layer/adapters/input/api/          │
├─────────────────────────────────────────────────────┤
│                  Service Layer                      │
│                     service/                        │
├─────────────────────────────────────────────────────┤
│                Business Logic Layer                 │
│                    biz_layer/                       │
├─────────────────────────────────────────────────────┤
│                  Agentic Layer                      │
│      (Memory Management, Vectorization, Retrieval)  │
│                  agentic_layer/                     │
├─────────────────────────────────────────────────────┤
│                  Memory Layer                       │
│        (MemCell, Episode, Profile Extraction)       │
│                  memory_layer/                      │
├─────────────────────────────────────────────────────┤
│                   Core Layer                        │
│      (DI, Middleware, Multi-tenancy, Cache)         │
│                      core/                          │
├─────────────────────────────────────────────────────┤
│               Infrastructure Layer                  │
│       (MongoDB, Milvus, Elasticsearch, Redis)       │
│          infra_layer/adapters/out/                  │
└─────────────────────────────────────────────────────┘
```

### Rust Architecture (`evermemos-rs/src/`)

```
┌─────────────────────────────────────────────────────┐
│                API Layer (Axum)                     │
│                    api/                             │
├─────────────────────────────────────────────────────┤
│              Agentic Retrieval Layer                │
│            (Keyword/Vector/Hybrid/RRF/Agentic)     │
│                   agentic/                          │
├─────────────────────────────────────────────────────┤
│               Memory Extraction Layer               │
│      (MemCell, Episode, Profile, Foresight, etc.)  │
│                    memory/                          │
├─────────────────────────────────────────────────────┤
│                 Storage Layer                       │
│      (SurrealDB schema + models + repositories)     │
│                    storage/                         │
├─────────────────────────────────────────────────────┤
│               Core Runtime Layer                    │
│   (error, tracing, telemetry, metrics, tenant)      │
│                     core/                           │
├─────────────────────────────────────────────────────┤
│             Protocol/Worker Layer                   │
│      (MCP stdio server, NATS worker)                │
│           mcp/, tasks/, worker_main.rs              │
└─────────────────────────────────────────────────────┘
```

## Directory Structure (Key Paths)

### Python System

```
EverMemOS/
├── src/
│   ├── run.py                    # Application entry point
│   ├── app.py                    # FastAPI app configuration
│   ├── base_app.py               # Base application setup
│   ├── bootstrap.py              # Bootstrap and initialization
│   ├── application_startup.py    # Startup hooks
│   ├── manage.py                 # Management commands
│   ├── run_memorize.py           # Batch memorization runner
│   ├── task.py                   # Task definitions
│   ├── addon.py                  # Plugin system
│   ├── project_meta.py           # Project metadata
│   │
│   ├── agentic_layer/            # Memory orchestration
│   │   ├── memory_manager.py     # Core memory manager
│   │   ├── vectorize_service.py  # Embedding service
│   │   ├── rerank_service.py     # Reranking service
│   │   ├── fetch_mem_service.py  # Memory retrieval
│   │   ├── agentic_utils.py      # Agentic utilities
│   │   ├── retrieval_utils.py    # Retrieval utilities
│   │   ├── metrics/              # Performance metrics
│   │   └── ...
│   │
│   ├── memory_layer/             # Memory extraction pipeline
│   │   ├── memory_manager.py     # Extraction coordinator
│   │   ├── constants.py          # Memory constants
│   │   ├── memcell_extractor/    # MemCell boundary extraction
│   │   ├── memory_extractor/     # Episode/Profile/Foresight/EventLog extractors
│   │   ├── cluster_manager/      # Memory clustering
│   │   ├── profile_manager/      # Profile aggregation
│   │   ├── llm/                  # LLM abstraction/providers
│   │   └── prompts/              # Prompt templates (en/zh)
│   │
│   ├── infra_layer/              # External adapters
│   │   ├── adapters/
│   │   │   ├── input/            # API, jobs, mcp, mq consumers
│   │   │   └── out/              # Persistence/search/event adapters
│   │   └── scripts/              # Infra scripts and migrations
│   │
│   ├── biz_layer/                # Business workflows
│   ├── service/                  # Service implementations
│   ├── core/                     # Framework infrastructure (DI/middleware/tenant/cache)
│   ├── api_specs/                # DTOs, memory models, enums
│   ├── common_utils/             # Shared utility library
│   ├── migrations/               # DB migrations
│   ├── config/                   # App config data (e.g. stopwords)
│   └── devops_scripts/           # DevOps helper scripts
│
├── tests/                        # Python test suite
├── demo/                         # Python demos and scripts
├── docs/                         # Global documentation
├── evaluation/                   # Evaluation framework
├── data/                         # Sample datasets
├── data_format/                  # Input format specs
└── figs/                         # Figures/images
```

### Rust System

```
EverMemOS/evermemos-rs/
├── Cargo.toml                    # Rust dependencies + features
├── Cargo.lock                    # Rust dependency lockfile
├── .env / .env.template          # Rust runtime configuration
├── justfile                      # Build/run/test task shortcuts
├── src/
│   ├── main.rs                   # HTTP server binary
│   ├── mcp_main.rs               # MCP stdio binary
│   ├── worker_main.rs            # Worker binary
│   ├── lib.rs                    # Module exports
│   │
│   ├── api/                      # Axum routers + DTO mapping
│   │   ├── memory_router.rs      # /api/v1/memories* routes
│   │   ├── global_profile_router.rs
│   │   ├── behavior_history_router.rs  # feature-gated
│   │   ├── health_router.rs
│   │   ├── ui_router.rs
│   │   ├── dto.rs
│   │   ├── middleware.rs
│   │   └── mod.rs
│   │
│   ├── agentic/                  # Retrieval manager + strategies
│   │   ├── manager.rs            # KEYWORD/VECTOR/HYBRID/RRF/AGENTIC orchestration
│   │   ├── retrieval_utils.rs
│   │   ├── prompts.rs
│   │   ├── strategies/
│   │   └── mod.rs
│   │
│   ├── memory/                   # Memory extraction pipeline
│   │   ├── manager.rs
│   │   ├── memcell_extractor.rs
│   │   ├── episode_extractor.rs
│   │   ├── profile_extractor.rs
│   │   ├── group_profile_extractor.rs
│   │   ├── foresight_extractor.rs
│   │   ├── event_log_extractor.rs
│   │   ├── cluster_manager.rs
│   │   ├── prompts/
│   │   └── mod.rs
│   │
│   ├── storage/                  # SurrealDB schema/models/repos
│   │   ├── db.rs                 # DB init/connect
│   │   ├── schema.rs             # SurrealDB DDL
│   │   ├── models/               # Domain data models
│   │   ├── repository/           # Data access repositories
│   │   └── mod.rs
│   │
│   ├── core/                     # Runtime foundation
│   │   ├── error.rs              # AppError definitions
│   │   ├── tracing.rs            # tracing bootstrap
│   │   ├── telemetry.rs          # OTEL init/shutdown guard
│   │   ├── metrics.rs            # HTTP metrics middleware
│   │   ├── tenant.rs             # tenant context
│   │   ├── cache.rs              # in-process cache
│   │   └── mod.rs
│   │
│   ├── llm/                      # LLM provider/vector/rerank adapters
│   │   ├── provider.rs
│   │   ├── openai.rs
│   │   ├── vectorize.rs
│   │   ├── rerank.rs
│   │   ├── cassette.rs
│   │   └── mod.rs
│   │
│   ├── mcp/                      # MCP JSON-RPC implementation
│   │   └── mod.rs
│   │
│   ├── tasks/                    # Background task workers
│   │   ├── nats_worker.rs
│   │   ├── task_types.rs
│   │   └── mod.rs
│   │
│   └── config.rs                 # Rust app config schema
│
├── docs/
│   ├── MCP_AGENT_RULES.md        # MCP behavior and integration rules
│   └── RUST_VS_PYTHON.md         # Rewrite parity/差异说明
├── static/                       # Embedded UI/static resources
├── data/                         # Local data folder (rocksdb files etc.)
├── demo/                         # Rust-side demo/parity scripts
└── docker-compose.otel.yaml      # Local observability stack
```

## Tech Stack

### Python System

| Category | Technology |
|----------|------------|
| Web Framework | FastAPI, Uvicorn |
| LLM Integration | LangChain, OpenAI, Anthropic, Google GenAI |
| Document Store | MongoDB with Beanie ODM |
| Vector Database | Milvus 2.5 |
| Full-text Search | Elasticsearch 8.x |
| Cache | Redis |
| Message Queue | Kafka, ARQ |
| Validation | Pydantic 2.x |
| Package Manager | uv |

### Rust System

| Category | Technology |
|----------|------------|
| Web Framework | Axum + Tower |
| Runtime | Tokio |
| Storage | SurrealDB (RocksDB backend) |
| Search | SurrealDB BM25 + HNSW |
| LLM Integration | async-openai (OpenAI-compatible) |
| Queue | NATS (async-nats) |
| Cache | moka (in-process) |
| Observability | tracing + OpenTelemetry (OTLP HTTP) |
| Package Manager | Cargo |

## Code Conventions

### Python (`src/`)
- **Formatter**: Black (line width 88)
- **Import Sorting**: isort
- **Type Checker**: Pyright
- **Pattern**: Async/await + repository/adapter pattern + DI

### Rust (`evermemos-rs/src/`)
- **Formatter**: rustfmt (`cargo fmt`)
- **Lint**: clippy (`cargo clippy -- -D warnings`)
- **Pattern**: async-first, explicit typed DTO/model, repository-based data access
- **Feature Flags**: optional modules gated in `Cargo.toml` features (e.g. `behavior-history`)

## Key Entry Points

### Python
- `src/run.py` - app entry
- `src/agentic_layer/memory_manager.py` - core memory orchestration
- `src/infra_layer/adapters/input/api/` - REST controllers

### Rust
- `evermemos-rs/src/main.rs` - API server entry
- `evermemos-rs/src/agentic/manager.rs` - retrieval orchestrator
- `evermemos-rs/src/mcp/mod.rs` - MCP protocol implementation
- `evermemos-rs/src/storage/schema.rs` - DB schema definition

## Memory Types (Cross-System Concept)

| Type | Python | Rust | Notes |
|------|:------:|:----:|-------|
| MemCell | ✅ | ✅ | Atomic conversational boundary unit |
| Episode | ✅ | ✅ | Episodic memory |
| Profile | ✅ | ✅ | User profile memory |
| GroupProfile | ✅ | ✅ | Group-level profile memory |
| Foresight | ✅ | ✅ | Predictive memory |
| EventLog | ✅ | ✅ | Atomic event facts |
| BehaviorHistory | ✅ | ✅* | Rust is feature-gated (`behavior-history`) |

## Retrieval Strategies

| Strategy | Python | Rust |
|----------|:------:|:----:|
| KEYWORD | ✅ | ✅ |
| VECTOR | ✅ | ✅ |
| HYBRID | ✅ | ✅ |
| RRF | ✅ | ✅ |
| AGENTIC | ✅ | ✅ |

## Database Schema

### Python System
- MongoDB documents: `src/infra_layer/adapters/out/persistence/document/memory/`
- Milvus collections: `src/infra_layer/adapters/out/search/milvus/memory/`
- Elasticsearch indices: `src/infra_layer/adapters/out/search/elasticsearch/memory/`

### Rust System
- Schema file: `evermemos-rs/src/storage/schema.rs`
- Physical engine: SurrealDB with RocksDB
- Tables include: `memcell`, `episodic_memory`, `foresight_record`, `event_log_record`, `user_profile`, `group_profile`, `memory_request_log`, and optional `behavior_history`

## Common Commands

### Python System

```bash
# Development
uv sync
make run
python src/run.py

# Testing
pytest
pytest tests/test_memory_layer/
pytest --cov=src

# Code quality
black src/
isort src/
pyright

# Infrastructure
docker-compose up -d
docker-compose down
```

### Rust System

```bash
cd evermemos-rs

# Build/check
cargo check
cargo build --bin evermemos
cargo fmt
cargo clippy -- -D warnings

# Run
cargo run --bin evermemos
cargo run --bin evermemos-mcp
just start
just stop

# Optional feature
cargo run --bin evermemos --features behavior-history
cargo check --features behavior-history
```

## Environment Variables

### Python (`.env` from `env.template`)
- `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GOOGLE_API_KEY`
- `MONGODB_URI`, `REDIS_URL`, `MILVUS_HOST`, `ELASTICSEARCH_URL`

### Rust (`evermemos-rs/.env`)
- `OPENAI_API_KEY`, `OPENAI_BASE_URL`, model/vector/rerank related keys
- `SURREAL_ENDPOINT` (defaults to local rocksdb)
- `NATS_URL` and worker-related keys
- MCP: `EVERMEMOS_BASE_URL`, `EVERMEMOS_GROUP_ID`, `EVERMEMOS_USER_ID`
- OTEL: `OTEL_ENABLED`, `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`

## Development Guidelines

### When modifying Python code
1. Stay under `src/` and mirror existing layer boundaries.
2. Keep async APIs and DI style consistent.
3. Update tests under `tests/` where relevant.

### When modifying Rust code
1. Keep changes under `evermemos-rs/` modular boundaries (`api/agentic/memory/storage/core`).
2. Prefer typed DTOs and repository methods over ad-hoc query strings in handlers.
3. If adding optional capability, prefer a Cargo feature (default off unless required).
4. Validate both default and feature-enabled builds when touching feature-gated code.

## Commit Workflow (evermemos-rs)

When committing code changes under `evermemos-rs/`, follow this process:

1. Commit code changes first:
  ```bash
  git add -A
  git commit -m "fix|feat|refactor: ..."
  ```
2. Create/update a solution note in `evermemos-rs/.solution/` for that commit.
3. Amend the solution note into the same commit (do not create a new commit just for docs):
  ```bash
  HASH=$(git rev-parse --short HEAD)
  MSG=$(git log -1 --format="%s" | tr ' :' '--' | tr '[:upper:]' '[:lower:]')
  touch evermemos-rs/.solution/${HASH}-${MSG}.md
  git add evermemos-rs/.solution/
  git commit --amend --no-edit
  ```

### `.solution` naming and required content

- File naming: `.solution/{short_hash}-{commit_message_slugified}.md`
- Include sections: background, root cause (if bug fix), solution, impacted files, validation.

### Recursion guard

- ✅ Correct: `code commit` → `write .solution doc` → `git commit --amend --no-edit`
- ❌ Avoid: separate `docs:` commit only for `.solution` notes (causes recursive bookkeeping)
- Pure documentation commits (README/CHANGELOG/.solution README补档) can skip `.solution` note.

## Important Considerations

1. **Dual-system repo**: confirm target system (`src/` vs `evermemos-rs/`) before coding.
2. **Feature parity work**: Rust rewrite may intentionally diverge in infra implementation.
3. **Async first**: both systems are async-heavy and I/O-bound.
4. **Prompt consistency**: memory extraction behavior depends on prompt templates in each system.
5. **Tenant/context safety**: preserve request context handling in API/middleware paths.

## Documentation References

### Python docs
- `docs/ARCHITECTURE.md`
- `docs/installation/SETUP.md`
- `docs/api_docs/memory_api.md`
- `docs/dev_docs/development_guide.md`

### Rust docs
- `evermemos-rs/docs/RUST_VS_PYTHON.md`
- `evermemos-rs/docs/MCP_AGENT_RULES.md`

## Testing Approach

### Python
- Unit tests in `tests/` mirroring `src/`
- `pytest-asyncio` for async coverage

### Rust
- `cargo test` for Rust unit/integration tests
- `just test-completeness` / `just test-parity` for parity checks
- For feature-gated modules, compile both:
  - `cargo check`
  - `cargo check --features behavior-history`
