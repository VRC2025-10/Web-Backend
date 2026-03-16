# ADR-003: Algebraic Error Types

## Status
Accepted

## Context

The API has structured error codes (`ERR-AUTH-001`, `ERR-PROF-002`, etc.) that must map cleanly to HTTP status codes and JSON error bodies. We need an error system that:

1. Makes it impossible to forget an error variant (exhaustive matching)
2. Maps each error code from the spec to exactly one Rust enum variant
3. Converts cleanly between domain errors, infrastructure errors, and API errors
4. Never uses a catch-all `anyhow::Error` in production code paths

## Decision

Three-layer error algebra with total (non-lossy) conversions:

### Layer 1: Domain Errors

```rust
/// Business rule violations. No HTTP concepts.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    // Profile errors
    #[error("Profile validation failed")]
    ProfileValidation(ValidationDetails),
    #[error("XSS attempt detected in bio")]
    BioDangerous,
    #[error("Profile not found")]
    ProfileNotFound,

    // Auth errors
    #[error("Session expired or invalid")]
    SessionInvalid,
    #[error("Account suspended")]
    AccountSuspended,
    #[error("Not a guild member")]
    NotGuildMember,
    #[error("CSRF state mismatch")]
    CsrfMismatch,

    // Role errors
    #[error("Insufficient role: requires {required}, has {actual}")]
    InsufficientRole { required: &'static str, actual: String },
    #[error("Only super_admin can grant admin role")]
    AdminRoleEscalation,
    #[error("Only super_admin can grant super_admin role")]
    SuperAdminRoleEscalation,
    #[error("Cannot modify super_admin without being super_admin")]
    SuperAdminProtected,

    // Moderation errors
    #[error("Report target not found")]
    ReportTargetNotFound,
    #[error("Duplicate report")]
    DuplicateReport,
    #[error("Report reason out of range")]
    ReportReasonLength,

    // Event errors
    #[error("Event not found")]
    EventNotFound,

    // Club/Gallery errors
    #[error("Club not found")]
    ClubNotFound,
    #[error("Gallery image not found")]
    GalleryImageNotFound,
    #[error("Invalid gallery status")]
    InvalidGalleryStatus,

    // User errors
    #[error("User not found")]
    UserNotFound,
}
```

### Layer 2: Infrastructure Errors

```rust
/// Infrastructure failures. Not exposed to clients.
#[derive(Debug, thiserror::Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Discord API error: {0}")]
    DiscordApi(String),
    #[error("Webhook delivery failed: {0}")]
    Webhook(String),
    #[error("Token exchange failed")]
    TokenExchange,
}
```

### Layer 3: API Error (HTTP-facing)

```rust
/// The final error type returned to clients. Maps to HTTP status + JSON body.
#[derive(Debug)]
pub enum ApiError {
    // Domain errors (mapped from DomainError)
    ProfileValidation(ValidationDetails),   // 400, ERR-PROF-001
    BioDangerous,                            // 400, ERR-PROF-002
    ProfileNotFound,                         // 404, ERR-PROF-004
    SessionInvalid,                          // 401, ERR-AUTH-003
    AccountSuspended,                        // 403, ERR-AUTH-004
    CsrfFailed,                              // 403, ERR-CSRF-001
    InsufficientRole { .. },                 // 403, ERR-PERM-001 or ERR-PERM-002
    AdminRoleEscalation,                     // 403, ERR-ROLE-001
    SuperAdminRoleEscalation,                // 403, ERR-ROLE-002
    SuperAdminProtected,                     // 403, ERR-ROLE-003
    RoleLevelInsufficient,                   // 403, ERR-ROLE-004
    ReportTargetNotFound,                    // 404, ERR-MOD-001
    DuplicateReport,                         // 409, ERR-MOD-002
    ReportReasonLength,                      // 400, ERR-MOD-003
    EventNotFound,                           // 404, ERR-NOT-FOUND
    ClubNotFound,                            // 404, ERR-NOT-FOUND
    GalleryImageNotFound,                    // 404, ERR-NOT-FOUND
    InvalidGalleryStatus,                    // 400, ERR-GALLERY-003
    UserNotFound,                            // 404, ERR-USER-001
    SystemTokenInvalid,                      // 401, ERR-SYNC-001
    SystemValidation(ValidationDetails),     // 400, ERR-SYNC-002
    RateLimited,                             // 429, ERR-RATELIMIT-001
    ValidationError(ValidationDetails),      // 400, ERR-VALIDATION

    // Infrastructure errors (logged, not exposed)
    Internal(String),                        // 500, ERR-INTERNAL (message redacted in response)
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match self {
            Self::ProfileValidation(d) => (StatusCode::BAD_REQUEST, "ERR-PROF-001", "Validation failed", Some(d)),
            Self::BioDangerous => (StatusCode::BAD_REQUEST, "ERR-PROF-002", "Dangerous content detected in bio", None),
            // ... exhaustive match arms for all variants ...
            Self::Internal(ref msg) => {
                tracing::error!(error = %msg, "Internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "ERR-INTERNAL", "Internal server error", None)
            }
        };

        let body = json!({
            "error": code,
            "message": message,
            "details": details,
        });

        (status, Json(body)).into_response()
    }
}
```

### Conversion Chain

```
DomainError ──From──→ ApiError
InfraError  ──From──→ ApiError (always maps to Internal, error details logged but not exposed)
```

All `From` impls are exhaustive (no wildcard arms). Adding a new `DomainError` variant forces updating the `ApiError` conversion — the compiler ensures completeness.

## Consequences

### Positive
- Every error code in the API spec has exactly one corresponding Rust enum variant
- Adding a new error to the domain forces handling it in the API layer (compiler-enforced)
- Infrastructure errors never leak to clients (always converted to `Internal`)
- Error types serve as documentation of all possible failure modes
- Structured logging captures the original error for debugging

### Negative
- More boilerplate than `anyhow::Error` or a single error enum
- `From` impls must be maintained manually (thiserror helps but doesn't auto-map across layers)
- Three-layer structure may feel heavy for simple CRUD operations

### Risks
- Error variant explosion as features grow → Mitigate by grouping related errors and using structured variants

## Alternatives Considered

### Alternative 1: anyhow::Error everywhere
- **Pros**: Zero boilerplate, any error type works
- **Cons**: No compile-time exhaustiveness, error codes must be attached ad-hoc, easy to forget mapping
- **Rejection Reason**: Fundamentally incompatible with "every error code maps to one variant" requirement

### Alternative 2: Single flat error enum
- **Pros**: Simpler than three layers
- **Cons**: Mixes domain, infra, and API concerns; DB errors visible in HTTP types
- **Rejection Reason**: Violates hexagonal architecture (domain must not depend on HTTP types)
