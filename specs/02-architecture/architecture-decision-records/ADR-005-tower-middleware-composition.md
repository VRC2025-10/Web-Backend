# ADR-005: Tower Middleware Composition

## Status
Accepted

## Context

Each API layer has different middleware requirements:

| Concern | Public | Internal | System | Auth |
|---------|--------|----------|--------|------|
| Rate limit | 60/min/IP | 120/min/user | 30/min/token | 10/min/IP |
| CORS | Yes | Yes | No | Yes |
| CSRF (Origin check) | No | Yes (mutations) | No | No |
| Session auth | No | Yes | No | No |
| Bearer auth | No | No | Yes | No |
| Cache-Control | `public, max-age=30` | `private, no-store` | None | None |
| Request logging | Yes | Yes | Yes | Yes |
| Request ID | Yes | Yes | Yes | Yes |
| Metrics | Yes | Yes | Yes | Yes |

Tower's `Layer` trait allows composing these as typed middleware stacks.

## Decision

Build each API layer's router with an explicitly typed middleware stack. Use Tower's `.layer()` method on Axum's `Router` for per-group middleware.

### Router Composition

```rust
fn build_router(state: AppState) -> Router {
    // Shared layers (applied to all routes)
    let shared = ServiceBuilder::new()
        .layer(RequestIdLayer::new())
        .layer(RequestLoggingLayer::new())
        .layer(MetricsLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CatchPanicLayer::new());

    // Per-layer routers with specific middleware
    let public_routes = public_router()
        .layer(CorsLayer::permissive_read_only())
        .layer(CacheControlLayer::public_cache(30))
        .layer(RateLimitLayer::per_ip(60));

    let internal_routes = internal_router()
        .layer(CorsLayer::with_origin(&state.config.frontend_origin))
        .layer(CsrfLayer::new(&state.config.frontend_origin))
        .layer(CacheControlLayer::private_no_store())
        .layer(RateLimitLayer::per_user_or_ip(120));

    let system_routes = system_router()
        .layer(RateLimitLayer::global(30));

    let auth_routes = auth_router()
        .layer(CorsLayer::with_origin(&state.config.frontend_origin))
        .layer(RateLimitLayer::per_ip(10));

    Router::new()
        .nest("/api/v1/public", public_routes)
        .nest("/api/v1/internal", internal_routes)
        .nest("/api/v1/system", system_routes)
        .nest("/api/v1/auth", auth_routes)
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_endpoint))
        .layer(shared)
        .with_state(state)
}
```

### Custom Layer Implementations

#### RateLimitLayer

```rust
pub struct RateLimitLayer {
    governor: Arc<RateLimiter<...>>,
    key_extractor: KeyExtractor,
}

pub enum KeyExtractor {
    PerIp,                    // Extract from X-Forwarded-For or peer addr
    PerUserOrIp,              // Extract from session user_id, fallback to IP
    Global,                   // Single global bucket
}
```

Uses `governor` crate with `DashMap`-based keyed rate limiter. Lock-free on the hot path (atomic increment + check).

#### CsrfLayer

```rust
pub struct CsrfLayer {
    expected_origin: String,
}

impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CsrfService {
            inner,
            expected_origin: self.expected_origin.clone(),
        }
    }
}
```

Only applies to `POST`, `PUT`, `PATCH`, `DELETE` methods. Checks `Origin` header equals `expected_origin`.

#### CacheControlLayer

```rust
pub enum CachePolicy {
    PublicCache { max_age: u32, stale_while_revalidate: u32 },
    PrivateNoStore,
    NoCache,
}
```

Injects `Cache-Control` header into every response based on the policy.

## Consequences

### Positive
- Each API layer's middleware requirements are visible in the router composition code
- Tower's type system ensures layers are composed in the correct order
- Custom layers are reusable and testable in isolation
- Adding a new API layer (e.g., `v2`) just means a new router with its own stack

### Negative
- Tower layer type signatures can become deeply nested and hard to read in error messages
- Custom `Layer` + `Service` impls require understanding Tower's `poll_ready` / `call` contract

### Risks
- `governor` crate's in-memory rate limiter loses state on restart → Acceptable for our scale; state rebuild is instant

## Alternatives Considered

### Alternative 1: Axum middleware functions only (no Tower layers)
- **Pros**: Simpler, Axum-native feel
- **Cons**: Less composable, no per-layer configuration, middleware ordering is implicit
- **Rejection Reason**: Tower layers are more powerful and educational

### Alternative 2: External rate limiter (Redis-based)
- **Pros**: Survives restarts, shared across instances
- **Cons**: Adds Redis dependency for a single-node deployment; overkill
- **Rejection Reason**: In-memory `governor` is sufficient (see A-004)
