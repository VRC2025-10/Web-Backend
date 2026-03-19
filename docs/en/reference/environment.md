# Environment Variables Quick Reference

> **Navigation**: [Docs Home](../README.md) > [Reference](README.md) > Environment

Quick-reference table for all environment variables, grouped by category. For detailed descriptions, validation rules, and examples, see the [Configuration Reference](configuration.md).

---

## Server

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `BACKEND_BASE_URL` | ✅ | — | `https://api.example.com` |
| `FRONTEND_ORIGIN` | ✅ | — | `https://example.com` |
| `BIND_ADDRESS` | — | `0.0.0.0:3000` | `127.0.0.1:8080` |
| `RUST_LOG` | — | `info` | `vrc_backend=debug` |

## Database

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `DATABASE_URL` | ✅ | — | `postgres://vrc:pass@localhost:5432/vrc_db` |
| `DATABASE_MAX_CONNECTIONS` | — | `10` | `20` |

## Discord

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `DISCORD_CLIENT_ID` | ✅ | — | `123456789012345678` |
| `DISCORD_CLIENT_SECRET` | ✅ | — | `abc123...` |
| `DISCORD_GUILD_ID` | ✅ | — | `987654321098765432` |
| `DISCORD_WEBHOOK_URL` | — | — | `https://discord.com/api/webhooks/...` |

## Security

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `SESSION_SECRET` | ✅ | — | (≥ 32 chars, CSPRNG) |
| `SYSTEM_API_TOKEN` | ✅ | — | (≥ 64 chars, CSPRNG) |
| `COOKIE_SECURE` | — | `false` | `true` (production) |
| `TRUST_X_FORWARDED_FOR` | — | `false` | `true` (behind proxy) |

## Session & Background Tasks

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `SESSION_MAX_AGE_SECS` | — | `604800` | `1209600` (14 days) |
| `SESSION_CLEANUP_INTERVAL_SECS` | — | `3600` | `1800` (30 min) |
| `EVENT_ARCHIVAL_INTERVAL_SECS` | — | `86400` | `43200` (12 hours) |

## Optional Features

| Variable | Required | Default | Example |
|----------|:--------:|---------|---------|
| `SUPER_ADMIN_DISCORD_ID` | — | — | `111222333444555666` |

---

## Generating Secrets

```bash
# Session secret (minimum 32 characters)
openssl rand -hex 32

# System API token (minimum 64 characters)
openssl rand -hex 64
```

---

## Related Documents

- [Configuration Reference](configuration.md) — Full details, validation rules, and production checklist
- [Quickstart](../getting-started/quickstart.md) — Initial environment setup
