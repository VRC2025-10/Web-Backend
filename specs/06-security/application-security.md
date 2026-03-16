# Application Security

## HTTP Security Headers

Set by Caddy reverse proxy (global) and/or Axum middleware:

```
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
Content-Security-Policy: default-src 'none'; frame-ancestors 'none'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Cache-Control: no-store  (for non-public endpoints)
```

Note: The `Content-Security-Policy` is restrictive because this is a JSON API — no HTML is served except error pages. If the backend ever serves HTML, the CSP must be expanded.

## CORS Configuration

```rust
let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact(
        config.frontend_origin.parse().unwrap()
    ))
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
    .allow_headers([
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::ORIGIN,
    ])
    .allow_credentials(true)  // Required for cookie-based auth
    .max_age(Duration::from_secs(3600));
```

Key decisions:
- **Single origin only**: `FRONTEND_ORIGIN` env var (no wildcards)
- **Credentials allowed**: Required for `session_id` cookie to be sent cross-origin
- **No `Authorization` header**: We use cookies, not Bearer tokens (except System API which is server-to-server)

## CSRF Protection

### Strategy: Origin Header Validation

All state-changing requests (`POST`, `PUT`, `PATCH`, `DELETE`) on the Internal API layer must include an `Origin` header matching `FRONTEND_ORIGIN`.

```rust
pub struct CsrfLayer {
    allowed_origin: HeaderValue,
}

impl<S> tower::Layer<S> for CsrfLayer {
    type Service = CsrfMiddleware<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CsrfMiddleware {
            inner,
            allowed_origin: self.allowed_origin.clone(),
        }
    }
}

// In the middleware:
fn check_csrf(request: &Request, allowed_origin: &HeaderValue) -> Result<(), ApiError> {
    let method = request.method();
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return Ok(());  // Safe methods — no CSRF check
    }

    let origin = request.headers()
        .get(header::ORIGIN)
        .ok_or(ApiError::CsrfFailed)?;

    if origin != allowed_origin {
        return Err(ApiError::CsrfFailed);
    }

    Ok(())
}
```

### Why Origin Header (not CSRF Tokens)?

1. **Simplicity**: No need to generate, store, or transmit separate CSRF tokens
2. **Stateless**: No server-side CSRF token storage needed
3. **Browser guarantee**: `Origin` header is set by the browser and cannot be forged by JavaScript
4. **SameSite=Lax**: Session cookie is already protected from cross-origin POST by `SameSite=Lax`; Origin check is defense-in-depth

The combination of `SameSite=Lax` cookies + `Origin` header validation provides double CSRF protection.

### System API Exemption

System API endpoints (`/api/v1/system/*`) do NOT perform CSRF checks because:
- They use Bearer token authentication (not cookies)
- Callers are server-side systems (GAS, bots), not browsers
- There is no cookie to CSRF-exploit

## Input Validation

### Request Body Size Limits

```rust
// Global limit applied at Axum layer
let app = Router::new()
    // ...
    .layer(DefaultBodyLimit::max(1_048_576));  // 1 MB
```

### Field-Level Validation Rules

All validation is performed in the domain layer before any database interaction.

| Field | Rule | Implementation |
|-------|------|----------------|
| `nickname` | 1–50 chars, trimmed | `len()` check after `.trim()` |
| `vrc_id` | Regex: `^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$` | Compiled `once_cell` regex |
| `x_id` | Regex: `^[a-zA-Z0-9_]{1,15}$` | Compiled regex |
| `bio_markdown` | Max 2000 chars | `len()` check |
| `avatar_url` | Starts with `https://`, max 500 chars | Prefix check + `len()` |
| `image_url` | Starts with `https://`, max 500 chars | Prefix check + `len()` |
| `caption` | Max 200 chars | `len()` check |
| `reason` (report) | 10–1000 chars | `len()` check |
| `name` (club) | 1–100 chars | `len()` check |
| `description_markdown` | Max 2000 chars | `len()` check |
| `external_id` | 1–100 chars | `len()` check |
| `title` (event) | 1–200 chars | `len()` check |
| `tags[]` | Each 1–50 chars, max 10 tags | Per-element + `len()` check |
| `page` | ≥ 1, integer | Serde + manual check |
| `per_page` | 1–100, integer | Serde + manual check |
| `redirect_to` | Starts with `/`, no `//`, no `\`, no control chars | Custom validator |

### Markdown → HTML Pipeline

```
User Input (bio_markdown)
  │
  ├─ 1. Length check (≤ 2000 chars)
  │
  ├─ 2. pulldown-cmark: Markdown → Raw HTML
  │     Options: ENABLE_STRIKETHROUGH (no raw HTML passthrough)
  │
  ├─ 3. ammonia::clean(): Sanitize HTML
  │     Allowed tags: p, h1-h6, strong, em, a, ul, ol, li,
  │                   code, pre, blockquote, br, img
  │     Allowed attributes:
  │       a: href (must be http:// or https://)
  │       img: src (must be https://), alt
  │     All other tags/attributes stripped
  │     Event handlers (onclick, etc.) stripped
  │     JavaScript URIs stripped
  │
  ├─ 4. Post-sanitization check
  │     Reject if output contains: <script, javascript:, on\w+=
  │     (defense-in-depth against ammonia bypass)
  │
  └─ 5. Store both bio_markdown and bio_html
```

```rust
use pulldown_cmark::{Parser, Options, html::push_html};
use ammonia::Builder;

pub fn render_markdown(input: &str) -> Result<String, DomainError> {
    // Parse markdown
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(input, options);

    let mut raw_html = String::with_capacity(input.len() * 2);
    push_html(&mut raw_html, parser);

    // Sanitize
    let clean_html = Builder::new()
        .tags(hashset!["p", "h1", "h2", "h3", "h4", "h5", "h6",
                        "strong", "em", "a", "ul", "ol", "li",
                        "code", "pre", "blockquote", "br", "img"])
        .tag_attributes(hashmap![
            "a" => hashset!["href"],
            "img" => hashset!["src", "alt"]
        ])
        .url_schemes(hashset!["http", "https"])
        .link_rel(Some("noopener noreferrer"))
        .clean(&raw_html)
        .to_string();

    // Post-sanitization defense-in-depth check
    let lower = clean_html.to_lowercase();
    if lower.contains("<script")
        || lower.contains("javascript:")
        || regex_is_match!(r"on\w+=", &lower)
    {
        return Err(DomainError::BioDangerous);
    }

    Ok(clean_html)
}
```

## Dependency Security

### Supply Chain Protection

- `Cargo.lock` committed to version control (reproducible builds)
- `cargo audit` run in CI pipeline (checks for known CVEs in dependencies)
- `cargo deny` for license compliance and duplicate dependency detection
- Minimal dependency tree philosophy — avoid transitive dependency bloat
- Pin major versions in `Cargo.toml` (e.g., `axum = "0.8"` not `axum = "*"`)

### Security-Sensitive Dependencies

| Crate | Purpose | Security Relevance |
|-------|---------|-------------------|
| `ring` | HMAC-SHA256, SHA-256, CSPRNG | Core cryptographic operations |
| `subtle` | Constant-time comparison | System API token validation |
| `ammonia` | HTML sanitization | XSS prevention |
| `reqwest` | HTTP client (Discord API) | TLS, certificate validation |
| `sqlx` | Database access | SQL injection prevention (compile-time verification) |
| `tower-http` | CORS, body limits | HTTP security headers |

These crates are audited by the Rust community via `cargo-crev` and have strong security track records.
