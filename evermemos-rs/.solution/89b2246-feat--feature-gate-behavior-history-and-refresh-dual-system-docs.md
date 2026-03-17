## Background
Implemented BehaviorHistory completion and feature-gating in Rust rewrite, plus dual-system documentation refresh for Python and Rust structures.

## Root Cause
BehaviorHistory support in MCP and related wiring was incomplete, and feature-flag behavior needed to be default-off. Documentation also needed synchronized dual-system guidance.

## Solution
- Added/finished BehaviorHistory MCP tool handling and dispatch paths.
- Introduced `behavior-history` Cargo feature with default disabled behavior.
- Applied conditional compilation across affected API/storage/agentic/main/MCP/UI modules.
- Updated AGENTS/CLAUDE docs to reflect dual-system architecture and workflow guidance.

## Impacted Files
- AGENTS.md
- CLAUDE.md
- evermemos-rs/Cargo.toml
- evermemos-rs/src/agentic/manager.rs
- evermemos-rs/src/api/dto.rs
- evermemos-rs/src/api/mod.rs
- evermemos-rs/src/api/ui_router.rs
- evermemos-rs/src/main.rs
- evermemos-rs/src/mcp/mod.rs
- evermemos-rs/src/mcp_main.rs
- evermemos-rs/src/storage/models/mod.rs
- evermemos-rs/src/storage/repository/mod.rs

## Validation
- `cargo check` (default features) passed.
- `cargo check --features behavior-history` passed.
