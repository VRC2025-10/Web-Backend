# Installation

> **Navigation**: [Docs Home](../README.md) > [Getting Started](README.md) > Installation

This guide covers everything you need to install and configure the VRC Backend.

---

## Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| Rust | 1.85+ (Edition 2024) | Build and run the backend |
| PostgreSQL | 16+ | Primary database |
| Docker & Docker Compose | Latest | Containerized development and deployment |
| Discord Application | — | OAuth2 authentication |

---

## Method 1: Docker Development (Recommended)

Use Docker Compose to run PostgreSQL while running the Rust application on the host. This is the fastest way to get started.

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/Web-Backend.git
cd Web-Backend
```

### 2. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` and fill in the required variables:

```dotenv
# Database (matches docker-compose.yml defaults)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/vrc

# Discord OAuth2 (see "Discord Application Setup" below)
DISCORD_CLIENT_ID=your_discord_client_id
DISCORD_CLIENT_SECRET=your_discord_client_secret
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback
DISCORD_GUILD_ID=your_discord_guild_id

# Secrets
SESSION_SECRET=<at least 32 characters>
SYSTEM_API_TOKEN=<at least 64 characters>

# Frontend
FRONTEND_ORIGIN=http://localhost:5173
```

> **Tip**: Generate secrets with:
> ```bash
> openssl rand -hex 32   # SESSION_SECRET
> openssl rand -hex 64   # SYSTEM_API_TOKEN
> ```

### 3. Start PostgreSQL

```bash
docker compose up -d
```

### 4. Build and Run

```bash
cargo run
```

Database migrations run automatically on startup.

---

## Method 2: Manual Setup

Install and configure everything without Docker.

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

Verify:

```bash
rustc --version   # Should be 1.85+
cargo --version
```

### 2. Install PostgreSQL

**Ubuntu/Debian:**

```bash
sudo apt update
sudo apt install postgresql-16 postgresql-client-16
sudo systemctl start postgresql
```

**macOS (Homebrew):**

```bash
brew install postgresql@16
brew services start postgresql@16
```

### 3. Create Database

```bash
sudo -u postgres psql -c "CREATE DATABASE vrc;"
sudo -u postgres psql -c "CREATE USER vrc_user WITH PASSWORD 'your_password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE vrc TO vrc_user;"
```

### 4. Configure Environment

```bash
cp .env.example .env
```

Update `DATABASE_URL` to match your local PostgreSQL setup:

```dotenv
DATABASE_URL=postgres://vrc_user:your_password@localhost:5432/vrc
```

Fill in all other required variables as shown in Method 1.

### 5. Build and Run

```bash
cargo run
```

Migrations are applied automatically on startup — no manual migration step is needed.

---

## Method 3: Full Docker Production

Deploy the complete stack (application + PostgreSQL + Caddy reverse proxy) using the production Docker Compose configuration.

```bash
# Configure environment
cp .env.example .env
# Edit .env with production values (see below)

# Start the full stack
docker compose -f docker-compose.prod.yml up -d
```

### Production Environment Considerations

| Variable | Recommended Value |
|----------|-------------------|
| `COOKIE_SECURE` | `true` |
| `TRUST_X_FORWARDED_FOR` | `true` |
| `BIND_ADDRESS` | `0.0.0.0:3000` (default) |
| `DATABASE_MAX_CONNECTIONS` | Tune based on load (default: `10`) |
| `RUST_LOG` | `info` or `warn` |

The `docker-compose.prod.yml` stack includes:

- **app** — The VRC Backend Rust application
- **postgres** — PostgreSQL 16 database
- **caddy** — Caddy reverse proxy with automatic HTTPS

---

## Discord Application Setup

The VRC Backend uses Discord OAuth2 for authentication. You must create a Discord Application to obtain the required credentials.

### 1. Create a Discord Application

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **New Application** and give it a name
3. Note the **Application ID** — this is your `DISCORD_CLIENT_ID`

### 2. Configure OAuth2

1. Navigate to **OAuth2** in the left sidebar
2. Under **Redirects**, add your redirect URI:
   - Development: `http://localhost:3000/api/v1/auth/callback`
   - Production: `https://your-domain.com/api/v1/auth/callback`
3. Copy the **Client Secret** — this is your `DISCORD_CLIENT_SECRET`

### 3. Get Your Guild ID

1. Open Discord and enable **Developer Mode** (User Settings → Advanced → Developer Mode)
2. Right-click your server name and select **Copy Server ID**
3. Set this as `DISCORD_GUILD_ID`

### 4. Update Environment

```dotenv
DISCORD_CLIENT_ID=123456789012345678
DISCORD_CLIENT_SECRET=your_client_secret_here
DISCORD_REDIRECT_URI=http://localhost:3000/api/v1/auth/callback
DISCORD_GUILD_ID=987654321098765432
```

---

## Optional Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDRESS` | `0.0.0.0:3000` | Server listen address |
| `DATABASE_MAX_CONNECTIONS` | `10` | Max PostgreSQL connection pool size |
| `SESSION_MAX_AGE_SECS` | `604800` (7 days) | Session lifetime |
| `SESSION_CLEANUP_INTERVAL_SECS` | `3600` (1 hour) | Expired session cleanup interval |
| `EVENT_ARCHIVAL_INTERVAL_SECS` | `86400` (1 day) | Event archival task interval |
| `SUPER_ADMIN_DISCORD_ID` | — | Discord ID for the super admin user |
| `DISCORD_WEBHOOK_URL` | — | Discord webhook for notifications |
| `COOKIE_SECURE` | `false` | Set `true` in production (HTTPS) |
| `TRUST_X_FORWARDED_FOR` | `false` | Set `true` behind a reverse proxy |
| `RUST_LOG` | — | Log level filter (e.g., `info`, `debug`) |

---

## Verifying the Installation

After starting the server, verify it is running:

```bash
curl http://localhost:3000/health
```

A successful response confirms the server is up and connected to the database.

---

## Makefile Targets

The project includes a Makefile with common development tasks:

```bash
make setup    # Initial project setup
make run      # Run the server
make build    # Build release binary
make test     # Run tests
make lint     # Run clippy lints
make fmt      # Format code
make check    # Run all checks
make db-up    # Start PostgreSQL via Docker Compose
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Database connection refused | Ensure PostgreSQL is running: `docker compose up -d` or check your local PostgreSQL service |
| Migration errors | Migrations run on startup. Check `DATABASE_URL` is correct and the database exists |
| Discord OAuth2 errors | Verify `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, and `DISCORD_REDIRECT_URI` match your Discord Application settings |
| `SESSION_SECRET too short` | Must be at least 32 characters |
| `SYSTEM_API_TOKEN too short` | Must be at least 64 characters |
| Port already in use | Change `BIND_ADDRESS` or stop the conflicting process |

---

## Related Documents

- [Quickstart](quickstart.md)
- [Examples](examples.md)
- [Docker Architecture](../../../specs/07-infrastructure/docker-architecture.md)
- [Database](../../../specs/04-database/README.md)
- [Security — Authentication Design](../../../specs/06-security/authentication-design.md)
