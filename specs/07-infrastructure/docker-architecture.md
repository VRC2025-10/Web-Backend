# Docker Architecture

## Dockerfile (Multi-Stage Build)

```dockerfile
# ============================================================
# Stage 1: Build (chef + compile)
# ============================================================
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies only (cached unless Cargo.lock changes)
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin vrc-backend

# ============================================================
# Stage 2: Runtime (minimal)
# ============================================================
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 app && \
    useradd --uid 1000 --gid app --no-create-home app

COPY --from=builder /app/target/release/vrc-backend /usr/local/bin/vrc-backend
COPY --from=builder /app/migrations /app/migrations

USER app
WORKDIR /app
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/vrc-backend", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/vrc-backend"]
```

### Build Optimization Notes

- **cargo-chef**: Caches dependency compilation. If only application code changes (not `Cargo.lock`), dependencies are reused from cache. Reduces rebuild from ~8 min to ~30s.
- **SQLX_OFFLINE=true**: Uses `sqlx-data.json` (pre-generated offline query metadata) instead of requiring a live database at compile time. This file is committed to version control.
- **debian:bookworm-slim** (not scratch): We need `ca-certificates` for TLS connections to Discord API and `libc` for jemalloc. Scratch would require static linking.
- **Non-root user**: Application runs as `app:app` (UID/GID 1000).

### Image Size Estimate

| Layer | Size |
|-------|------|
| `debian:bookworm-slim` base | ~80 MB |
| `ca-certificates` | ~0.5 MB |
| `vrc-backend` binary | ~15–25 MB (release, stripped) |
| `migrations/` | ~10 KB |
| **Total** | **~100 MB** |

---

## docker-compose.yml

```yaml
services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: vrc-backend
    restart: unless-stopped
    ports:
      - "127.0.0.1:3000:3000"
    env_file:
      - .env
    environment:
      - RUST_LOG=vrc_backend=info,tower_http=info,sqlx=warn
    depends_on:
      db:
        condition: service_healthy
    volumes:
      - gallery_data:/var/lib/vrc/gallery
    networks:
      - internal

  db:
    image: postgres:16-bookworm
    container_name: vrc-db
    restart: unless-stopped
    environment:
      POSTGRES_DB: vrc_class_reunion
      POSTGRES_USER: vrc_app
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    secrets:
      - db_password
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "127.0.0.1:5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U vrc_app -d vrc_class_reunion"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    networks:
      - internal
    shm_size: '256mb'
    command:
      - postgres
      - -c
      - shared_buffers=128MB
      - -c
      - work_mem=4MB
      - -c
      - effective_cache_size=512MB
      - -c
      - random_page_cost=1.1
      - -c
      - log_min_duration_statement=100

  caddy:
    image: caddy:2-alpine
    container_name: vrc-caddy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"  # HTTP/3 QUIC
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - app
    networks:
      - internal

volumes:
  pgdata:
    driver: local
  gallery_data:
    driver: local
  caddy_data:
    driver: local
  caddy_config:
    driver: local

secrets:
  db_password:
    file: ./secrets/db_password.txt

networks:
  internal:
    driver: bridge
```

---

## Caddyfile

```caddyfile
{
    email admin@example.com
    servers {
        protocols h1 h2 h3
    }
}

your-domain.example.com {
    # Security headers
    header {
        Strict-Transport-Security "max-age=63072000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
        Permissions-Policy "camera=(), microphone=(), geolocation=()"
        -Server
    }

    # Proxy to Axum backend
    reverse_proxy app:3000 {
        header_up X-Forwarded-For {remote_host}
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-Proto {scheme}

        # Health check
        health_uri /health
        health_interval 30s
        health_timeout 5s
    }

    # Request size limit (1 MB)
    request_body {
        max_size 1MB
    }

    # Access log
    log {
        output file /data/access.log {
            roll_size 100mb
            roll_keep 5
        }
        format json
    }
}
```

---

## Server Migration Procedure

The "easy migration" requirement is satisfied by Docker Compose — the entire stack is portable.

### Migration Steps

1. **On the old server**:
   ```bash
   # Stop the stack
   docker compose down

   # Backup database
   docker compose exec db pg_dump -U vrc_app vrc_class_reunion | gzip > backup.sql.gz

   # Export volumes (if needed)
   docker run --rm -v vrc_pgdata:/data -v $(pwd):/backup alpine \
     tar czf /backup/pgdata.tar.gz /data
   ```

2. **Transfer** to new server:
   ```bash
   scp backup.sql.gz docker-compose.yml Caddyfile .env new-server:~/vrc-backend/
   ```

3. **On the new server**:
   ```bash
   # Start fresh stack (DB creates empty)
   docker compose up -d db
   docker compose exec db pg_isready -U vrc_app

   # Restore database
   gunzip -c backup.sql.gz | docker compose exec -T db psql -U vrc_app vrc_class_reunion

   # Start application and caddy
   docker compose up -d

   # Update DNS to point to new server IP
   ```

4. **DNS Cutover**: Update A/AAAA record to new server IP. Caddy automatically obtains a new TLS certificate.

Total downtime: ~5 minutes (DNS propagation is the bottleneck).
