# Correctness-by-Construction Patterns

The Rust implementation uses an 8-layer defense-in-depth strategy where each layer catches a different class of bugs. The key principle: **every verification technique operates on the actual implementation code** — no separate specification language.

## Defense Layers

```
Layer 1: Kani Bounded Model Checking     — exhaustive proof on actual Rust functions
Layer 2: Type-State Authorization         — role constraints enforced at compile time
Layer 3: Exhaustive Error Matching        — no unhandled error paths
Layer 4: SQLx Compile-Time Verification   — SQL correctness checked at build time
Layer 5: Newtype Wrappers                 — prevent semantic type confusion
Layer 6: Serde Strictness                 — reject malformed input at deserialization
Layer 7: Domain Validation                — business rule enforcement at runtime
Layer 8: Database Constraints             — final safety net at persistence layer
```

## 1. Kani Bounded Model Checking

Unlike TLA+ (which models a separate specification), Kani runs directly on the Rust source code. Proof harnesses are `#[cfg(kani)]` functions that live next to the code they verify.

```rust
// This proof harness IS the specification AND runs on the implementation.
// If validate_role_change is refactored, this proof automatically re-verifies.
#[cfg(kani)]
#[kani::proof]
fn proof_role_change_no_escalation() {
    let actor_role = any_role();
    let target_role = any_role();
    let new_role = any_role();

    if validate_role_change(actor_role, target_role, new_role).is_ok() {
        assert!(role_level(actor_role) >= role_level(new_role));
    }
}
```

**What this proves**: Within the bounded domain, every possible combination of (actor, target, new_role) either fails validation or satisfies the invariant. **The proof and the code are the same artifact.**

## 2. Type-State Authorization (ADR-002)

Authorization is not checked at runtime with `if` statements — it is enforced by the type system. A handler that requires `staff` role literally cannot compile if the extractor is missing or has the wrong role type.

```rust
// This compiles:
async fn create_club(auth: AuthenticatedUser<Staff>, ...) -> ... { }

// This does NOT compile — Member doesn't satisfy Staff:
// async fn create_club(auth: AuthenticatedUser<Member>, ...) -> ... { }
// Error: `Member` does not implement `SatisfiesRole<Staff>`
```

**What this proves**: Every endpoint has its minimum required role statically checked. Forgetting to add authentication to an endpoint is a compile error, not a runtime bug.

## 3. Exhaustive Error Matching (ADR-003)

All domain errors are enum variants. The `match` compiler enforces exhaustiveness:

```rust
impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::ProfileValidation(d) => ApiError::ProfileValidation(d),
            DomainError::BioDangerous => ApiError::BioDangerous,
            DomainError::SessionInvalid => ApiError::SessionInvalid,
            // ... every variant must be handled
            // Adding a new DomainError variant causes a compile error here
        }
    }
}
```

**What this proves**: Every error path is explicitly handled. No error is silently swallowed or generates a generic 500 without intentional mapping.

## 4. SQLx Compile-Time Query Verification

SQL queries are type-checked against the actual database schema at compile time:

```rust
let user = sqlx::query_as!(
    User,
    "SELECT id, discord_id, role as \"role: UserRole\" FROM users WHERE id = $1",
    user_id
)
.fetch_optional(&pool)
.await?;
```

**What this proves**:
- The table and columns exist
- Parameter types match (`$1` is compatible with `user_id`'s type)
- Return type matches `User` struct fields
- Column count and order are correct
- SQL syntax is valid

A schema change that breaks any query is a compile error.

## 5. Newtype Wrappers for Value Objects

Prevent mixing up semantically different values that share the same underlying type:

```rust
/// Discord user ID (snowflake, 17-20 digit string)
pub struct DiscordId(String);

/// Internal database user ID
pub struct UserId(i32);

/// Session token (raw, never stored in DB)
pub struct SessionToken([u8; 32]);

/// Session token hash (stored in DB)
pub struct SessionTokenHash([u8; 32]);
```

**What this proves**: You cannot accidentally store a raw session token in the database or use a token hash as a cookie value. The type system makes this category of bug impossible.

## 6. `#[serde(deny_unknown_fields)]`

Request bodies reject unexpected fields:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    // ...
}
```

**What this proves**: API consumers cannot sneak in extra fields (e.g., `"role": "super_admin"`) that might be accidentally deserialized into domain objects.

## 7. Domain Validation

Runtime validation for business rules that cannot be expressed in the type system:

```rust
pub fn validate_vrc_id(id: &str) -> Result<(), DomainError> {
    let re = Regex::new(r"^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .unwrap();
    if !re.is_match(id) {
        return Err(DomainError::ProfileValidation(
            ValidationDetail::vrc_id("Must match usr_{uuid} format (lowercase)")
        ));
    }
    Ok(())
}
```

**What this proves**: Invalid data is rejected before it reaches the database. Combined with proptest, we generate thousands of random inputs to verify this function never accepts invalid data.

## 8. Database Constraints

The final safety net — even if all application-level checks fail:

```sql
ALTER TABLE users ADD CONSTRAINT valid_role
    CHECK (role IN ('member', 'staff', 'admin', 'super_admin'));

ALTER TABLE profiles ADD CONSTRAINT nickname_length
    CHECK (LENGTH(nickname) BETWEEN 1 AND 50);

ALTER TABLE sessions ADD CONSTRAINT future_expiry
    CHECK (expires_at > created_at);
```

**What this proves**: Even a bug in the application code cannot persist invalid data. The database rejects it with a constraint violation error.

## Summary: No Specification-Implementation Gap

Every verification technique in this project operates on **the same code that runs in production**:

| Layer | Operates On | Gap Risk |
|-------|-------------|----------|
| Kani | Actual Rust functions | **Zero** — same binary |
| Type-State | Actual handler signatures | **Zero** — compile time |
| Error Matching | Actual enum variants | **Zero** — compile time |
| SQLx | Actual SQL strings vs actual schema | **Zero** — compile time |
| Newtypes | Actual type definitions | **Zero** — compile time |
| Serde | Actual struct definitions | **Zero** — compile time |
| Domain validation | Actual validation functions | **Zero** — tested by proptest on same code |
| DB constraints | Actual DDL in migrations | **Zero** — applied to production DB |
