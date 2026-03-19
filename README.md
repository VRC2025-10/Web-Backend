# VRC Web-Backend

[![CI](https://github.com/VRC2025-10/Web-Backend/actions/workflows/ci.yml/badge.svg)](https://github.com/VRC2025-10/Web-Backend/actions/workflows/ci.yml)
[![Security](https://github.com/VRC2025-10/Web-Backend/actions/workflows/security.yml/badge.svg)](https://github.com/VRC2025-10/Web-Backend/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL: 16](https://img.shields.io/badge/PostgreSQL-16-336791.svg)](https://www.postgresql.org/)

A high-performance, type-safe REST API backend for the VRChat Class Reunion community platform. Built with Rust and Axum, featuring compile-time SQL verification, type-state authorization, and formal verification with Kani.

## Overview

VRC Web-Backend powers the community website for VRChat October Class Reunion events. It provides:

- **Discord OAuth2 Authentication** — seamless login via Discord with guild membership verification
- **Role-Based Access Control** — type-state authorization (`super_admin` → `admin` → `staff` → `member`)
- **Public API** — unauthenticated access to member profiles, events, clubs, and galleries
- **Internal API** — authenticated endpoints for profile management and administration
- **System API** — machine-to-machine integration for external tools (GAS, Discord bots)
- **Real-time Notifications** — Discord webhook integration for events, reports, and moderation

### Architecture

```
[Browser]  ──→ Public API    (no auth, cached — member/event/club/gallery browsing)
           ──→ Internal API  (session cookie — profile editing, admin features)
           ──→ Auth API      (Discord OAuth2 login flow)

[GAS/Bot]  ──→ System API    (Bearer token — M2M integration)
```

The backend follows **hexagonal architecture** (ports and adapters) with strict layer separation:

```
src/
├── domain/          # Business logic, entities, value objects, port traits
│   ├── entities/    # User, Profile, Event, Club, Gallery, Session, Report
│   ├── ports/       # Repository & service trait definitions
│   └── value_objects/
├── adapters/
│   ├── inbound/     # HTTP routes, extractors, middleware (Axum)
│   └── outbound/    # PostgreSQL repos, Discord client, markdown renderer
├── auth/            # Session extraction, role verification
├── background/      # Scheduled tasks (session cleanup, event archival)
├── config/          # Environment-based configuration
└── errors/          # Algebraic error types per API layer
```

### Key Design Decisions

- **Compile-Time SQL** — every query verified against the live schema via SQLx offline mode
- **Zero-Copy Hot Paths** — public API responses use `Bytes` and `Cow<'_, str>` on cache hits
- **Tower Middleware Calculus** — auth, rate limiting, CORS, and caching compose as typed Tower layers
- **Custom `#[handler]` Macro** — generates route registration and permission checks from annotations
- **Lock-Free Rate Limiting** — per-IP token bucket via atomic operations (governor)
- **Property-Based Testing** — all input validators tested with proptest
- **Formal Verification** — critical domain logic verified with Kani bounded model checking

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) 1.85+ (Edition 2024)
- [PostgreSQL](https://www.postgresql.org/) 16+
- [Docker](https://docs.docker.com/get-docker/) & Docker Compose (optional, for containerized setup)
- A [Discord Application](https://discord.com/developers/applications) with OAuth2 configured

### Using Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/VRC2025-10/Web-Backend.git
cd Web-Backend

# Configure environment
cp .env.example .env
# Edit .env with your Discord OAuth2 credentials and other settings

# Start PostgreSQL
docker compose up -d

# Run the backend
cargo run
```

### Manual Setup

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and configure
git clone https://github.com/VRC2025-10/Web-Backend.git
cd Web-Backend
cp .env.example .env
# Edit .env — see docs/setup.md for all configuration options

# Start PostgreSQL (must be running)
# Ensure DATABASE_URL in .env points to your PostgreSQL instance

# Build and run
cargo build --release
cargo run --release
```

The server starts on `http://localhost:8080` by default (configurable via `BIND_ADDRESS`).

## Usage

### API Layers

| Layer | Path | Auth | Purpose |
|-------|------|------|---------|
| Public | `/api/v1/public/*` | None | Member profiles, events, clubs, galleries |
| Internal | `/api/v1/internal/*` | Session Cookie | Profile editing, admin features |
| System | `/api/v1/system/*` | Bearer Token | External system integration |
| Auth | `/api/v1/auth/*` | None | Discord OAuth2 login flow |
| Health | `/health` | None | Health check endpoint |

### Example Requests

```bash
# Health check
curl http://localhost:8080/health

# List public members
curl http://localhost:8080/api/v1/public/members?page=1&per_page=20

# List published events
curl http://localhost:8080/api/v1/public/events?status=published

# System API — sync events (requires SYSTEM_API_TOKEN)
curl -X POST http://localhost:8080/api/v1/system/events \
  -H "Authorization: Bearer $SYSTEM_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"external_source_id": "evt_001", "title": "October Reunion", ...}'
```

### Configuration

All configuration is via environment variables. See [`.env.example`](.env.example) for the full list or the [setup guide](docs/setup.md) for detailed descriptions.

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection URL |
| `DISCORD_CLIENT_ID` | Yes | Discord OAuth2 client ID |
| `DISCORD_CLIENT_SECRET` | Yes | Discord OAuth2 client secret |
| `DISCORD_REDIRECT_URI` | Yes | OAuth2 callback URL |
| `DISCORD_GUILD_ID` | Yes | Target Discord guild ID |
| `SESSION_SECRET` | Yes | Session signing secret (≥32 chars) |
| `SYSTEM_API_TOKEN` | Yes | System API auth token (≥64 chars) |
| `FRONTEND_ORIGIN` | Yes | Frontend URL for CORS |

## Documentation

Full documentation is available in both English and Japanese:

| | English | 日本語 |
|---|---|---|
| **Documentation Hub** | [docs/en/](docs/en/README.md) | [docs/ja/](docs/ja/README.md) |
| Architecture | [Architecture](docs/en/architecture/README.md) | [アーキテクチャ](docs/ja/architecture/README.md) |
| Getting Started | [Getting Started](docs/en/getting-started/README.md) | [はじめに](docs/ja/getting-started/README.md) |
| API Reference | [Reference](docs/en/reference/README.md) | [リファレンス](docs/ja/reference/README.md) |
| Guides | [Guides](docs/en/guides/README.md) | [ガイド](docs/ja/guides/README.md) |
| Development | [Development](docs/en/development/README.md) | [開発](docs/ja/development/README.md) |
| Design Decisions | [Design](docs/en/design/README.md) | [設計](docs/ja/design/README.md) |

### Legacy / Quick Links

- [Setup Guide](docs/setup.md) — environment variables, build instructions, auth configuration
- [API Reference](docs/api/README.md) — full endpoint specifications with request/response examples
- [System Specification](specs/README.md) — architecture, requirements, threat model, and more

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (Edition 2024) |
| Runtime | Tokio |
| Framework | Axum 0.8 |
| Database | PostgreSQL 16 + SQLx 0.8 |
| Auth | Discord OAuth2 + server-side sessions |
| Security | ammonia (XSS), governor (rate limiting), subtle (timing-safe) |
| Observability | tracing + Prometheus metrics |
| Reverse Proxy | Caddy 2 (automatic HTTPS, HTTP/3) |
| Container | Docker (multi-stage build) |

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) before submitting a pull request.

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)

## License

This project is licensed under the [MIT License](LICENSE).
