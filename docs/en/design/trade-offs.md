# Trade-offs

> **Audience**: Architects, Contributors
>
> **Navigation**: [Docs Home](../README.md) > [Design](README.md) > Trade-offs

## Overview

Every design decision involves trade-offs. This document records the key trade-offs made in the VRC Web-Backend, why each choice was made, and under what conditions we would reconsider.

---

## 1. Rust Over Go/TypeScript

| Aspect | Detail |
|--------|--------|
| **Decision** | Use Rust with Axum as the backend language and framework |
| **Alternatives Considered** | Go (Gin/Echo), TypeScript (Express/Fastify), Python (FastAPI) |

### Why This Choice

- **Compile-time safety**: Ownership, lifetimes, and the type system prevent entire classes of bugs (null pointers, data races, use-after-free)
- **Educational value**: Aligns with the "Romance Through Rigor" philosophy — learning Rust teaches deep systems understanding
- **Performance**: Zero-cost abstractions, no garbage collector, predictable latency
- **Type system expressiveness**: Enables patterns like type-state authorization that are impossible or impractical in other languages

### Consequences

| Positive | Negative |
|----------|----------|
| Memory safety without GC | Steeper learning curve for contributors |
| Compile-time bug prevention | Longer compile times (~2-3 min full build) |
| Excellent performance characteristics | Smaller ecosystem than Node.js/Go |
| Forces explicit error handling | More verbose code for simple operations |
| Educational value for contributors | Harder to find Rust developers |

### When to Reconsider

- If the project needs many contributors quickly with minimal onboarding time
- If compile times become a serious bottleneck (mitigated by `cargo-chef` caching)
- If a critical library is unavailable in Rust but readily available elsewhere

---

## 2. Hexagonal Architecture Over Simple Layered

| Aspect | Detail |
|--------|--------|
| **Decision** | Hexagonal architecture with explicit port traits and adapter implementations |
| **Alternatives Considered** | Simple 3-layer (handler → service → repository), MVC, vertical slices |

### Why This Choice

- **Clean domain isolation**: Domain types have zero external dependencies
- **Testability**: Domain logic testable with mock implementations, no database needed
- **Enforced boundaries**: Port traits create hard boundaries between layers
- **Educational value**: Teaches architectural thinking and SOLID principles

### Consequences

| Positive | Negative |
|----------|----------|
| Pure, testable domain core | More boilerplate (traits + impls) |
| Swappable infrastructure | More files and indirection |
| Clear dependency direction | Overkill for simple CRUD operations |
| Self-documenting boundaries | Learning curve for the pattern itself |

### When to Reconsider

- If the application becomes predominantly CRUD with little domain logic
- If the team finds the abstraction layers consistently unnecessary
- If a simpler architecture would reduce development velocity significantly

### Related ADR

- [ADR-0001: Hexagonal Architecture](adr/0001-hexagonal-architecture.md)

---

## 3. Type-State Authorization Over Runtime Checks

| Aspect | Detail |
|--------|--------|
| **Decision** | Phantom type parameters on `AuthenticatedUser<R: Role>` for compile-time role enforcement |
| **Alternatives Considered** | Runtime role checks in handlers, middleware-based RBAC, attribute-based access control |

### Why This Choice

- **Compile-time enforcement**: Passing `AuthenticatedUser<Member>` to an admin endpoint is a compile error
- **Self-documenting**: Handler signatures declare required roles explicitly
- **No forgotten checks**: Can't accidentally omit a role check — the type system enforces it

### Consequences

| Positive | Negative |
|----------|----------|
| Unauthorized access is a compile error | Complex generic bounds in function signatures |
| Handler signatures show required roles | Harder for newcomers to understand |
| No runtime overhead for role checks | Inflexible for dynamic permission changes |
| Zero chance of forgotten role checks | Requires extractor per role |

### When to Reconsider

- If role requirements become highly dynamic (e.g., per-resource permissions)
- If the role hierarchy becomes deep enough that phantom types create combinatorial explosion
- If the team consistently finds the generic bounds confusing

### Related ADR

- [ADR-0002: Type-State Authorization](adr/0002-type-state-authorization.md)

---

## 4. SQLx Compile-Time Verification Over ORM

| Aspect | Detail |
|--------|--------|
| **Decision** | Use SQLx with compile-time query verification and offline mode |
| **Alternatives Considered** | Diesel (Rust ORM), SeaORM, raw SQL strings without verification |

### Why This Choice

- **Compile-time verification**: SQL syntax, column existence, and type mapping are checked at build time
- **Full SQL control**: No ORM abstraction layer hiding query behavior
- **No runtime surprises**: Schema mismatches are caught before deployment
- **Offline mode**: `sqlx-data.json` allows CI builds without a running database

### Consequences

| Positive | Negative |
|----------|----------|
| SQL errors caught at build time | Requires running PostgreSQL during development |
| Full control over queries | No automatic migration generation |
| Type-safe result mapping | Manual `sqlx-data.json` refresh when queries change |
| No ORM overhead | More manual SQL than ORM-based approaches |

### When to Reconsider

- If the number of queries grows so large that compile-time verification significantly slows builds
- If an ORM provides compile-time verification comparable to SQLx
- If the team finds writing raw SQL consistently error-prone

### Related ADR

- [ADR-0003: Compile-Time SQL Verification](adr/0003-compile-time-sql.md)

---

## 5. Single-Node Deployment Over Distributed

| Aspect | Detail |
|--------|--------|
| **Decision** | Single Proxmox VM with Docker Compose (app + PostgreSQL + Caddy) |
| **Alternatives Considered** | Kubernetes, Docker Swarm, multi-node with load balancer, serverless |

### Why This Choice

- **Community size**: ~50-300 members — a single node handles this trivially
- **Operational simplicity**: One VM to manage, one Docker Compose file, one backup target
- **Cost efficiency**: Single VM on existing Proxmox infrastructure
- **Reduced failure modes**: No distributed system issues (network partitions, consensus, split-brain)

### Consequences

| Positive | Negative |
|----------|----------|
| Simple operations | Single point of failure |
| Low cost | Vertical scaling only |
| Fast deployment | No geographic distribution |
| Easy debugging | Downtime during upgrades |
| Simple backup strategy | Cannot handle massive traffic spikes |

### When to Reconsider

- If community grows beyond ~1,000 concurrent users
- If uptime SLA requires >99.9% availability
- If geographic distribution is needed for latency
- If the single VM's resources become a bottleneck

---

## 6. Discord-Only Authentication Over Email/Password

| Aspect | Detail |
|--------|--------|
| **Decision** | Discord OAuth2 as the sole authentication method |
| **Alternatives Considered** | Email/password with optional Discord linking, multiple OAuth2 providers, magic links |

### Why This Choice

- **Community-native**: The VRChat community already lives on Discord — every member has an account
- **No password management**: No password hashing, reset flows, or credential stuffing risks
- **Guild verification**: OAuth2 guilds endpoint verifies the user is actually in the Discord server
- **Simplified auth flow**: Single provider means single auth flow to maintain

### Consequences

| Positive | Negative |
|----------|----------|
| Zero password management | Hard dependency on Discord's uptime |
| Natural community verification | Excludes users without Discord |
| Simplified implementation | Subject to Discord API rate limits |
| Reduced attack surface | No fallback authentication method |
| Users don't need new credentials | Discord TOS changes could impact us |

### When to Reconsider

- If Discord has major outages that affect the community
- If the community expands beyond Discord users
- If Discord changes its OAuth2 API or pricing significantly
- If users request alternative login methods

### Related ADR

- [ADR-0006: Discord-Only Authentication](adr/0006-discord-only-authentication.md)

---

## 7. Server-Side Sessions Over JWT

| Aspect | Detail |
|--------|--------|
| **Decision** | Server-side sessions with SHA-256 hashed tokens stored in PostgreSQL |
| **Alternatives Considered** | JWT (stateless), JWT with refresh tokens, Redis-backed sessions |

### Why This Choice

- **Immediate revocation**: Deleting a session row instantly invalidates the token — no waiting for expiry
- **No token bloat**: Session ID is a small opaque token, not a large JWT
- **Server-side control**: Can enumerate all sessions, force logout, audit session activity
- **Simpler security model**: No JWT signature key rotation, no algorithm confusion attacks

### Consequences

| Positive | Negative |
|----------|----------|
| Instant session revocation | Database lookup on every request |
| Small token size | Doesn't scale horizontally without shared DB |
| Full server-side control | Slightly more latency per request |
| No JWT-specific vulnerabilities | Session table grows over time, needs cleanup |

### When to Reconsider

- If horizontal scaling requires stateless authentication
- If database session lookups become a performance bottleneck
- If moving to a microservice architecture where token-based auth is more natural
- Consider Redis as an intermediate step before full JWT migration

### Related ADR

- [ADR-0007: Server-Side Sessions over JWT](adr/0007-server-side-sessions.md)

---

## 8. Japanese Error Messages Over English

| Aspect | Detail |
|--------|--------|
| **Decision** | User-facing error messages are in Japanese |
| **Alternatives Considered** | English-only, i18n with message keys, bilingual responses |

### Why This Choice

- **Primary audience**: The VRChat October Class Reunion community is predominantly Japanese-speaking
- **Reduced complexity**: Single language eliminates i18n framework overhead
- **Better UX**: Native language reduces user confusion and support requests
- **Error codes for developers**: Machine-readable error codes (e.g., `ERR-AUTH-001`) are language-independent

### Consequences

| Positive | Negative |
|----------|----------|
| Better UX for primary audience | Non-Japanese speakers see untranslated messages |
| No i18n framework overhead | Adding additional languages requires retrofit |
| Simpler codebase | Error message testing requires Japanese knowledge |
| Error codes are universal | Documentation must clarify which messages are user-facing |

### When to Reconsider

- If the community becomes significantly multilingual
- If non-Japanese speakers report usability issues
- If an i18n library with minimal overhead becomes available
- If the community expands to international groups

### Related ADR

- [ADR-0008: Japanese Error Messages](adr/0008-japanese-error-messages.md)

---

## Trade-off Summary Matrix

| # | Decision | Primary Benefit | Primary Cost | Reversibility |
|---|----------|----------------|--------------|---------------|
| 1 | Rust over Go/TypeScript | Compile-time safety | Learning curve | Low (full rewrite) |
| 2 | Hexagonal over layered | Domain isolation | Boilerplate | Medium (gradual simplification) |
| 3 | Type-state over runtime | Compile-time roles | Complexity | Medium (can add runtime checks) |
| 4 | SQLx over ORM | Query verification | Needs live DB | Medium (can add ORM layer) |
| 5 | Single-node over distributed | Simplicity | SPOF | High (can distribute later) |
| 6 | Discord-only over email | Community-native | Vendor lock-in | Medium (can add providers) |
| 7 | Sessions over JWT | Instant revocation | DB per request | Medium (can migrate to JWT) |
| 8 | Japanese over English | UX for audience | Limited audience | High (can add i18n) |

## Related Documents

- [Design Principles](principles.md) — the principles that informed these trade-offs
- [Design Patterns](patterns.md) — how the chosen approaches are implemented
- [ADRs](adr/README.md) — detailed decision records
