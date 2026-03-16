# Rate Limiting

## Algorithm

Token bucket via the `governor` crate. Each bucket refills at a constant rate. A request is allowed if a token is available; otherwise, a `429 Too Many Requests` response is returned.

## Per-Layer Configuration

| Layer | Rate | Burst | Key | Response Header |
|-------|------|-------|-----|-----------------|
| Public | 60 req/min | 10 | Client IP (X-Forwarded-For or peer addr) | `Retry-After: <seconds>` |
| Internal | 120 req/min | 20 | User ID (from session), fallback to IP | `Retry-After: <seconds>` |
| System | 30 req/min | 5 | Global (single bucket) | `Retry-After: <seconds>` |
| Auth | 10 req/min | 3 | Client IP | `Retry-After: <seconds>` |

## Implementation

```rust
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter<String, DashMapStateStore<String>, DefaultClock>>,
    key_extractor: KeyExtractor,
}

pub enum KeyExtractor {
    /// Extract IP from X-Forwarded-For (if TRUST_X_FORWARDED_FOR) or peer address
    PerIp,
    /// Extract session user_id if available, otherwise fall back to PerIp
    PerUserOrIp,
    /// Single bucket for all requests
    Global,
}
```

The `governor` crate uses `DashMap` internally — a sharded concurrent `HashMap` backed by atomic operations. No mutex contention on the hot path.

## IP Extraction Security

When `TRUST_X_FORWARDED_FOR=true`:
- Take the **last** IP in `X-Forwarded-For` header (rightmost = added by the trusted reverse proxy)
- If header is missing, use TCP peer address

When `TRUST_X_FORWARDED_FOR=false`:
- Always use TCP peer address (safe for direct connections)

## Response on Rate Limit

```
HTTP/1.1 429 Too Many Requests
Retry-After: 5
Content-Type: application/json

{
  "error": "ERR-RATELIMIT-001",
  "message": "Rate limit exceeded. Please retry after the indicated time.",
  "details": null
}
```
