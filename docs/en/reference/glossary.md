# Glossary

> **Navigation**: [Docs Home](../README.md) > [Reference](README.md) > Glossary

Domain terms, abbreviations, and technical concepts used in the VRC Web-Backend project.

---

## Domain Terms

| Term | Japanese | Definition |
|------|----------|------------|
| **Class Reunion** | 同期会 | The VRChat community that this application serves. Members share a common entry period into VRChat. |
| **Member** | メンバー | A Discord guild member who has authenticated via the application. Represented as a user record in the database. |
| **Profile** | プロフィール | A member's self-managed public or private profile containing nickname, VRChat ID, bio, and avatar. |
| **Club** | 部活動 | A community-organized interest group (e.g., photography, music). Managed by Staff+. |
| **Gallery** | ギャラリー | A collection of images belonging to a club. Images go through a moderation workflow (pending → approved/rejected). |
| **Event** | イベント | A scheduled community activity (meetup, game night, etc.). Synced from external sources via the System API. |
| **Event Tag** | イベントタグ | A label attached to an event for categorization (e.g., `meetup`, `weekly`, `game-night`). |
| **Report** | 通報 | A user-submitted complaint about inappropriate content or behavior, reviewed by Staff+. |
| **Guild** | ギルド / サーバー | A Discord server. The application is scoped to a single guild identified by `DISCORD_GUILD_ID`. |

## API Categories

| Term | Definition |
|------|------------|
| **Public API** | Read-only endpoints accessible without authentication. Rate limited per IP (60/min). |
| **Internal API** | Endpoints for authenticated users (session cookie). Rate limited per user (120/min). |
| **Admin API** | Subset of Internal API requiring Staff+ or Admin+ role. Nested under `/api/v1/internal/admin`. |
| **System API** | Machine-to-machine endpoints using Bearer token auth. Rate limited globally (30/min). |
| **Auth API** | Discord OAuth2 login and callback endpoints. Rate limited per IP (10/min). |
| **GAS** | Google Apps Script — one of the external systems that may call the System API to sync event data. |

## Roles & Authorization

| Term | Definition |
|------|------------|
| **RBAC** | Role-Based Access Control. The authorization model used by this application. |
| **Role** | One of four permission levels: `member` (0), `staff` (1), `admin` (2), `super_admin` (3). |
| **Role Level** | Numeric value associated with each role, used for authorization comparisons. |
| **member** | Default role (level 0). Can use Public and Internal APIs. |
| **staff** | Elevated role (level 1). Can manage reports, clubs, and gallery images. |
| **admin** | Administrative role (level 2). Can manage users, roles, and statuses. |
| **super_admin** | Highest role (level 3). Protected from demotion. Set via `SUPER_ADMIN_DISCORD_ID`. |

## Authentication & Session

| Term | Definition |
|------|------------|
| **Session** | Server-side session stored in the database, identified by an opaque token in an HTTP-only cookie. |
| **OAuth State** | A CSPRNG-generated token stored in the database during the OAuth2 flow to prevent CSRF attacks. Single-use, expires in 10 minutes. |
| **CSRF** | Cross-Site Request Forgery. Mitigated via `Origin` header validation on all state-mutating Internal API requests. |
| **CORS** | Cross-Origin Resource Sharing. Configured to allow requests only from `FRONTEND_ORIGIN`. |

## Architecture & Patterns

| Term | Definition |
|------|------------|
| **Hexagonal Architecture** | Also known as Ports and Adapters. The application separates domain logic from infrastructure via trait-based boundaries. |
| **Port** | A trait (interface) defining how the domain interacts with external systems (e.g., `UserRepository`, `SessionStore`). |
| **Adapter** | A concrete implementation of a port (e.g., `PgUserRepository` implementing `UserRepository` for PostgreSQL). |
| **Type-State Pattern** | A Rust compile-time pattern where state transitions are encoded in the type system, preventing invalid states. Used in builders and request processing. |
| **Tower Layer** | Middleware in the Tower/Axum ecosystem. Used for authentication, rate limiting, CORS, and logging. |
| **Error Algebra** | The pattern of composing domain-specific error types into a unified error hierarchy using Rust enums, enabling exhaustive matching. |
| **Compile-Time Verification** | Leveraging Rust's type system and tools like `sqlx` compile-time query checking to catch errors before runtime. |

## Infrastructure

| Term | Definition |
|------|------------|
| **Axum** | The Rust web framework used for HTTP routing and request handling. Built on Tower and Hyper. |
| **SQLx** | Async Rust SQL toolkit with compile-time query verification against a real database. |
| **Caddy** | Reverse proxy used in production for TLS termination and automatic HTTPS. |
| **Prometheus** | Monitoring system. The `/metrics` endpoint exposes application metrics in Prometheus text format. |

---

## Related Documents

- [API Overview](api/README.md) — API categories and authentication methods
- [Admin API](api/admin.md) — Role requirements and authorization matrix
- [Configuration Reference](configuration.md) — Environment variable definitions
- [Error Catalog](errors.md) — Error code reference
