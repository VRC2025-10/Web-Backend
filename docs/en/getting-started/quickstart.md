# Quickstart

> **Navigation**: [Docs Home](../README.md) > [Getting Started](README.md) > Quickstart

Get the VRC Backend running in 5 minutes.

---

## 1. Clone the Repository

```bash
git clone https://github.com/your-org/Web-Backend.git
cd Web-Backend
```

## 2. Copy the Environment File

```bash
cp .env.example .env
```

## 3. Set Discord OAuth2 Credentials

Open `.env` and fill in the Discord-related variables:

```dotenv
# Your Discord Application's ID (from Developer Portal → General Information)
DISCORD_CLIENT_ID=123456789012345678

# Your Discord Application's secret (from Developer Portal → OAuth2)
DISCORD_CLIENT_SECRET=your_client_secret_here

# Must match a redirect URI registered in your Discord Application
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback

# The Discord server ID whose members can authenticate
DISCORD_GUILD_ID=987654321098765432

# URL of your frontend application (for CORS)
FRONTEND_ORIGIN=http://localhost:5173
```

> Don't have a Discord Application yet? See the [Installation Guide — Discord Application Setup](installation.md#discord-application-setup).

## 4. Generate Secrets

```bash
# Session secret (minimum 32 characters)
echo "SESSION_SECRET=$(openssl rand -hex 32)" >> .env

# System API token (minimum 64 characters)
echo "SYSTEM_API_TOKEN=$(openssl rand -hex 64)" >> .env
```

## 5. Start PostgreSQL

```bash
docker compose up -d
```

This starts a PostgreSQL 16 container configured to work with the default `DATABASE_URL` in `.env.example`.

## 6. Run the Server

```bash
cargo run
```

The first build will take a few minutes. On startup, the server will:

1. Connect to PostgreSQL
2. Run any pending database migrations automatically
3. Start background tasks (session cleanup, event archival)
4. Begin listening on `http://localhost:3000`

## 7. Verify It Works

```bash
curl http://localhost:3000/health
```

You should receive a successful response confirming the server is running and connected to the database.

## 8. Try the Public API

```bash
curl http://localhost:3000/api/v1/public/members
```

This returns a paginated list of members. The public API requires no authentication.

## 9. What's Next?

| Next Step | Link |
|-----------|------|
| Explore more API examples | [Examples](examples.md) |
| Review full installation options | [Installation](installation.md) |
| Read the API documentation | [API Docs](../../api/README.md) |
| Understand the architecture | [Architecture](../../../specs/02-architecture/README.md) |
| Run the test suite | `make test` |
| Check code quality | `make lint && make fmt` |

---

## Quick Reference

| Action | Command |
|--------|---------|
| Start database | `docker compose up -d` |
| Stop database | `docker compose down` |
| Run server | `cargo run` |
| Run tests | `make test` |
| Build release | `make build` |
| Check health | `curl http://localhost:3000/health` |

---

## Related Documents

- [Installation](installation.md) — Full installation guide with all methods and troubleshooting
- [Examples](examples.md) — Comprehensive API usage examples
- [API Documentation](../../api/README.md)
