# CLAUDE.md

Refer to [AGENTS.md](AGENTS.md) for comprehensive project documentation including:
- Project architecture and structure
- Tech stack and dependencies
- Code conventions and patterns
- Key abstractions and files
- Development guidelines
- Database schema

## Working Mode (IMPORTANT)

This repo has **two implementations**:
- Python baseline: `src/`
- Rust rewrite: `evermemos-rs/`

Before coding, decide target system first and keep changes within that boundary.

## Quick Commands

### Python System (`src/`)

```bash
docker-compose up -d          # Start infrastructure
uv sync                       # Install dependencies
make run                      # Run application
pytest                        # Run tests
black src/ && isort src/      # Format code
pyright                       # Type check
```

### Rust System (`evermemos-rs/`)

```bash
cd evermemos-rs

cargo check                   # Fast compile check
cargo run --bin evermemos     # Run API server
cargo run --bin evermemos-mcp # Run MCP stdio server
cargo fmt                     # Format
cargo clippy -- -D warnings   # Lint
cargo test                    # Unit tests

# Optional feature modules
cargo check --features behavior-history
```

## Key Entry Points

### Python
- `src/run.py` - Application entry
- `src/agentic_layer/memory_manager.py` - Core memory manager
- `src/infra_layer/adapters/input/api/` - REST API controllers

### Rust
- `evermemos-rs/src/main.rs` - API server entry
- `evermemos-rs/src/agentic/manager.rs` - Retrieval orchestration
- `evermemos-rs/src/mcp/mod.rs` - MCP JSON-RPC tool server
- `evermemos-rs/src/storage/schema.rs` - SurrealDB schema

## Remember

- All I/O is async - use `await` / async equivalents
- Multi-tenant context must be preserved in API paths
- Prompt-driven extraction exists in both systems:
	- Python: `src/memory_layer/prompts/`
	- Rust: `evermemos-rs/src/memory/prompts/`
- If touching Rust feature-gated code, validate both builds:
	- `cargo check`
	- `cargo check --features behavior-history`
