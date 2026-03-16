# ADR-004: Custom Procedural Macros

## Status
Accepted

## Context

The hexagonal architecture generates significant boilerplate:
- Every route handler needs the same pattern: extract user → validate input → call use case → map response
- Permission annotations are repeated across handlers  
- OpenAPI documentation must be kept in sync with handler signatures

A procedural macro crate can eliminate this boilerplate while keeping the type-state system intact.

## Decision

Create a separate `vrc-macros` crate (`proc-macro = true`) with the following macros:

### 1. `#[derive(Validate)]` — Compile-time validation rules

```rust
#[derive(Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(max_length = 100)]
    pub vrc_id: Option<String>,

    #[validate(regex = r"^@?[a-zA-Z0-9_]{1,15}$")]
    pub x_id: Option<String>,

    #[validate(max_length = 5000, xss_check)]
    pub bio_markdown: String,

    pub is_public: bool,
}
```

Generates a `validate(&self) -> Result<(), ValidationDetails>` method at compile time.

### 2. `#[api_handler]` — Route handler boilerplate reduction

```rust
#[api_handler(
    method = POST,
    path = "/admin/clubs",
    role = Staff,
    rate_limit = "internal",
    summary = "Create a new club",
    response(201, CreateClubResponse),
    error(403, "ERR-PERM-001"),
)]
async fn create_club(
    user: AuthenticatedUser<Staff>,
    state: AppState,
    body: CreateClubRequest,
) -> Result<CreateClubResponse, ApiError> {
    state.club_use_case.create(user.user_id, body).await
}
```

Expands to: Axum handler with `ValidatedJson` extraction, error mapping, and OpenAPI metadata registration.

### 3. `#[derive(ErrorCode)]` — Error code string generation

```rust
#[derive(ErrorCode)]
pub enum ProfileError {
    #[code("ERR-PROF-001")] Validation(ValidationDetails),
    #[code("ERR-PROF-002")] BioDangerous,
    #[code("ERR-PROF-004")] NotFound,
}
```

Generates `fn error_code(&self) -> &'static str` and ensures code uniqueness at compile time.

## Consequences

### Positive
- Dramatically reduces boilerplate in handler definitions
- Validation rules are co-located with struct definitions
- Error codes are verified unique at compile time
- OpenAPI spec can be auto-generated from macro annotations
- Educational: writing proc macros is an advanced Rust skill

### Negative
- Proc macro debugging is notoriously difficult (`cargo expand` helps)
- Compile time increases with more proc macro usage
- Learning curve for contributors who need to modify macros

### Risks
- Proc macro crate becomes a bottleneck for incremental compilation → Mitigate by keeping macros focused and small
- IDE support for macro-generated code may be poor → `rust-analyzer` handles derive macros well; attribute macros less so

## Alternatives Considered

### Alternative 1: No macros, manual boilerplate
- **Pros**: Fully explicit, easy to debug, no magic
- **Cons**: Repetitive, error-prone, code drift between handler and spec
- **Rejection Reason**: Does not maximize difficulty; repetition is boring, macros are romantic

### Alternative 2: Use existing crates (utoipa, validator)
- **Pros**: Battle-tested, no custom code to maintain
- **Cons**: Less educational, less tailored to our exact needs
- **Rejection Reason**: Building our own is the point. Can fall back to these crates if time-constrained.
