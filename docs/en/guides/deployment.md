# Deployment Guide

> **Audience**: Operators
>
> **Navigation**: [Docs Home](../README.md) > [Guides](README.md) > Deployment

## Overview

The VRC Web-Backend runs as a Docker Compose stack on a single Proxmox VM. The production stack consists of three services: the Rust backend, PostgreSQL, and Caddy (reverse proxy with automatic TLS).

## Deployment Topology

```mermaid
graph TD
    subgraph "Internet"
        Users["Users<br/>(HTTPS)"]
        GAS["Google Apps Script<br/>(HTTPS)"]
        Bot["Discord Bot<br/>(HTTPS)"]
    end

    subgraph "Proxmox VM"
        subgraph "Docker Compose Stack"
            Caddy["Caddy 2<br/>:443 (HTTPS)<br/>:80 (redirect)<br/>Automatic TLS + HTTP/3"]
            App["VRC Backend<br/>:3000 (HTTP)<br/>Rust/Axum"]
            DB[("PostgreSQL 16<br/>:5432<br/>Persistent Volume")]
        end
    end

    subgraph "External"
        Discord["Discord API"]
    end

    Users -->|"HTTPS :443"| Caddy
    GAS -->|"HTTPS :443"| Caddy
    Bot -->|"HTTPS :443"| Caddy

    Caddy -->|"HTTP :3000"| App
    App -->|"TCP :5432"| DB
    App -->|"HTTPS"| Discord

    style Caddy fill:#1F88E5,color:white
    style App fill:#4CAF50,color:white
    style DB fill:#FF9800,color:white
```

## Prerequisites

- Proxmox VM (or any Linux server) with:
  - Docker Engine ≥ 24.0
  - Docker Compose ≥ 2.20
  - 2+ GB RAM
  - 20+ GB disk
- Domain name pointing to the server's IP address
- Discord application configured (see [Configuration Guide](configuration.md))

## Production Stack (docker-compose.prod.yml)

The production Docker Compose file defines three services:

```yaml
services:
  app:
    image: vrc-backend:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://vrc_user:${DB_PASSWORD}@postgres:5432/vrc_prod
      - SESSION_SECRET_FILE=/run/secrets/session_secret
      - SYSTEM_API_TOKEN_FILE=/run/secrets/system_api_token
      - DISCORD_CLIENT_ID=${DISCORD_CLIENT_ID}
      - DISCORD_CLIENT_SECRET=${DISCORD_CLIENT_SECRET}
      - DISCORD_GUILD_ID=${DISCORD_GUILD_ID}
      - DISCORD_REDIRECT_URI=${DISCORD_REDIRECT_URI}
      - FRONTEND_ORIGIN=${FRONTEND_ORIGIN}
      - COOKIE_SECURE=true
      - TRUST_X_FORWARDED_FOR=true
      - RUST_LOG=info
    secrets:
      - session_secret
      - system_api_token
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped
    networks:
      - backend

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=vrc_user
      - POSTGRES_DB=vrc_prod
      - POSTGRES_PASSWORD_FILE=/run/secrets/db_password
    secrets:
      - db_password
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U vrc_user -d vrc_prod"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped
    networks:
      - backend

  caddy:
    image: caddy:2-alpine
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"  # HTTP/3
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - app
    restart: unless-stopped
    networks:
      - backend

volumes:
  pgdata:
  caddy_data:
  caddy_config:

networks:
  backend:

secrets:
  db_password:
    file: ./secrets/db_password.txt
  session_secret:
    file: ./secrets/session_secret.txt
  system_api_token:
    file: ./secrets/system_api_token.txt
```

## Caddy Configuration

The Caddyfile configures automatic TLS, HTTP/3, and reverse proxying:

```caddyfile
api.your-domain.com {
    reverse_proxy app:3000

    # Enable HTTP/3
    servers {
        protocol {
            experimental_http3
        }
    }

    # Security headers (supplementary to backend headers)
    header {
        -Server
    }

    # Access logging
    log {
        output file /var/log/caddy/access.log
    }
}
```

Caddy automatically:
- Obtains and renews TLS certificates via Let's Encrypt
- Redirects HTTP to HTTPS
- Enables HTTP/3 (QUIC)
- Handles TLS termination

## Secrets Management

Create the secrets directory and generate secrets:

```bash
mkdir -p secrets

# Database password
openssl rand -base64 32 > secrets/db_password.txt

# Session secret (minimum 64 hex characters)
openssl rand -hex 64 > secrets/session_secret.txt

# System API token (minimum 64 hex characters)
openssl rand -hex 64 > secrets/system_api_token.txt

# Restrict permissions
chmod 600 secrets/*.txt
```

> **Important**: The `secrets/` directory should NOT be committed to version control. Add it to `.gitignore`.

## Deployment Procedure

### Initial Deployment

```bash
# 1. Clone the repository
git clone <repo-url> /opt/vrc-backend
cd /opt/vrc-backend

# 2. Create secrets
mkdir -p secrets
openssl rand -base64 32 > secrets/db_password.txt
openssl rand -hex 64 > secrets/session_secret.txt
openssl rand -hex 64 > secrets/system_api_token.txt
chmod 600 secrets/*.txt

# 3. Create .env file with Discord credentials
cat > .env << 'EOF'
DISCORD_CLIENT_ID=your_client_id
DISCORD_CLIENT_SECRET=your_client_secret
DISCORD_GUILD_ID=your_guild_id
DISCORD_REDIRECT_URI=https://api.your-domain.com/api/v1/auth/callback
FRONTEND_ORIGIN=https://your-domain.com
EOF

# 4. Build and start
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d

# 5. Run database migrations
docker compose -f docker-compose.prod.yml exec app ./vrc-backend migrate

# 6. Verify health
curl -s https://api.your-domain.com/api/v1/public/health | jq .
```

### Updating (Subsequent Deployments)

```bash
cd /opt/vrc-backend

# 1. Pull latest code
git pull origin main

# 2. Rebuild and restart
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d

# 3. Run any new migrations
docker compose -f docker-compose.prod.yml exec app ./vrc-backend migrate

# 4. Verify health
curl -s https://api.your-domain.com/api/v1/public/health | jq .
```

## Rollback Procedure

If a deployment causes issues:

```bash
# 1. Check logs for errors
docker compose -f docker-compose.prod.yml logs app --tail=100

# 2. Roll back to previous version
git checkout <previous-tag-or-commit>
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d

# 3. If database migration needs rollback (manual)
# Note: Always test migrations in a staging environment first
docker compose -f docker-compose.prod.yml exec postgres \
  psql -U vrc_user -d vrc_prod -f /path/to/rollback.sql

# 4. Verify health
curl -s https://api.your-domain.com/api/v1/public/health | jq .
```

## Health Check Verification

The backend exposes a health check endpoint:

```bash
# Basic health check
curl -s https://api.your-domain.com/api/v1/public/health

# Expected response
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

## Database Backups

### Automated Backup Script

```bash
#!/bin/bash
# backup.sh — Run daily via cron
BACKUP_DIR="/opt/backups/vrc"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

mkdir -p "$BACKUP_DIR"

# Dump the database
docker compose -f /opt/vrc-backend/docker-compose.prod.yml exec -T postgres \
  pg_dump -U vrc_user -d vrc_prod --format=custom \
  > "$BACKUP_DIR/vrc_prod_${TIMESTAMP}.dump"

# Remove old backups
find "$BACKUP_DIR" -name "*.dump" -mtime +${RETENTION_DAYS} -delete

echo "Backup completed: vrc_prod_${TIMESTAMP}.dump"
```

### Cron Schedule

```bash
# Daily backup at 3:00 AM
0 3 * * * /opt/scripts/backup.sh >> /var/log/vrc-backup.log 2>&1
```

### Restore from Backup

```bash
# Stop the application
docker compose -f docker-compose.prod.yml stop app

# Restore
docker compose -f docker-compose.prod.yml exec -T postgres \
  pg_restore -U vrc_user -d vrc_prod --clean --if-exists \
  < /opt/backups/vrc/vrc_prod_20250301_030000.dump

# Restart the application
docker compose -f docker-compose.prod.yml start app
```

## Monitoring

### Log Monitoring

```bash
# Follow application logs
docker compose -f docker-compose.prod.yml logs -f app

# Follow all service logs
docker compose -f docker-compose.prod.yml logs -f

# Check for errors in the last hour
docker compose -f docker-compose.prod.yml logs --since 1h app | grep -i error
```

### Resource Monitoring

```bash
# Check container resource usage
docker stats --no-stream

# Check disk usage for volumes
docker system df -v
```

### Prometheus Metrics Endpoint

If configured, the backend exposes a metrics endpoint for Prometheus scraping:

```
GET /metrics
```

Metrics include:
- `http_requests_total` — total HTTP requests by method, path, and status
- `http_request_duration_seconds` — request latency histogram
- `db_pool_connections` — database connection pool status
- `rate_limit_rejections_total` — rate limit rejections by tier

### Prometheus Scrape Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'vrc-backend'
    scrape_interval: 15s
    static_configs:
      - targets: ['app:3000']
    metrics_path: /metrics
```

## Docker Image Build

The Dockerfile uses multi-stage builds with `cargo-chef` for efficient caching:

```mermaid
graph TD
    Stage1["Stage 1: Chef<br/>Install cargo-chef"]
    Stage2["Stage 2: Planner<br/>cargo chef prepare<br/>(dependency plan)"]
    Stage3["Stage 3: Builder<br/>cargo chef cook (deps)<br/>cargo build --release"]
    Stage4["Stage 4: Runtime<br/>Minimal Debian image<br/>Copy binary only"]

    Stage1 --> Stage2
    Stage2 --> Stage3
    Stage3 --> Stage4

    Stage4 -->|"~50MB image"| Final["Production Image"]
```

This produces a minimal production image containing only the compiled binary and required runtime libraries.

## Related Documents

- [Configuration Guide](configuration.md) — environment variables and secrets
- [Security Guide](security.md) — security hardening
- [Troubleshooting](troubleshooting.md) — deployment issues
- [Architecture Overview](../architecture/README.md) — system structure
