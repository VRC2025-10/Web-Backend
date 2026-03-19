# Troubleshooting Guide

> **Audience**: All
>
> **Navigation**: [Docs Home](../README.md) > [Guides](README.md) > Troubleshooting

## Overview

This guide covers common issues encountered when developing, building, and running the VRC Web-Backend, along with their symptoms, causes, and solutions.

---

## Database Issues

### Database Connection Refused

| | Detail |
|---|---|
| **Symptom** | Application fails to start with: `error connecting to database: Connection refused (os error 111)` |
| **Cause** | PostgreSQL is not running or is not accepting connections on the expected host/port |
| **Solution** | |

```bash
# Check if PostgreSQL is running
docker compose ps postgres

# If not running, start it
docker compose up -d postgres

# Verify it's accepting connections
docker compose exec postgres pg_isready -U postgres

# Check the DATABASE_URL in .env
grep DATABASE_URL .env
# Should be: postgres://user:pass@localhost:5432/dbname (development)
# Or:        postgres://user:pass@postgres:5432/dbname (Docker)
```

If using Docker Compose, ensure the application is on the same Docker network as PostgreSQL.

---

### Migration Failed

| | Detail |
|---|---|
| **Symptom** | `sqlx migrate run` fails with `error: connection refused` or `database "vrc_dev" does not exist` |
| **Cause** | Database is unreachable, or the target database hasn't been created |
| **Solution** | |

```bash
# Verify PostgreSQL is running
docker compose up -d postgres

# Create the database if it doesn't exist
docker compose exec postgres createdb -U postgres vrc_dev

# Verify DATABASE_URL points to the correct database
echo $DATABASE_URL

# Run migrations
cargo sqlx migrate run
```

---

## Authentication Issues

### CSRF Validation Failed

| | Detail |
|---|---|
| **Symptom** | `403 Forbidden` with error code `ERR-CSRF-001` on POST/PUT/DELETE requests |
| **Cause** | The `Origin` header in the request doesn't match `FRONTEND_ORIGIN` in the backend configuration |
| **Solution** | |

```bash
# Check what FRONTEND_ORIGIN is set to
grep FRONTEND_ORIGIN .env

# It must match exactly what the browser sends as the Origin header
# Common mistakes:
#   - Trailing slash: https://example.com/  (wrong)
#   - Wrong port:     http://localhost:3000  (wrong, should be frontend port)
#   - Wrong protocol: http:// vs https://
```

Ensure `FRONTEND_ORIGIN` matches your frontend's URL exactly:
- Development: `http://localhost:5173` (or your frontend dev server port)
- Production: `https://your-domain.com` (no trailing slash)

---

### Login Failed — Not in Guild

| | Detail |
|---|---|
| **Symptom** | Login redirects back to the frontend with an error, or returns `403 Forbidden` with `ERR-AUTH-003` |
| **Cause** | The Discord user is not a member of the configured guild (`DISCORD_GUILD_ID`) |
| **Solution** | |

1. Verify the user has joined the Discord server
2. Verify `DISCORD_GUILD_ID` is correct:
   ```bash
   grep DISCORD_GUILD_ID .env
   ```
3. Enable Discord Developer Mode (User Settings → Advanced → Developer Mode)
4. Right-click the server name → Copy Server ID
5. Compare with the configured `DISCORD_GUILD_ID`

---

### Login Failed — Account Suspended

| | Detail |
|---|---|
| **Symptom** | Login redirects with an error or returns `403 Forbidden` with `ERR-AUTH-004` |
| **Cause** | The user's account has been suspended by an admin |
| **Solution** | |

An admin must unsuspend the user's account. Check the user's status:

```bash
# Query the database (development)
docker compose exec postgres psql -U postgres -d vrc_dev \
  -c "SELECT id, discord_id, display_name, role, is_suspended FROM users WHERE discord_id = '<discord_id>';"
```

---

### Session Invalid

| | Detail |
|---|---|
| **Symptom** | `401 Unauthorized` on authenticated endpoints that previously worked |
| **Cause** | Session has expired, been manually revoked, or database sessions were cleared |
| **Solution** | |

1. Log out and log in again to create a new session
2. Check session expiry configuration:
   ```bash
   grep SESSION_MAX_AGE_HOURS .env
   # Default is 168 hours (7 days)
   ```
3. Verify the sessions table has entries:
   ```bash
   docker compose exec postgres psql -U postgres -d vrc_dev \
     -c "SELECT COUNT(*) FROM sessions WHERE expires_at > NOW();"
   ```

---

## System API Issues

### System Token Invalid

| | Detail |
|---|---|
| **Symptom** | `401 Unauthorized` with error code `ERR-SYNC-001` on system API endpoints |
| **Cause** | The `Authorization: Bearer <token>` header doesn't match `SYSTEM_API_TOKEN` |
| **Solution** | |

```bash
# Verify the token matches (compare lengths at minimum)
grep SYSTEM_API_TOKEN .env | wc -c

# Test with curl
curl -v -H "Authorization: Bearer $(grep SYSTEM_API_TOKEN .env | cut -d= -f2)" \
  http://localhost:3000/api/v1/system/events
```

Common mistakes:
- Extra whitespace or newline in the token
- Using the wrong environment's token
- Token shorter than 64 characters

---

### Rate Limit Exceeded

| | Detail |
|---|---|
| **Symptom** | `429 Too Many Requests` response |
| **Cause** | Client has exceeded the rate limit for the endpoint tier |
| **Solution** | |

Wait for the rate limit window to reset. Rate limits per tier:

| Tier | Limit | Reset Window |
|------|-------|-------------|
| Public | 60 requests | Per minute |
| Internal | 120 requests | Per minute |
| System | 30 requests | Per minute |
| Auth | 10 requests | Per minute |

For system API integrations, implement exponential backoff:

```python
import time

def call_api_with_retry(url, headers, max_retries=3):
    for attempt in range(max_retries):
        response = requests.post(url, headers=headers)
        if response.status_code != 429:
            return response
        wait = 2 ** attempt  # 1s, 2s, 4s
        time.sleep(wait)
    return response
```

---

## Build Issues

### SQLx Build Error

| | Detail |
|---|---|
| **Symptom** | `error: error communicating with database` during `cargo build` |
| **Cause** | SQLx compile-time queries need either a running PostgreSQL database or offline metadata |
| **Solution** | |

**Option A: Start the database**
```bash
docker compose up -d postgres
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc_dev
cargo build
```

**Option B: Use offline mode**
```bash
# If .sqlx/ directory exists in the repo, build with offline mode
export SQLX_OFFLINE=true
cargo build
```

**Option C: Regenerate offline metadata**
```bash
# Requires a running database
docker compose up -d postgres
cargo sqlx prepare
# Commit the updated .sqlx/ directory
git add .sqlx/
git commit -m "Update sqlx offline metadata"
```

---

### Clippy Warnings

| | Detail |
|---|---|
| **Symptom** | `cargo clippy` reports warnings or errors |
| **Cause** | Code style issues or potential bugs detected by Clippy |
| **Solution** | |

```bash
# Auto-fix what Clippy can fix
cargo clippy --fix --allow-dirty

# Manual fixes required for remaining warnings
cargo clippy -- -W clippy::all

# For CI (treat warnings as errors)
cargo clippy -- -D warnings
```

---

## Performance Issues

### Slow Response Times

| | Detail |
|---|---|
| **Symptom** | API responses take longer than expected (>100ms for simple queries) |
| **Cause** | Verbose logging, unoptimized queries, or connection pool exhaustion |
| **Solution** | |

1. **Check log level**:
   ```bash
   grep RUST_LOG .env
   # Use 'info' or 'warn' in production, NOT 'debug' or 'trace'
   ```

2. **Check database connection pool**:
   ```bash
   # Check active connections
   docker compose exec postgres psql -U postgres -d vrc_dev \
     -c "SELECT count(*) FROM pg_stat_activity WHERE datname = 'vrc_dev';"
   ```

3. **Check for slow queries**:
   ```bash
   docker compose exec postgres psql -U postgres -d vrc_dev \
     -c "SELECT query, mean_exec_time, calls FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 10;"
   ```

4. **Check container resources**:
   ```bash
   docker stats --no-stream
   ```

---

### High Memory Usage

| | Detail |
|---|---|
| **Symptom** | Application container using more memory than expected |
| **Cause** | Debug build (not release), memory leak, or large response caching |
| **Solution** | |

1. Ensure the production build uses `--release`:
   ```bash
   # In Dockerfile, should be:
   cargo build --release
   ```

2. Check container memory:
   ```bash
   docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}"
   ```

3. Restart the application if memory usage is abnormally high:
   ```bash
   docker compose -f docker-compose.prod.yml restart app
   ```

---

## Quick Reference

| Error Code | Meaning | First Action |
|------------|---------|-------------|
| `ERR-AUTH-001` | Invalid or missing authentication | Log in again |
| `ERR-AUTH-002` | Session expired | Log in again |
| `ERR-AUTH-003` | Not in Discord guild | Join the Discord server |
| `ERR-AUTH-004` | Account suspended | Contact an admin |
| `ERR-CSRF-001` | CSRF validation failed | Check `FRONTEND_ORIGIN` |
| `ERR-SYNC-001` | Invalid system token | Check `SYSTEM_API_TOKEN` |
| `ERR-SYNC-002` | Invalid request body | Check request payload format |
| `ERR-VAL-001` | Input validation failed | Check field constraints |
| `ERR-USR-001` | User not found | Verify user ID |
| `ERR-USR-002` | User suspended | Contact an admin |
| `ERR-EVT-001` | Event not found | Verify event ID |
| `ERR-SYS-001` | Internal server error | Check application logs |

## Related Documents

- [Configuration Guide](configuration.md) — environment variable reference
- [Deployment Guide](deployment.md) — deployment procedures
- [Security Guide](security.md) — security-related issues
- [Error Reference](../reference/errors.md) — complete error code listing
