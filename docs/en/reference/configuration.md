# Configuration Reference

> **Navigation**: [Docs Home](../README.md) > [Reference](README.md) > Configuration

Complete reference for all environment variables used by the VRC Web-Backend. Variables are read at startup. The server will refuse to start if any required variable is missing or fails validation.

---

## Required Variables

### Server

| Variable | Type | Validation | Description |
|----------|------|------------|-------------|
| `BACKEND_BASE_URL` | string | Valid URL with scheme (e.g., `https://api.example.com`) | Base URL of the backend, used for constructing OAuth2 redirect URIs and absolute URLs |
| `FRONTEND_ORIGIN` | string | Valid URL origin (e.g., `https://example.com`) | Frontend origin for CORS `Access-Control-Allow-Origin` and CSRF `Origin` validation |

### Database

| Variable | Type | Validation | Description |
|----------|------|------------|-------------|
| `DATABASE_URL` | string | Valid PostgreSQL connection string (`postgres://...`) | PostgreSQL database connection URL |

### Discord OAuth2

| Variable | Type | Validation | Description |
|----------|------|------------|-------------|
| `DISCORD_CLIENT_ID` | string | Non-empty | Discord application client ID from the Developer Portal |
| `DISCORD_CLIENT_SECRET` | string | Non-empty | Discord application client secret |
| `DISCORD_GUILD_ID` | string | Non-empty; valid Discord snowflake | Discord server (guild) ID whose members are allowed to authenticate |

### Security

| Variable | Type | Validation | Description |
|----------|------|------------|-------------|
| `SESSION_SECRET` | string | Minimum 32 characters | Secret used for session token generation and validation |
| `SYSTEM_API_TOKEN` | string | Minimum 64 characters | Bearer token for System API and metrics endpoint authentication |

---

## Optional Variables

### Server

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `BIND_ADDRESS` | string | `0.0.0.0:3000` | Valid socket address (`host:port`) | Address and port the HTTP server binds to |
| `RUST_LOG` | string | `info` | Valid `tracing` filter directive | Log level filter (e.g., `debug`, `vrc_backend=debug,tower_http=trace`) |

### Database

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `DATABASE_MAX_CONNECTIONS` | integer | `10` | 1–100 | Maximum number of connections in the database connection pool |

### Session

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `SESSION_MAX_AGE_SECS` | integer | `604800` (7 days) | ≥ 60 | Maximum session lifetime in seconds |
| `SESSION_CLEANUP_INTERVAL_SECS` | integer | `3600` (1 hour) | ≥ 60 | Interval between expired session cleanup runs |

### Background Tasks

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `EVENT_ARCHIVAL_INTERVAL_SECS` | integer | `86400` (24 hours) | ≥ 60 | Interval between event archival task runs |

### Security

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `COOKIE_SECURE` | boolean | `false` | `true` or `false` | Set `Secure` flag on session cookies. **Must be `true` in production** (HTTPS only) |
| `TRUST_X_FORWARDED_FOR` | boolean | `false` | `true` or `false` | Trust `X-Forwarded-For` header for client IP extraction. Enable when behind a reverse proxy |

### Optional Features

| Variable | Type | Default | Validation | Description |
|----------|------|---------|------------|-------------|
| `SUPER_ADMIN_DISCORD_ID` | string | — | Valid Discord snowflake | Discord ID of the user to automatically grant `super_admin` role on login |
| `DISCORD_WEBHOOK_URL` | string | — | Valid HTTPS URL | Discord webhook URL for sending admin notifications |

---

## Example `.env` File

```dotenv
# === Required ===

# Server
BACKEND_BASE_URL=http://localhost:3000
FRONTEND_ORIGIN=http://localhost:5173

# Database
DATABASE_URL=postgres://vrc:vrc_password@localhost:5432/vrc_db

# Discord
DISCORD_CLIENT_ID=123456789012345678
DISCORD_CLIENT_SECRET=your_client_secret_here
DISCORD_GUILD_ID=987654321098765432

# Security
SESSION_SECRET=your_session_secret_at_least_32_characters_long
SYSTEM_API_TOKEN=your_system_api_token_at_least_64_characters_long_generate_with_openssl_rand_hex_64

# === Optional ===

# BIND_ADDRESS=0.0.0.0:3000
# DATABASE_MAX_CONNECTIONS=10
# SESSION_MAX_AGE_SECS=604800
# SESSION_CLEANUP_INTERVAL_SECS=3600
# EVENT_ARCHIVAL_INTERVAL_SECS=86400
# COOKIE_SECURE=false
# TRUST_X_FORWARDED_FOR=false
# SUPER_ADMIN_DISCORD_ID=
# DISCORD_WEBHOOK_URL=
# RUST_LOG=info
```

---

## Production Checklist

| Check | Variable | Value |
|-------|----------|-------|
| ✅ HTTPS enabled | `COOKIE_SECURE` | `true` |
| ✅ Behind reverse proxy | `TRUST_X_FORWARDED_FOR` | `true` |
| ✅ Strong session secret | `SESSION_SECRET` | ≥ 32 chars, generated with CSPRNG |
| ✅ Strong API token | `SYSTEM_API_TOKEN` | ≥ 64 chars, generated with CSPRNG |
| ✅ Correct frontend origin | `FRONTEND_ORIGIN` | Production frontend URL (no trailing slash) |
| ✅ Super admin configured | `SUPER_ADMIN_DISCORD_ID` | Discord ID of the initial admin |

---

## Related Documents

- [Environment Quick Reference](environment.md) — Compact grouped table
- [Quickstart](../getting-started/quickstart.md) — Initial environment setup
- [Auth API](api/auth.md) — How session and cookie settings affect authentication
