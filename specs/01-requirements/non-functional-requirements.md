# Non-Functional Requirements

## Performance

| ID | Requirement | Target | Measurement |
|----|------------|--------|-------------|
| NFR-PERF-001 | Public API latency (cached response) | P99 < 5ms | k6 load test against `/public/members` with warm cache |
| NFR-PERF-002 | Public API latency (DB hit, no cache) | P99 < 50ms | k6 load test against `/public/members/{id}` |
| NFR-PERF-003 | Internal API latency (session lookup + DB query) | P99 < 100ms | k6 load test against `/internal/me/profile` |
| NFR-PERF-004 | System API latency (event upsert) | P99 < 200ms | k6 load test against `/system/events` |
| NFR-PERF-005 | Auth callback total time (Discord API + DB + redirect) | P99 < 2s | Synthetic monitoring (Discord API is the bottleneck) |
| NFR-PERF-006 | Concurrent connections | ≥ 200 simultaneous | k6 with 200 VUs sustained for 60s |
| NFR-PERF-007 | Memory footprint (idle, no traffic) | < 50 MB RSS | `docker stats` after 60s idle |
| NFR-PERF-008 | Memory footprint (under load, 50 concurrent) | < 150 MB RSS | `docker stats` during k6 test |
| NFR-PERF-009 | Docker image size | < 50 MB (compressed) | `docker images` after multi-stage build |

## Scalability

| ID | Requirement | Target | Rationale |
|----|------------|--------|-----------|
| NFR-SCALE-001 | Data volume: users | Up to 1,000 rows | 2–4x headroom over expected 300 |
| NFR-SCALE-002 | Data volume: events | Up to 10,000 rows | Multi-year accumulation |
| NFR-SCALE-003 | Data volume: gallery images | Up to 50,000 rows | High-growth scenario |
| NFR-SCALE-004 | Single-node deployment | Must handle all traffic on one Docker host | No horizontal scaling required (see A-004) |

## Availability

| ID | Requirement | Target | Measurement |
|----|------------|--------|-------------|
| NFR-AVAIL-001 | Uptime SLA | 99.5% monthly | Uptime monitoring (e.g., UptimeRobot) |
| NFR-AVAIL-002 | Planned maintenance window | ≤ 5 minutes downtime per deployment | Timed `docker compose` restart |
| NFR-AVAIL-003 | Recovery Time Objective (RTO) | < 30 minutes | From alert to service restored |
| NFR-AVAIL-004 | Recovery Point Objective (RPO) | < 1 hour | PostgreSQL WAL archiving + daily pg_dump |
| NFR-AVAIL-005 | Graceful shutdown | Drain in-flight requests within 30 seconds | Tokio graceful shutdown + SIGTERM handling |

## Security

| ID | Requirement | Target | Reference |
|----|------------|--------|-----------|
| NFR-SEC-001 | No SQL injection | Zero instances | SQLx parameterized queries (compile-time verified) |
| NFR-SEC-002 | No XSS via user content | Zero instances | ammonia HTML sanitization on all user Markdown |
| NFR-SEC-003 | CSRF protection on mutating endpoints | All POST/PUT/PATCH/DELETE on Internal API | Origin header validation against `FRONTEND_ORIGIN` |
| NFR-SEC-004 | Timing-safe token comparison | System API token | SHA-256 + `subtle::ConstantTimeEq` |
| NFR-SEC-005 | Session cookies are HttpOnly, Secure, SameSite | All session cookies | Enforced in cookie builder |
| NFR-SEC-006 | Rate limiting on all API layers | Per-layer limits (see API spec) | `governor` crate with in-memory state |
| NFR-SEC-007 | Dependency vulnerability scanning | Zero known critical CVEs at release | `cargo audit` in CI |
| NFR-SEC-008 | No secrets in logs | Zero secret values logged | Structured logging with redaction |

## Observability

| ID | Requirement | Target | Measurement |
|----|------------|--------|-------------|
| NFR-OBS-001 | Structured JSON logging | All log output is machine-parseable JSON | `tracing` + `tracing-subscriber` with JSON formatter |
| NFR-OBS-002 | Request ID propagation | Every request/response includes `X-Request-Id` header | Tower middleware generates UUID if not present |
| NFR-OBS-003 | Request duration logging | Every request logs method, path, status, duration_ms | `tower-http::trace` layer |
| NFR-OBS-004 | Health check endpoint | `GET /health` returns 200 with DB connectivity status | Kubernetes/Docker liveness probe compatible |
| NFR-OBS-005 | Prometheus-compatible metrics endpoint | `GET /metrics` exposes request count, latency histogram, error rate | `metrics` crate + `metrics-exporter-prometheus` |

## Build & Development

| ID | Requirement | Target | Measurement |
|----|------------|--------|-------------|
| NFR-BUILD-001 | Incremental compile time | < 30 seconds | `cargo build` after single-file change |
| NFR-BUILD-002 | Full release build time | < 5 minutes | `cargo build --release` from clean (CI) |
| NFR-BUILD-003 | Rust edition | 2024 | `Cargo.toml` |
| NFR-BUILD-004 | MSRV (Minimum Supported Rust Version) | stable latest at development start | `rust-toolchain.toml` |
| NFR-BUILD-005 | Clippy lint pass | Zero warnings with `clippy::pedantic` | CI gate |
| NFR-BUILD-006 | `cargo fmt` compliance | Zero formatting violations | CI gate |
