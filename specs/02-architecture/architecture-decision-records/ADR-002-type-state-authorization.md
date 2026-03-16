# ADR-002: Type-State Authorization

## Status
Accepted

## Context

The system has four roles: `member`, `staff`, `admin`, `super_admin`. Different API endpoints require different minimum roles. Traditional approach: runtime check `if user.role >= required_role { ... } else { return 403 }`. This works but:

1. Forgetting a role check compiles and silently becomes a security hole
2. The type signature of a handler does not communicate its permission requirements
3. Tests must explicitly verify role checks for every endpoint

We want: **If a handler requires Admin, it should be impossible to call it with a Member — at compile time.**

## Decision

Implement **Type-State Authorization** using Rust phantom types:

```rust
// Role marker types (zero-sized, no runtime cost)
pub struct Member;
pub struct Staff;
pub struct Admin;
pub struct SuperAdmin;

// Sealed trait — only our marker types can implement it
pub trait Role: Send + Sync + 'static {
    const NAME: &'static str;
    const LEVEL: u8;
}

impl Role for Member     { const NAME: &'static str = "member";      const LEVEL: u8 = 0; }
impl Role for Staff      { const NAME: &'static str = "staff";       const LEVEL: u8 = 1; }
impl Role for Admin      { const NAME: &'static str = "admin";       const LEVEL: u8 = 2; }
impl Role for SuperAdmin { const NAME: &'static str = "super_admin"; const LEVEL: u8 = 3; }

// Compile-time role hierarchy: "R satisfies minimum role Min"
pub trait SatisfiesRole<Min: Role>: Role {}
impl SatisfiesRole<Member> for Member {}
impl SatisfiesRole<Member> for Staff {}
impl SatisfiesRole<Member> for Admin {}
impl SatisfiesRole<Member> for SuperAdmin {}
impl SatisfiesRole<Staff> for Staff {}
impl SatisfiesRole<Staff> for Admin {}
impl SatisfiesRole<Staff> for SuperAdmin {}
impl SatisfiesRole<Admin> for Admin {}
impl SatisfiesRole<Admin> for SuperAdmin {}
impl SatisfiesRole<SuperAdmin> for SuperAdmin {}

// The extractor: AuthenticatedUser<R> only succeeds if session user's role >= R
pub struct AuthenticatedUser<R: Role> {
    pub user_id: UserId,
    pub discord_id: DiscordId,
    pub role: UserRole,      // Still available at runtime for dynamic checks
    _phantom: PhantomData<R>,
}

// Axum extractor: at runtime, resolves session → user → verifies role level
#[async_trait]
impl<R: Role> FromRequestParts<AppState> for AuthenticatedUser<R> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let session_id = extract_session_cookie(parts)?;
        let session = state.session_repo.find_valid(session_id).await?;
        let user = state.user_repo.find_by_id(session.user_id).await?;

        if user.status == UserStatus::Suspended {
            return Err(ApiError::AccountSuspended);
        }

        if user.role.level() < R::LEVEL {
            return Err(ApiError::InsufficientRole {
                required: R::NAME,
                actual: user.role.as_str(),
            });
        }

        Ok(AuthenticatedUser {
            user_id: user.id,
            discord_id: user.discord_id,
            role: user.role,
            _phantom: PhantomData,
        })
    }
}
```

### Usage in Handlers

```rust
// This handler ONLY accepts Staff or above — enforced by the type system
async fn create_club(
    user: AuthenticatedUser<Staff>,  // Compile-time: must be Staff+
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateClubRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // `user` is guaranteed to be Staff+ — no runtime role check needed here
    let club = state.club_use_case.create(user.user_id, body).await?;
    Ok((StatusCode::CREATED, Json(club)))
}

// This handler accepts any logged-in user
async fn get_my_profile(
    user: AuthenticatedUser<Member>,  // Member = minimum role = any authenticated user
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let profile = state.profile_use_case.get_own(user.user_id).await?;
    Ok(Json(profile))
}
```

### Role Change Security (Runtime Layer)

For role changes (`PATCH /admin/users/{id}/role`), the hierarchical constraints (e.g., "only super_admin can grant admin") use **runtime** checks because they depend on both the caller's role and the target's current role — this cannot be fully encoded at compile time:

```rust
async fn change_role(
    caller: AuthenticatedUser<Admin>,  // Compile-time: must be Admin+
    State(state): State<AppState>,
    Path(target_id): Path<UserId>,
    ValidatedJson(body): ValidatedJson<ChangeRoleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Runtime: additional business rules
    state.admin_use_case.change_role(caller.role, target_id, body.new_role).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Consequences

### Positive
- Forgetting a role check is a **compile error**, not a security vulnerability
- Handler signatures are self-documenting: `AuthenticatedUser<Admin>` immediately tells you the permission level
- Zero runtime cost for the phantom type (erased at monomorphization)
- IDE autocomplete shows which role is required
- Adding a new role propagates type errors to all handlers that need updating

### Negative
- More complex type signatures (mitigated by type aliases)
- Monomorphization may increase binary size slightly (one version of the extractor per role level)
- Dynamic role checks still needed for role-change business rules (hybrid approach)

### Risks
- Axum's `FromRequestParts` trait bounds with generics may complicate error types → Mitigate with a unified `ApiError` rejection type

## Alternatives Considered

### Alternative 1: Runtime-only role checks (middleware or per-handler)
- **Pros**: Simpler, no generics needed
- **Cons**: Forgetting a check = silent security hole; handler signatures don't show permission requirements
- **Rejection Reason**: Sacrifices the core "compile-time safety" goal

### Alternative 2: Attribute macro `#[require_role(Admin)]`
- **Pros**: Less typing than generic parameter, reads like a decorator
- **Cons**: Hides the mechanism (proc macro magic), harder to debug, less composable
- **Rejection Reason**: Could be added later as syntactic sugar over the type-state approach, but the extractor approach is more transparent and idiomatic Rust
