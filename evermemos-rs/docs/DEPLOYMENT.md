# Deployment Guide (evermemos-rs)

## 1. Prerequisites

- Rust toolchain (stable)
- `cargo`, `just`
- One OpenAI-compatible LLM endpoint for extraction/retrieval quality
- Optional: NATS server (only if you use async message ingestion)
- Optional: OTEL collector (if metrics/traces export enabled)

## 2. Environment Setup

```bash
cd evermemos-rs
cp .env.template .env
```

Fill at least:
- `LLM__BASE_URL`, `LLM__API_KEY`, `LLM__MODEL`
- `VECTORIZE__BASE_URL`, `VECTORIZE__API_KEY`, `VECTORIZE__MODEL`

Default local storage uses embedded SurrealDB:
- `SURREAL__ENDPOINT=rocksdb://./data/surreal`

## 3. Install Mode (`cargo install`)

If you want reusable CLI binaries instead of `cargo run`, install both server and MCP:

### Local source install

```bash
cd evermemos-rs
cargo install --path . --bin evermemos --bin evermemos-mcp
```

### Install from Git repository

```bash
cargo install --git https://github.com/dahai9/evermemos-rs --bin evermemos --bin evermemos-mcp
```

### With BehaviorHistory feature

```bash
cargo install --path . --features behavior-history --bin evermemos --bin evermemos-mcp
```

Installed commands:
- `evermemos`
- `evermemos-mcp`

By default binaries are placed in Cargo's bin directory (typically `~/.cargo/bin`).

## 4. Server Mode (HTTP API)

### Development

```bash
cargo run --bin evermemos
# or
just serve
```

### Background run

```bash
just start
just logs
just stop
```

### Health check

```bash
curl http://127.0.0.1:8080/health
```

## 5. BehaviorHistory Feature

BehaviorHistory is feature-gated and disabled by default.

Enable when needed:

```bash
cargo run --bin evermemos --features behavior-history
```

When enabled, routes include:
- `GET/POST /api/v1/behavior-history`
- `DELETE /api/v1/behavior-history/{id}`
- `GET /api/v1/behavior-history/stats`

## 6. MCP Mode

Run MCP stdio bridge:

```bash
cargo run --bin evermemos-mcp
```

With BehaviorHistory tools:

```bash
cargo run --bin evermemos-mcp --features behavior-history
```

Required MCP env vars (client side):
- `EVERMEMOS_BASE_URL`
- `EVERMEMOS_GROUP_ID`
- `EVERMEMOS_USER_ID`
- Optional: `EVERMEMOS_API_KEY`, `EVERMEMOS_RETRIEVE_METHOD`

## 7. Worker Mode (NATS Consumer)

Use standalone worker for dedicated ingestion process:

```bash
cargo run --bin evermemos-worker
```

Ensure:
- `NATS__ENABLED=true`
- `NATS__URL` reachable
- `NATS__SUBJECT_MEMORIZE` matches producer contract

## 8. Release Build

```bash
cargo build --release --bin evermemos
cargo build --release --bin evermemos-mcp
cargo build --release --bin evermemos-worker
```

Optimizations are preconfigured in `Cargo.toml` release profile (`opt-level=z`, LTO, strip, panic=abort).

## 9. Observability (OTEL)

Enable export:

```dotenv
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
OTEL_SERVICE_NAME=evermemos-rs
```

Local OTEL stack example:

```bash
docker compose -f docker-compose.otel.yaml up -d
```

## 10. Validation Checklist

Before production rollout:
- `cargo check`
- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test`
- `curl /health` passes
- expected API auth behavior verified (`API_KEY` on/off)
- data directory and backup strategy confirmed (`data/surreal`)

## 11. Common Issues

- LLM 401/403: verify `LLM__API_KEY` and endpoint compatibility.
- Empty retrieval quality: verify embedding model and `VECTORIZE__DIMENSIONS`.
- MCP connected but no data: confirm `EVERMEMOS_BASE_URL` and server is healthy.
- BehaviorHistory not found: binary likely started without `--features behavior-history`.
