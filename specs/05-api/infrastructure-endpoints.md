# Infrastructure Endpoints

Operational endpoints for health checks and metrics collection. Not rate-limited.

---

## GET `/health`

Liveness and readiness probe. Checks application state and database connectivity.

### Response — `200 OK`

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "database": "connected",
  "timestamp": "2025-06-15T10:00:00Z"
}
```

### Response — `503 Service Unavailable`

```json
{
  "status": "unhealthy",
  "version": "0.1.0",
  "uptime_seconds": 86400,
  "database": "disconnected",
  "timestamp": "2025-06-15T10:00:00Z"
}
```

### Implementation

```rust
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1")
        .fetch_one(&state.db_pool)
        .await
        .is_ok();

    let body = HealthResponse {
        status: if db_ok { "healthy" } else { "unhealthy" },
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        database: if db_ok { "connected" } else { "disconnected" },
        timestamp: Utc::now(),
    };

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(body))
}
```

---

## GET `/metrics`

Prometheus-compatible metrics endpoint. Exposes runtime and application metrics.

### Response — `200 OK`

```
Content-Type: text/plain; version=0.0.4; charset=utf-8
```

### Exposed Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `http_requests_total{method, path, status}` | Counter | Total HTTP requests |
| `http_request_duration_seconds{method, path}` | Histogram | Request latency (buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s) |
| `http_requests_in_flight` | Gauge | Currently processing requests |
| `db_pool_connections_active` | Gauge | Active database connections |
| `db_pool_connections_idle` | Gauge | Idle database connections |
| `db_query_duration_seconds{query}` | Histogram | Database query latency |
| `sessions_active_total` | Gauge | Active sessions (refreshed periodically) |
| `rate_limit_rejections_total{layer}` | Counter | Rate limit rejections per API layer |
| `discord_api_requests_total{endpoint, status}` | Counter | Discord API call count |
| `discord_api_duration_seconds{endpoint}` | Histogram | Discord API call latency |

### Implementation

Uses `metrics` crate with `metrics-exporter-prometheus` backend. A middleware layer records request-level metrics automatically.

```rust
// In main.rs / router setup
let metrics_handle = PrometheusBuilder::new()
    .install_recorder()
    .expect("failed to install metrics recorder");

let app = Router::new()
    .route("/metrics", get(move || std::future::ready(metrics_handle.render())))
    // ...
    .layer(MetricsLayer::new());
```
