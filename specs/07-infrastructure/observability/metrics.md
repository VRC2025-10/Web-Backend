# Observability: Metrics, Logging & Monitoring

## Logging

### Strategy

Structured JSON logs via `tracing` + `tracing-subscriber` with `fmt::json()` formatter.

### Log Levels

| Level | Usage |
|-------|-------|
| `error` | Unrecoverable errors: DB connection failure, Discord API 5xx, panic |
| `warn` | Recoverable issues: rate limit hit, suspended user attempt, validation failure |
| `info` | Normal operations: request start/end, session created/deleted, event synced, deploy |
| `debug` | Development: SQL queries, full request/response bodies (NEVER in production) |
| `trace` | Framework internals (do not use in application code) |

### Log Format

```json
{
  "timestamp": "2025-06-15T10:00:00.123Z",
  "level": "INFO",
  "target": "vrc_backend::adapters::inbound::routes::auth",
  "message": "User authenticated",
  "span": {
    "request_id": "01J5EXAMPLE",
    "method": "GET",
    "path": "/api/v1/internal/auth/me",
    "user_id": 42
  }
}
```

### Request Tracing Middleware

Every request gets a unique `request_id` (ULID) injected into the tracing span:

```rust
use tower_http::trace::{TraceLayer, MakeSpan};
use tracing::Span;
use ulid::Ulid;

let trace_layer = TraceLayer::new_for_http()
    .make_span_with(|request: &Request<Body>| {
        let request_id = Ulid::new().to_string();
        tracing::info_span!(
            "http_request",
            request_id = %request_id,
            method = %request.method(),
            path = %request.uri().path(),
            status = tracing::field::Empty,
            latency_ms = tracing::field::Empty,
        )
    })
    .on_response(|response: &Response, latency: Duration, span: &Span| {
        span.record("status", response.status().as_u16());
        span.record("latency_ms", latency.as_millis());
        tracing::info!("Request completed");
    });
```

### Log Retention

- **Docker log driver**: `json-file` with `max-size=50m` and `max-file=5` (250 MB max per container)
- In production, consider forwarding to Loki or a log aggregation service for search and alerting

---

## Metrics

### Prometheus Metrics (via `metrics` + `metrics-exporter-prometheus`)

#### HTTP Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `http_requests_total` | Counter | `method`, `path`, `status` | Total HTTP requests |
| `http_request_duration_seconds` | Histogram | `method`, `path` | Request duration |
| `http_requests_in_flight` | Gauge | — | Concurrent requests |

Histogram buckets: `[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]`

#### Database Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `db_pool_active_connections` | Gauge | — | In-use connections |
| `db_pool_idle_connections` | Gauge | — | Idle connections |
| `db_query_duration_seconds` | Histogram | `query_name` | Query duration |

#### Application Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `sessions_active_total` | Gauge | — | Active sessions (polled every 60s) |
| `rate_limit_rejections_total` | Counter | `layer` | Rate limit 429 responses |
| `discord_api_duration_seconds` | Histogram | `endpoint` | Discord API latency |
| `discord_api_errors_total` | Counter | `endpoint`, `status` | Discord API errors |
| `markdown_render_duration_seconds` | Histogram | — | Markdown → HTML render time |

### Dashboard Panels (Grafana or similar)

| Panel | Query | Alert Threshold |
|-------|-------|-----------------|
| Request Rate | `rate(http_requests_total[5m])` | > 100 req/s sustained |
| Error Rate | `rate(http_requests_total{status=~"5.."}[5m])` | > 1% of total |
| P99 Latency | `histogram_quantile(0.99, http_request_duration_seconds)` | > 100ms |
| DB Pool Usage | `db_pool_active_connections / 20` | > 80% |
| Rate Limit Hits | `rate(rate_limit_rejections_total[5m])` | > 10/min (potential attack) |

---

## SLI/SLO Definitions

| SLI | Measurement | SLO Target |
|-----|-------------|------------|
| **Availability** | `1 - (5xx responses / total responses)` | 99.5% per month |
| **Latency (cached)** | P99 of responses with cache hit | < 5ms |
| **Latency (DB)** | P99 of responses requiring DB query | < 50ms |
| **Latency (Discord API)** | P99 of OAuth2 callback flow | < 3s |
| **Freshness** | Time between event sync and API availability | < 5 minutes |

### Error Budget

With 99.5% availability target:
- Monthly budget: 0.5% × 43,200 min ≈ **216 minutes downtime** per month
- This is generous for a community website — allows for maintenance windows and unexpected issues

### Alerting Rules

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| High Error Rate | 5xx > 5% for 5 min | Critical | Page on-call |
| DB Pool Exhausted | active = max for 2 min | Critical | Scale pool or investigate |
| High Latency | P99 > 500ms for 10 min | Warning | Investigate query performance |
| Discord API Down | errors > 50% for 5 min | Warning | Check Discord status page |
| Disk Space Low | < 20% free | Warning | Clean logs / expand disk |
| Certificate Expiry | < 7 days | Warning | Check Caddy auto-renewal |

Alerts are sent to a Discord webhook in the operations channel.
