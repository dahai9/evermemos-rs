# Configuration Reference (evermemos-rs)

Configuration is loaded from `.env` (via `dotenvy`) and nested keys use double underscore (`__`).

Example:

```dotenv
LLM__BASE_URL=http://127.0.0.1:4000
```

## 1. LLM

- `LLM__BASE_URL`: OpenAI-compatible chat endpoint base URL
- `LLM__API_KEY`: auth token
- `LLM__MODEL`: generation model (e.g. `gpt-4o-mini`)
- `LLM__TEMPERATURE`: decoding temperature
- `LLM__MAX_TOKENS`: max output tokens

## 2. Vectorize

- `VECTORIZE__BASE_URL`: embedding endpoint base URL
- `VECTORIZE__API_KEY`: embedding endpoint key
- `VECTORIZE__MODEL`: embedding model name
- `VECTORIZE__DIMENSIONS`: embedding dimension
- `VECTORIZE__BATCH_SIZE`: embedding request batch size

## 3. Rerank (optional)

- `RERANK__ENABLED`: `true/false`
- `RERANK__BASE_URL`: reranker endpoint
- `RERANK__API_KEY`: reranker auth key
- `RERANK__MODEL`: rerank model name

## 4. Storage (SurrealDB)

- `SURREAL__ENDPOINT`: embedded or remote endpoint
  - embedded example: `rocksdb://./data/surreal`
  - remote example: `ws://host:8000`
- `SURREAL__NS`: namespace
- `SURREAL__DB`: database name
- `SURREAL__USER`, `SURREAL__PASS`: credentials

## 5. Server

- `SERVER__HOST`: bind host (`0.0.0.0` for container/remote access)
- `SERVER__PORT`: listen port

## 6. NATS (optional)

- `NATS__ENABLED`: enable/disable worker and in-server consumer startup
- `NATS__URL`: NATS cluster URL
- `NATS__STREAM`: stream name
- `NATS__SUBJECT_MEMORIZE`: subject consumed for memory jobs

## 7. Tenant / Auth

- `TENANT__ORG_HEADER`: organization header key (default intended: `X-Organization-Id`)
- `TENANT__SPACE_HEADER`: space header key (default intended: `X-Space-Id`)
- `API_KEY`: bearer token for API auth
  - `none` or empty means auth disabled

## 8. Logging and Telemetry

- `RUST_LOG`: tracing filter directives
- `OTEL_ENABLED`: enable OTEL export
- `OTEL_EXPORTER_OTLP_ENDPOINT`: collector endpoint (HTTP/protobuf)
- `OTEL_SERVICE_NAME`: service name for observability backends

## 9. MCP Runtime Variables

Used by `evermemos-mcp` process (or MCP client launcher):

- `EVERMEMOS_BASE_URL`: API server URL
- `EVERMEMOS_GROUP_ID`: memory scope key for group/session
- `EVERMEMOS_USER_ID`: memory scope key for user
- `EVERMEMOS_API_KEY`: optional API token
- `EVERMEMOS_RETRIEVE_METHOD`: default retrieve strategy

## 10. Cassette Mode

For reproducible dev/test with reduced token usage:

- `LLM_CASSETTE_MODE`: `off` / `record` / `replay` / `auto`
- `LLM_CASSETTE_FILE`: cassette JSON file path

Typical commands:

```bash
just record
just replay
just auto-test
```

## 11. Recommended Profiles

### Minimal local dev

- Embedded SurrealDB
- `API_KEY=none`
- `OTEL_ENABLED=false`
- `NATS__ENABLED=false`

### Integration test / parity test

- `LLM_CASSETTE_MODE=record` once
- switch to `replay` for stable repeat runs

### Pre-production

- `API_KEY` non-empty
- `OTEL_ENABLED=true`
- stable remote model/vector/rerank endpoints
- explicit backup plan for SurrealDB data
