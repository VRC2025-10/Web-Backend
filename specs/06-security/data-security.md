# Data Security

## Data Classification

| Category | Data Types | Sensitivity | Retention |
|----------|-----------|-------------|-----------|
| **Identity** | `discord_id`, `discord_display_name`, `discord_avatar_hash` | PII (Discord Public) | Until user leaves + 30 days |
| **Profile** | `nickname`, `vrc_id`, `x_id`, `bio_markdown`, `avatar_url` | PII (User-Provided) | Until user deletes or leaves + 30 days |
| **Session** | `token_hash`, `expires_at` | Authentication | Max 7 days (TTL) |
| **Content** | Events, clubs, gallery images, reports | Internal | Indefinite |
| **Operational** | Logs, metrics, traces | Operational | 30 days (logs), 90 days (metrics) |

## Encryption

### At Rest

| Layer | Mechanism | Details |
|-------|-----------|---------|
| Disk | Docker volume encryption | Relies on host OS disk encryption (e.g., LUKS, BitLocker) |
| Database | PostgreSQL TDE | Not enabled by default; use `pgcrypto` for field-level if needed |
| Session tokens | SHA-256 hash | Stored as one-way hash; raw token only in cookie |
| Secrets | Environment variables | Never stored in code, config files, or database |

### In Transit

| Path | Protocol | Configuration |
|------|----------|---------------|
| Browser → Caddy | TLS 1.2+ (Caddy auto-HTTPS) | Automatic certificate via Let's Encrypt or ACME |
| Caddy → Axum | HTTP (localhost) | Same machine; Docker internal network |
| Axum → PostgreSQL | TCP (localhost) | Same Docker network; optional `sslmode=require` for remote DB |
| Axum → Discord API | HTTPS (TLS 1.2+) | System root CA bundle via `reqwest` |

### Caddy → Axum (Plaintext) Justification

Caddy and Axum run in the same Docker Compose network. Traffic between them never leaves the Docker bridge. Adding TLS between them would:
- Add latency (~1ms per request for TLS handshake)
- Require certificate management for an internal service
- Provide no security benefit (the Docker bridge is not exposed)

If the architecture changes to separate hosts, add mTLS between Caddy and Axum.

## Secrets Management

| Secret | Source | Rotation Policy |
|--------|--------|-----------------|
| `DATABASE_URL` | Environment variable | On credential change |
| `DISCORD_CLIENT_ID` | Environment variable | Fixed (Discord app) |
| `DISCORD_CLIENT_SECRET` | Environment variable | On compromise |
| `SESSION_SECRET` | Environment variable (≥32 bytes, base64) | Quarterly or on compromise |
| `SYSTEM_API_TOKEN` | Environment variable (≥32 bytes, hex) | Quarterly or on compromise |
| `WEBHOOK_URL` | Environment variable | On channel change |

### Secret Hygiene Rules

1. **Never** commit secrets to version control (`.env` in `.gitignore`)
2. **Never** log secrets (sensitive fields excluded from `tracing` spans)
3. **Never** return secrets in API responses
4. Use `secrecy` crate's `SecretString` type for in-memory secret storage (zeroize on drop)
5. Docker Compose: use `env_file` directive pointing to `.env` (not inline `environment:`)

```rust
use secrecy::{ExposeSecret, SecretString};

pub struct AppConfig {
    pub database_url: SecretString,
    pub discord_client_secret: SecretString,
    pub session_secret: SecretString,
    pub system_api_token: SecretString,
    // Non-secret fields
    pub discord_client_id: String,
    pub discord_guild_id: String,
    pub frontend_origin: String,
}
```

## PII Handling

### Collection Minimization

We collect only:
- Discord public profile data (username, avatar, guild membership) — required for authentication
- User-provided profile data (nickname, VRC ID, X handle, bio) — voluntarily provided
- No email addresses, phone numbers, real names, or location data

### Access Control

| PII Field | Public API | Internal API (self) | Internal API (admin) |
|-----------|-----------|-------------------|---------------------|
| `discord_id` | ✅ (if public profile) | ✅ | ✅ |
| `discord_display_name` | ✅ (if public profile) | ✅ | ✅ |
| `nickname` | ✅ (if public profile) | ✅ | ❌ (not in admin user list) |
| `vrc_id` | ✅ (if public profile) | ✅ | ❌ |
| `x_id` | ✅ (if public profile) | ✅ | ❌ |
| `bio_markdown` | ❌ (only `bio_html`) | ✅ | ❌ |

### Right to Deletion

When a user leaves the Discord server (via System API `/sync/users/leave`):
1. User status → `suspended`
2. Profile → `is_public = false` (hidden from public, data retained for 30 days)
3. Sessions → all deleted (immediate revocation)
4. Club memberships → removed
5. After 30 days: background job permanently deletes profile data (future implementation, tracked as out-of-scope for MVP)

### Data Breach Response

If a database breach is detected:
1. Rotate all secrets (`SESSION_SECRET`, `SYSTEM_API_TOKEN`, `DISCORD_CLIENT_SECRET`)
2. Invalidate all sessions (`TRUNCATE sessions`)
3. Notify all users via Discord (since we have no email)
4. Assess exposed data scope using audit logs
5. Report to relevant authorities if required (GDPR: 72 hours)
