# Performance Optimization Notes

## Build Profile
- Release: LTO fat, codegen-units=1, strip=symbols, panic=abort, opt-level=3
- Binary: 16MB (debug defaults) → 8.3MB (optimized release)

## Key Optimizations Applied
- **Middleware**: metrics path normalization (single String::with_capacity), rate_limit key extraction (token prefix vs SHA256), request_id (pre-create HeaderValue), CSRF (zero-copy &str), security headers (1 layer vs 6)
- **DB Pool**: min_connections=2, acquire_timeout=5s, idle_timeout=600s, max_lifetime=1800s
- **SQL**: list_clubs correlated COUNT → LEFT JOIN GROUP BY; system.rs host resolution 2 queries → 1
- **Dockerfile**: RUSTFLAGS="-C target-cpu=x86-64-v3", explicit strip
- **Dependencies**: tokio/hyper/tower features trimmed from "full"

## PostgreSQL Indexes Added (migration 20250103000000)
- idx_sessions_token_hash_expires: composite for auth hot path
- idx_users_active_joined: partial on status='active' for public member listing
- idx_reports_status_created: composite for admin report filtering
- idx_events_status_start_time: composite for event listing

## Notes
- XSS check in internal.rs uses hand-written byte scanner, not regex — already fast
- Proc macro validate uses LazyLock for regex — correct
- PostgreSQL partial index cannot use NOW() (STABLE, not IMMUTABLE) — removed partial predicate from sessions index
