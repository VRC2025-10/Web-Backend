# CI/CD Pipeline Design

## Pipeline: GitHub Actions

### Trigger Strategy

| Event | Pipeline |
|-------|----------|
| Push to `main` | Full pipeline → deploy to production |
| Pull request to `main` | CI only (lint + test + build) — no deploy |
| Tag `v*.*.*` | Full pipeline + create GitHub Release |
| Manual dispatch | Deploy to production (with rollback option) |

### Pipeline Stages

```mermaid
graph LR
    A["Lint<br/>(clippy + fmt)"] --> B["Test<br/>(unit + integration)"]
    B --> C["Security Audit<br/>(cargo audit)"]
    C --> D["Build Image<br/>(Docker)"]
    D --> E["Push Image<br/>(Registry)"]
    E --> F["Deploy<br/>(Docker Compose)"]
    F --> G["Verify<br/>(Health Check)"]
    G --> H["Notify<br/>(Discord Webhook)"]

    style A fill:#e3f2fd
    style B fill:#e3f2fd
    style C fill:#fff3e0
    style D fill:#e8f5e9
    style E fill:#e8f5e9
    style F fill:#fce4ec
    style G fill:#fce4ec
    style H fill:#f3e5f5
```

### GitHub Actions Workflow

```yaml
name: CI/CD

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always
  SQLX_OFFLINE: true
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_DB: vrc_class_reunion_test
          POSTGRES_USER: vrc_app
          POSTGRES_PASSWORD: test_password
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    env:
      DATABASE_URL: postgres://vrc_app:test_password@localhost:5432/vrc_class_reunion_test
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run migrations
        run: cargo install sqlx-cli --no-default-features --features rustls,postgres && sqlx migrate run
      - name: Run tests
        run: cargo test --all-features -- --test-threads=1

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  build:
    name: Build Image
    needs: [lint, test, security]
    if: github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/metadata-action@v5
        id: meta
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=sha,prefix=
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=raw,value=latest,enable={{is_default_branch}}
      - uses: docker/build-push-action@v6
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy:
    name: Deploy
    needs: [build]
    if: github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    environment: production
    steps:
      - name: Deploy via SSH
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.DEPLOY_HOST }}
          username: ${{ secrets.DEPLOY_USER }}
          key: ${{ secrets.DEPLOY_SSH_KEY }}
          script: |
            cd /opt/vrc-backend
            docker compose pull app
            docker compose up -d app
            sleep 5
            curl -sf http://localhost:3000/health || (docker compose logs app --tail 50 && exit 1)

  notify:
    name: Notify
    needs: [deploy]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - name: Discord Notification
        uses: sarisia/actions-status-discord@v1
        if: always()
        with:
          webhook: ${{ secrets.DISCORD_DEPLOY_WEBHOOK }}
          status: ${{ job.status }}
          title: "VRC Backend Deployment"
          description: |
            **Commit:** ${{ github.sha }}
            **Branch:** ${{ github.ref_name }}
            **Actor:** ${{ github.actor }}
```

### Rollback Procedure

```bash
# On the production server
ssh deploy@production-server

# List available image tags
docker image ls ghcr.io/<repo>/vrc-backend

# Rollback to previous version
cd /opt/vrc-backend
docker compose pull app  # or: docker compose up -d app --pull never
# Edit docker-compose.yml to pin specific image tag, then:
docker compose up -d app

# Verify
curl -sf http://localhost:3000/health
```

### SQLx Offline Mode in CI

Since CI tests need a live database for integration tests but Docker builds need offline mode:

1. **Development**: Run `cargo sqlx prepare` after schema changes → generates `sqlx-data.json`
2. **CI tests**: Use live PostgreSQL service container (full verification)
3. **Docker build**: Set `SQLX_OFFLINE=true` (uses `sqlx-data.json`, no DB needed)

The `sqlx-data.json` file is committed to version control and must be regenerated whenever SQL queries change.
