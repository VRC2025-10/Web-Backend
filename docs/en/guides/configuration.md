# Configuration Guide

> **Audience**: Users, Operators
>
> **Navigation**: [Docs Home](../README.md) > [Guides](README.md) > Configuration

## Overview

The VRC Web-Backend is configured through environment variables, typically provided via a `.env` file in the project root. Configuration is validated at startup — the application will refuse to start if required variables are missing or invalid.

## How Configuration Works

```mermaid
graph TD
    ENV[".env file"]
    SYS["System environment variables"]
    APP["Application startup"]
    VAL["Validation"]

    ENV -->|"loaded by dotenvy"| APP
    SYS -->|"override .env"| APP
    APP --> VAL

    VAL -->|"All valid"| RUN["Application runs ✅"]
    VAL -->|"Missing/invalid"| FAIL["Startup fails ❌<br/>with specific error"]
```

1. The `.env` file is loaded by the `dotenvy` crate at startup
2. System environment variables override `.env` file values
3. All variables are validated (presence, format, minimum lengths)
4. If any required variable is missing or invalid, the application exits with a descriptive error

## Required Variables

| Variable | Description | Example | Constraints |
|----------|-------------|---------|-------------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@localhost:5432/vrc` | Valid PostgreSQL URL |
| `SESSION_SECRET` | HMAC key for session signing | (64+ random hex chars) | Minimum 64 characters |
| `DISCORD_CLIENT_ID` | Discord OAuth2 application ID | `1234567890` | Non-empty |
| `DISCORD_CLIENT_SECRET` | Discord OAuth2 application secret | `AbCdEf...` | Non-empty |
| `DISCORD_GUILD_ID` | Target Discord server ID | `9876543210` | Non-empty |
| `DISCORD_REDIRECT_URI` | OAuth2 callback URL | `https://api.example.com/api/v1/auth/callback` | Valid URL |
| `FRONTEND_ORIGIN` | Frontend URL for CORS/CSRF | `https://example.com` | Valid URL, no trailing slash |
| `SYSTEM_API_TOKEN` | Token for system API authentication | (64+ random hex chars) | Minimum 64 characters |

## Optional Variables

| Variable | Description | Default | Values |
|----------|-------------|---------|--------|
| `RUST_LOG` | Log level filter | `info` | `trace`, `debug`, `info`, `warn`, `error` |
| `HOST` | Bind address | `0.0.0.0` | Valid IP address |
| `PORT` | Bind port | `3000` | 1-65535 |
| `COOKIE_SECURE` | Require HTTPS for cookies | `true` | `true`, `false` |
| `TRUST_X_FORWARDED_FOR` | Trust proxy headers for client IP | `false` | `true`, `false` |
| `SESSION_MAX_AGE_HOURS` | Session lifetime | `168` (7 days) | Positive integer |

## Secret Generation

Generate cryptographically secure secrets with `openssl`:

```bash
# Generate SESSION_SECRET (64+ hex characters)
openssl rand -hex 64

# Generate SYSTEM_API_TOKEN (64+ hex characters)
openssl rand -hex 64

# Generate a database password
openssl rand -base64 32
```

> **Warning**: Never reuse secrets across environments (development, staging, production). Generate fresh secrets for each environment.

## Discord Application Setup

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application (or select an existing one)
3. Navigate to **OAuth2** settings
4. Add the redirect URI: `https://your-domain.com/api/v1/auth/callback`
5. Copy the **Client ID** and **Client Secret**
6. Navigate to **Bot** settings if needed for the guild ID
7. Note the **Guild ID** of your target Discord server (right-click server name → Copy Server ID with Developer Mode enabled)

Required OAuth2 scopes: `identify`, `guilds`

## Database Connection URL Format

```
postgres://USERNAME:PASSWORD@HOST:PORT/DATABASE_NAME
```

| Component | Development | Production |
|-----------|-------------|------------|
| USERNAME | `postgres` | Custom user |
| PASSWORD | `postgres` | From secrets file |
| HOST | `localhost` | `postgres` (Docker service name) |
| PORT | `5432` | `5432` |
| DATABASE_NAME | `vrc_dev` | `vrc_prod` |

**Development example:**
```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc_dev
```

**Production example (Docker Compose):**
```
DATABASE_URL=postgres://vrc_user:${DB_PASSWORD}@postgres:5432/vrc_prod
```

## Example .env File

```bash
# === Required ===
DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc_dev
SESSION_SECRET=<output of: openssl rand -hex 64>
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_GUILD_ID=your_guild_id
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback
FRONTEND_ORIGIN=http://localhost:5173
SYSTEM_API_TOKEN=<output of: openssl rand -hex 64>

# === Optional ===
RUST_LOG=debug
HOST=0.0.0.0
PORT=3000
COOKIE_SECURE=false
TRUST_X_FORWARDED_FOR=false
```

## Production Checklist

Before deploying to production, verify these settings:

| Setting | Requirement | Why |
|---------|-------------|-----|
| `COOKIE_SECURE` | `true` | Cookies only sent over HTTPS |
| `TRUST_X_FORWARDED_FOR` | `true` | Backend is behind Caddy reverse proxy |
| `FRONTEND_ORIGIN` | Actual production URL | CORS and CSRF validation use this |
| `DISCORD_REDIRECT_URI` | Production callback URL | Must match Discord Developer Portal |
| `RUST_LOG` | `info` or `warn` | Avoid verbose logging in production |
| `SESSION_SECRET` | Fresh 64+ char secret | Unique to production environment |
| `SYSTEM_API_TOKEN` | Fresh 64+ char token | Unique to production environment |
| Database password | Strong, unique password | Not the development default |

## Configuration Validation on Startup

The application validates all configuration at startup:

```
[INFO] Loading configuration...
[INFO] DATABASE_URL: ✓ (postgres://***@localhost:5432/vrc_dev)
[INFO] SESSION_SECRET: ✓ (128 characters)
[INFO] DISCORD_CLIENT_ID: ✓
[INFO] DISCORD_CLIENT_SECRET: ✓
[INFO] FRONTEND_ORIGIN: ✓ (http://localhost:5173)
[INFO] SYSTEM_API_TOKEN: ✓ (128 characters)
[INFO] Configuration valid. Starting server on 0.0.0.0:3000
```

If a variable is missing:

```
[ERROR] Configuration error: SESSION_SECRET is required but not set
[ERROR] Hint: Generate with: openssl rand -hex 64
```

If a variable is invalid:

```
[ERROR] Configuration error: SESSION_SECRET must be at least 64 characters (got 32)
```

## Related Documents

- [Deployment Guide](deployment.md) — production deployment with Docker
- [Security Guide](security.md) — security implications of configuration
- [Environment Variables Reference](../reference/environment.md) — complete variable listing
- [Troubleshooting](troubleshooting.md) — configuration-related issues
