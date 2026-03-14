# evermemos-rs

Rust rewrite of EverMemOS for lightweight deployment and simpler infrastructure.

`evermemos-rs` provides:
- HTTP API server (`evermemos`)
- MCP stdio server (`evermemos-mcp`)
- optional worker process (`evermemos-worker`)

## Install as Binaries (`cargo install`)

Install server + MCP binaries from local source:

```bash
cd evermemos-rs
cargo install --path . --bin evermemos --bin evermemos-mcp
```

After install, commands are available in Cargo bin path (usually `~/.cargo/bin`):

```bash
evermemos --help
evermemos-mcp --help
```

Install with BehaviorHistory feature enabled:

```bash
cargo install --path . --features behavior-history --bin evermemos --bin evermemos-mcp
```

You can also use:

```bash
just install
just install-behavior-history
```

## Quick Start

### 1) Prepare environment

```bash
cd evermemos-rs
cp .env.template .env
# edit .env and fill API keys / endpoints
```

### 2) Run API server

```bash
cargo run --bin evermemos
# or
just serve
```

Health check:

```bash
curl http://127.0.0.1:8080/health
```

### 3) Optional feature: BehaviorHistory

Default build does not enable BehaviorHistory routes/tools.

```bash
cargo run --bin evermemos --features behavior-history
cargo run --bin evermemos-mcp --features behavior-history
```

### 4) MCP server (for Claude/Cursor/Continue)

```bash
cargo run --bin evermemos-mcp
```

## Binaries

- `evermemos`: HTTP API server
- `evermemos-mcp`: MCP JSON-RPC 2.0 server over stdio
- `evermemos-worker`: standalone NATS consumer worker

## Core Commands

```bash
just build
just check
just fmt
just lint
just unit-test
just test-completeness
just test-parity
```

## Documentation

- [Documentation Index](/evermemos-rs/docs/README.md)
- [Deployment Guide](/evermemos-rs/docs/DEPLOYMENT.md)
- [Configuration Reference](/evermemos-rs/docs/CONFIG_REFERENCE.md)
- [File Guide](/evermemos-rs/docs/FILE_GUIDE.md)
- [MCP Agent Rules](/evermemos-rs/docs/MCP_AGENT_RULES.md)
- [Rust vs Python Parity](/evermemos-rs/docs/RUST_VS_PYTHON.md)

## Project Status

Current phase: core functionality complete, stabilization/production hardening in progress.

Completed milestones include:
- BehaviorHistory implementation (feature-gated)
- MCP server and memory tools
- OpenTelemetry tracing/metrics integration

Current priorities:
- Rate limiting
- auth hardening
- multi-tenant isolation completion
- operations/load-test refinement
