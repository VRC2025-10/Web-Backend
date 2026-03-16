# Technology Selection

## Final Technology Stack

| Category | Selection | Version | Justification |
|----------|-----------|---------|---------------|
| **Language** | Rust | Edition 2024, stable | Zero-cost abstractions, memory safety without GC, type system enables compile-time guarantees |
| **Async Runtime** | Tokio | 1.x (latest) | De facto standard for async Rust; multi-threaded work-stealing scheduler |
| **HTTP Framework** | Axum | 0.8.x | Tower-native, type-safe extractors, first-party Tokio integration |
| **Database Driver** | SQLx | 0.8.x | Compile-time SQL verification, async, pure-Rust PostgreSQL driver |
| **Database** | PostgreSQL | 16.x | Best open-source RDBMS; JSONB, UUIDs, ENUMs, CTEs, window functions |
| **HTTP Client** | reqwest | 0.12.x | De facto standard; TLS via rustls, connection pooling, async |
| **Serialization** | serde + serde_json | 1.x | Universal Rust serialization; zero-copy deserialize support |
| **Error Handling** | thiserror | 2.x | Derive macro for `std::error::Error`; clean error enums |
| **Markdown** | pulldown-cmark | 0.12.x | CommonMark-compliant, streaming parser, no allocations for simple docs |
| **HTML Sanitizer** | ammonia | 4.x | Configurable allowlist-based sanitizer; prevents all XSS vectors |
| **Rate Limiting** | governor | 0.8.x | Token-bucket algorithm, keyed (per-IP/user), lock-free atomic operations |
| **Logging** | tracing + tracing-subscriber | 0.1.x | Structured, async-aware, span-based instrumentation; JSON output |
| **Metrics** | metrics + metrics-exporter-prometheus | 0.24.x | Prometheus-compatible; histograms, counters, gauges |
| **Password/Token** | subtle | 2.x | Constant-time comparison for timing-attack-safe token verification |
| **UUID** | uuid | 1.x | v4 generation + serde + sqlx integration |
| **Time** | chrono | 0.4.x | DateTime handling, UTC, ISO 8601 parsing; serde + sqlx integration |
| **Environment** | dotenvy | 0.15.x | `.env` file loading (successor to `dotenv`) |
| **Global Allocator** | tikv-jemallocator | 0.6.x | Reduced fragmentation, better multi-threaded perf vs. system allocator |
| **TLS (for reqwest)** | rustls | via reqwest feature | Pure-Rust TLS, no OpenSSL dependency — simplifies Docker image |
| **Proc Macros** | syn + quote + proc-macro2 | 2.x | Building custom `#[derive]` and attribute macros |
| **Testing** | tokio::test + proptest + sqlx::test | latest | Async test runtime + property-based testing + DB test fixtures |
| **Formal Verification** | Kani (kani-verifier) | latest | AWS-backed bounded model checker for Rust; runs `#[kani::proof]` harnesses via CBMC on actual Rust code |
| **Reverse Proxy** | Caddy | 2.x | Automatic HTTPS (Let's Encrypt), simple config, HTTP/3 support |
| **Container** | Docker + docker-compose | latest | Multi-stage Rust build; `scratch`-based final image |

## Cargo.toml Dependency Map

```toml
[package]
name = "vrc-backend"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"          # MSRV: Rust 2024 edition requirement

[dependencies]
# Web framework
axum = { version = "0.8", features = ["macros"] }
axum-extra = { version = "0.10", features = ["cookie", "typed-header"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace", "timeout", "catch-panic", "request-id", "set-header"] }
tokio = { version = "1", features = ["full"] }
hyper = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "migrate"] }

# HTTP client (for Discord API)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "2"

# Markdown + sanitization
pulldown-cmark = "0.12"
ammonia = "4"

# Security
subtle = "2"
governor = { version = "0.8", features = ["dashmap"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dotenvy = "0.15"

# Global allocator
tikv-jemallocator = { version = "0.6", features = ["unprefixed_malloc_on_supported_platforms"] }

[dev-dependencies]
proptest = "1"
tokio-test = "0.4"
wiremock = "0.6"                    # HTTP mocking for Discord API tests
sqlx = { version = "0.8", features = ["testing"] }

[workspace]
members = [".", "vrc-macros"]

[profile.release]
lto = "fat"                         # Maximum link-time optimization
codegen-units = 1                   # Single codegen unit for best optimization
strip = "symbols"                   # Strip debug symbols from binary
panic = "abort"                     # No unwinding overhead
opt-level = 3                       # Maximum optimization
```

## Why Not Alternatives

### Language: Why Rust over Go/TypeScript/Python?

| Criterion | Rust | Go | TypeScript | Weight |
|-----------|------|-----|-----------|--------|
| Compile-time safety | 10 (borrow checker, type-state, exhaustive matching) | 5 (simple type system, no generics until recently) | 6 (TS types are unsound, runtime failures possible) | 5 |
| Performance | 10 (zero-cost abstractions, no GC) | 8 (GC pauses, but fast enough) | 4 (V8 overhead, GC) | 3 |
| Ecosystem maturity (web) | 7 (growing, Axum is solid) | 9 (stdlib net/http, Gin) | 10 (Express, Fastify, Next.js) | 2 |
| Educational value ("romance") | 10 (unique type system, borrow checker, proc macros) | 3 (intentionally simple) | 4 (well-known) | 5 |
| Binary deployment | 10 (single static binary, tiny Docker image) | 9 (single binary, slightly larger) | 5 (Node runtime needed) | 3 |
| **Weighted Score** | **9.4** | **5.8** | **5.4** | — |

**Decision**: Rust wins on the two highest-weighted criteria (compile-time safety and educational value).

### Framework: Why Axum over Actix-web/Rocket/Poem?

| Criterion | Axum | Actix-web | Rocket | Poem |
|-----------|------|-----------|--------|------|
| Tower middleware ecosystem | Native | Adapter needed | None | Partial |
| Type-safe extractors | Excellent | Good | Good | Good |
| Tokio integration | First-party | Own runtime | Tokio-based | Tokio-based |
| Active maintenance | Very active (tokio-rs) | Active | Slower releases | Active |
| Community size | Largest (2025-) | Historically largest | Medium | Small |

**Decision**: Axum — Tower-native middleware composition is central to our architecture.

### Database: Why PostgreSQL over SQLite/MySQL?

| Criterion | PostgreSQL | SQLite | MySQL |
|-----------|-----------|--------|-------|
| ENUMs (native) | Yes | No | Yes (limited) |
| UUID type | Yes (native) | No (text) | No (binary) |
| UPSERT (`ON CONFLICT`) | Full syntax | Basic | `ON DUPLICATE KEY` |
| JSON/JSONB | Excellent | JSON1 extension | JSON type |
| SQLx compile-time check | Full support | Full support | Full support |
| Concurrent writes | Excellent (MVCC) | Limited (file lock) | Good (InnoDB) |
| Docker simplicity | Official image | Embedded (no container) | Official image |

**Decision**: PostgreSQL — native ENUMs and UUIDs align perfectly with our schema design. SQLite lacks concurrent write support needed even at our modest scale.
