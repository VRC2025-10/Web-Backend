# Constraints

## Technical Constraints

| ID | Constraint | Rationale |
|----|-----------|-----------|
| TC-001 | Backend must run as a single Docker container + PostgreSQL container | Deployment simplicity; runs on Proxmox VM |
| TC-002 | Backend must be stateless across restarts (all state in PostgreSQL) | Docker containers are ephemeral |
| TC-003 | Gallery images stored on local filesystem (Docker volume); served by Caddy | No external cloud storage dependency; images co-located with the server |
| TC-004 | Discord OAuth2 is the only authentication method | Community is Discord-native; no need for email/password |
| TC-005 | System API uses a single static Bearer token (not JWT, not OAuth2 client credentials) | Only two consumers (GAS + Bot); token rotation is manual but infrequent |
| TC-006 | Backend does not embed a Discord gateway connection | Bot is a separate process; backend only calls Discord REST API for guild membership verification |
| TC-007 | SSL/TLS termination is external (reverse proxy) | Backend listens on HTTP only; simplifies Rust code |
| TC-008 | Compile-time SQL verification requires a PostgreSQL instance during `cargo build` | SQLx offline mode (`sqlx-data.json`) used in CI; live DB used in local development |

## Business Constraints

| ID | Constraint | Rationale |
|----|-----------|-----------|
| BC-001 | Zero operational cost is preferred; infrastructure budget is minimal (≤ $10/month) | Community project, no revenue |
| BC-002 | All user-facing text and error messages should be understandable to Japanese VRChat users | Primary audience is Japanese-speaking |
| BC-003 | GDPR-like data deletion is expected (profile can be made private; account can be suspended) | Respect user agency even without legal obligation |

## Regulatory Constraints

| ID | Constraint | Rationale |
|----|-----------|-----------|
| RC-001 | No PII beyond Discord-provided data (username, avatar, Discord ID) and user-submitted profile | Minimize data collection surface |
| RC-002 | Bio Markdown must be sanitized against XSS before storage and rendering | User-generated content displayed to other users |
