# Design Principles

> **Audience**: All
>
> **Navigation**: [Docs Home](../README.md) > [Design](README.md) > Principles

## Overview

These principles guide every design and implementation decision in the VRC Web-Backend. They are listed in order of priority — when principles conflict, higher-ranked ones take precedence.

---

## 1. "Romance Through Rigor"

**Choose the hardest viable path when it maximizes learning and correctness.**

This project exists at the intersection of production software and education. We don't choose Rust, hexagonal architecture, or formal verification because they're easy — we choose them because the struggle of implementing them correctly produces deep understanding and robust software.

"Viable" is the key qualifier. The path must lead to working, maintainable software. Complexity that doesn't serve correctness or learning is rejected.

**In practice:**
- Rust over Go/TypeScript — the borrow checker teaches ownership and lifetime reasoning
- Type-state authorization over runtime role checks — phantom types teach advanced type system usage
- Kani proofs over "trust me" comments — formal verification teaches mathematical reasoning about code

---

## 2. Type Safety Over Runtime Checks

**If the type system can prevent a bug, it must.**

Runtime checks are last resorts. The goal is to make invalid states unrepresentable at compile time. When you see a value of type `AuthenticatedUser<Admin>`, you know — without checking any runtime state — that this request comes from a verified admin.

**In practice:**
- `AuthenticatedUser<R: Role>` — role enforcement via phantom types

  ```rust
  // This function ONLY compiles if called with an Admin user.
  // No runtime role check needed.
  async fn admin_action(user: AuthenticatedUser<Admin>) -> Result<...> {
      // ...
  }
  ```

- `ValidatedJson<T>` — input validation runs before the handler sees data
- `NonEmpty<T>`, bounded integers — domain invariants encoded in types

---

## 3. Compile-Time Verification Wherever Possible

**Push error detection as far left as possible — ideally to `cargo build`.**

Every error caught at compile time is an error that can never reach production. We use the Rust compiler, SQLx's offline mode, and custom derive macros to verify correctness before the binary is produced.

**In practice:**
- SQLx compile-time query verification — SQL syntax errors, type mismatches, and missing columns are caught during build

  ```rust
  // This won't compile if the SQL is invalid or types don't match
  let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
      .fetch_one(&pool)
      .await?;
  ```

- `#[derive(Validate)]` — field validation rules are checked at macro expansion time
- `#[derive(ErrorCode)]` — error code derivation ensures every variant has a code

---

## 4. Defense in Depth

**No single security mechanism is trusted alone. Every layer assumes the previous one failed.**

Security is implemented in overlapping layers. XSS prevention doesn't just rely on sanitization — it adds a post-sanitize detection check. Session tokens aren't just compared — they're hashed first, then compared in constant time.

**In practice:**
- XSS prevention (3 layers):
  1. Markdown rendering via `pulldown-cmark` (controlled HTML generation)
  2. HTML sanitization via `ammonia` (allowlist-based tag/attribute filtering)
  3. Defense-in-depth `<script>` detection (catches sanitizer bypasses)

- Token comparison (2 layers):
  1. SHA-256 hashing (prevents timing leaks from variable-length comparison)
  2. `subtle::ConstantTimeEq` (prevents timing leaks from byte-by-byte comparison)

- CSRF protection + SameSite cookies + Origin validation (multiple overlapping protections)

---

## 5. Hexagonal Purity

**The domain core has zero external dependencies. It knows nothing about HTTP, SQL, or Discord.**

Domain types and business logic live in a pure Rust core that depends only on the standard library. All external concerns — databases, HTTP frameworks, third-party APIs — are accessed through port traits that the domain defines but never implements.

**In practice:**
- Domain types (`User`, `Event`, `Session`) contain no framework annotations
- Repository traits (`UserRepository`, `EventRepository`) define data access contracts
- The domain can be tested with in-memory implementations — no database, no HTTP server

```
domain/           ← Pure Rust, no external deps
  ├── types/      ← User, Event, Role, etc.
  ├── ports/      ← Repository traits, service traits
  └── errors/     ← DomainError enum

adapters/         ← Framework-specific implementations
  ├── postgres/   ← PostgresUserRepository impl
  ├── http/       ← Axum handlers, extractors
  └── discord/    ← Discord API client
```

---

## 6. Error Exhaustiveness

**No catch-all error handling. Every error variant is explicitly mapped.**

We use algebraic error types (Rust enums) with explicit `From` conversions between layers. There is no `anyhow::Error`, no `Box<dyn Error>`, no catch-all `_` match arm for error mapping. Every domain error maps to a specific API error, which maps to a specific HTTP response.

**In practice:**
- `DomainError` → `ApiError` → HTTP response with specific status code and error code

  ```rust
  // Every DomainError variant has an explicit ApiError mapping
  impl From<DomainError> for ApiError {
      fn from(err: DomainError) -> Self {
          match err {
              DomainError::UserNotFound => ApiError::not_found("ERR-USR-001"),
              DomainError::UserSuspended => ApiError::forbidden("ERR-USR-002"),
              // No wildcard — adding a new DomainError variant forces a compiler error here
          }
      }
  }
  ```

- Adding a new error variant causes a compile error until all match arms are updated
- Error codes are unique and documented in the error reference

---

## 7. Property-Based Correctness

**Critical invariants are verified by proptest and Kani, not just example-based tests.**

Example-based tests prove that specific inputs produce expected outputs. Property-based tests prove that *classes* of inputs satisfy invariants. Kani proofs provide bounded mathematical guarantees about code correctness.

**In practice:**
- `proptest` — generates thousands of random inputs to test invariants

  ```rust
  proptest! {
      #[test]
      fn sanitized_bio_never_contains_script(input in ".*") {
          let sanitized = sanitize_bio(&input);
          assert!(!sanitized.contains("<script"));
      }
  }
  ```

- Kani bounded model checking — proves invariants hold for all inputs within bounds

  ```rust
  #[kani::proof]
  #[kani::unwind(5)]
  fn verify_role_hierarchy_is_transitive() {
      let a: Role = kani::any();
      let b: Role = kani::any();
      let c: Role = kani::any();
      if a.has_permission_of(b) && b.has_permission_of(c) {
          assert!(a.has_permission_of(c));
      }
  }
  ```

---

## Principle Interactions

| Scenario | Resolution |
|----------|------------|
| Type safety requires more boilerplate | Accept boilerplate — type safety wins (Principle 2 > convenience) |
| Compile-time check requires live DB | Use SQLx offline mode as compromise (Principle 3 adapted) |
| Defense in depth adds latency | Accept minor latency — security wins (Principle 4 > performance) |
| Hexagonal purity means more traits | Accept traits — domain purity wins (Principle 5 > brevity) |
| Error exhaustiveness means more match arms | Accept match arms — exhaustiveness wins (Principle 6 > terseness) |

## Related Documents

- [Design Patterns](patterns.md) — how these principles manifest in code
- [Trade-offs](trade-offs.md) — what we gave up to uphold these principles
- [ADRs](adr/README.md) — individual decisions that apply these principles
- [Architecture Overview](../architecture/README.md) — system structure
