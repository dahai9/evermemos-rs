# ed65938 — feat(evermemos-rs): OpenTelemetry observability (traces + metrics)

## 限制

P2 优先级：为 evermemos-rs 接入 OpenTelemetry 可观测性，替代 Prometheus。

## 背景

evermemos-rs 原本只有 `tracing-subscriber` 的 stdout fmt 输出，没有 metrics、没有 trace export、没有 Grafana dashboard。P2 要求实现监控，同时不引入 Prometheus（用户希望平替）。

选择 OTel 的原因：
- 与现有 `tracing` crate 无缝桥接（`tracing-opentelemetry` layer）
- HTTP/protobuf 传输，不引入 tonic/gRPC（避免与 tower 栈的版本冲突风险）
- `grafana/otel-lgtm` 单容器本地 stack，极简部署
- `OTEL_ENABLED=false` 默认关闭，向后兼容无 collector 环境

## 解法

### 架构

```
evermemos-rs ─── OTLP HTTP :4318 ──► OTel Collector (otel-lgtm)
                                          │
                               ┌──────────┴──────────┐
                            Tempo                   Mimir
                        (trace storage)         (metrics storage)
                               └──────────┬──────────┘
                                       Grafana
                                   http://localhost:3000
```

### 关键设计决策

| 决策 | 原因 |
|------|------|
| HTTP/protobuf 不用 gRPC | 避免引入 tonic，与 axum 0.8 / tower 0.5 栈无冲突 |
| `reqwest-client` + `reqwest-rustls` | 复用已有 reqwest 0.12 dep，Cargo 不会产生重复 crate |
| `opentelemetry_sdk` 加 `rt-tokio` | batch exporter 后台 flush task 需要 Tokio runtime |
| `TelemetryGuard` 持有两个 provider | Drop 时保证 trace + metrics 都能 flush |
| `OTEL_ENABLED=false` 默认 | 无 collector 时零成本（no-op meter provider） |
| `OnceLock` 缓存 Instruments | 避免每请求重建 Counter/Histogram |

### Key diff

**Cargo.toml** — 新增 4 个 crate（复用现有 reqwest 0.12）:
```toml
opentelemetry       = { version = "0.28", features = ["trace", "metrics"] }
opentelemetry_sdk   = { version = "0.28", features = ["trace", "metrics", "rt-tokio"] }
opentelemetry-otlp  = { version = "0.28", default-features = false, features = [
    "trace", "metrics", "http-proto", "reqwest-client", "reqwest-rustls",
] }
tracing-opentelemetry = "0.29"
```

**main.rs before:**
```rust
app_tracing::init();
```
**main.rs after:**
```rust
let _telemetry = app_tracing::init();  // guard held to end of main → graceful flush
```

**router before:**
```rust
let app = memory_routes(state)
    .merge(health_routes())
    ...
```
**router after:**
```rust
let app = memory_routes(state)
    .merge(health_routes())
    ...
    .layer(axum::middleware::from_fn(metrics_middleware))  // ← new
    ...
```

## 影响文件

| 文件 | 改动 |
|------|------|
| `Cargo.toml` | 新增 opentelemetry / opentelemetry_sdk / opentelemetry-otlp / tracing-opentelemetry |
| `src/core/telemetry.rs` (NEW) | TelemetryGuard + init() + init_with_otel() + init_fmt_only() |
| `src/core/metrics.rs` (NEW) | `metrics_middleware` — Counter + Histogram via OnceLock |
| `src/core/tracing.rs` | 改为 shim → `pub use super::telemetry::{init, TelemetryGuard}` |
| `src/core/mod.rs` | 注册 `pub mod telemetry` + `pub mod metrics` |
| `src/main.rs` | `let _telemetry = ...`; import + wire `metrics_middleware` |
| `src/api/health_router.rs` | 响应加 `observability.{otel_enabled, otlp_endpoint}` |
| `.env` / `.env.template` | 新增 `OTEL_ENABLED`, `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME` |
| `docker-compose.otel.yaml` (NEW) | `grafana/otel-lgtm` 单容器 stack |

## 验证

```bash
# 1. cargo check — 0 error, 0 warning
cargo check 2>&1 | grep -E "^error|Finished"
# → Finished dev profile [unoptimized + debuginfo] target(s) in 1.50s

# 2. Default mode (OTEL disabled) — server starts & health OK
just start && sleep 10
curl -s http://localhost:8080/health
# → {"status":"ok","service":"evermemos-rs","version":"0.1.0",
#    "observability":{"otel_enabled":false,"otlp_endpoint":"disabled"}}

# 3. OTEL enabled mode (requires docker-compose.otel.yaml running)
docker compose -f docker-compose.otel.yaml up -d
OTEL_ENABLED=true just start
curl http://localhost:8080/api/v1/behavior-history?user_id=alice
# → traces appear in Grafana Explore → Tempo
# → http.server.requests{http.method="GET",...} in Grafana Explore → Metrics
```
