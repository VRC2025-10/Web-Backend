# API Design Overview

The REST API inherits the 4-layer architecture from the existing docs but refines error handling, adds health/metrics endpoints, and specifies precise Rust type mappings.

## Documents

| Document | Contents |
|----------|----------|
| [error-handling.md](./error-handling.md) | Complete error code catalog with HTTP status + Rust enum mapping |
| [rate-limiting.md](./rate-limiting.md) | Per-layer rate limiting design |
| [pagination.md](./pagination.md) | Pagination contract |

## API Layers Summary

| Layer | Base Path | Auth | Rate Limit | Cache | CSRF |
|-------|-----------|------|-----------|-------|------|
| Public | `/api/v1/public` | None | 60/min/IP | `public, max-age=30, s-w-r=60` | No |
| Internal | `/api/v1/internal` | Session Cookie | 120/min/user | `private, no-store` | Yes (Origin) |
| System | `/api/v1/system` | Bearer Token | 30/min/global | None | No |
| Auth | `/api/v1/auth` | None | 10/min/IP | None | No |
| Health | `/health` | None | None | None | No |
| Metrics | `/metrics` | None | None | None | No |

## Endpoint Catalog

### Public API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/public/members` | List public member profiles |
| GET | `/api/v1/public/members/{user_id}` | Get public member profile detail |
| GET | `/api/v1/public/events` | List events (filterable by status) |
| GET | `/api/v1/public/events/{event_id}` | Get event detail |
| GET | `/api/v1/public/clubs` | List all clubs |
| GET | `/api/v1/public/clubs/{id}` | Get club detail |
| GET | `/api/v1/public/clubs/{id}/gallery` | List approved gallery images for club |

### Internal API

| Method | Path | Min Role | Description |
|--------|------|----------|-------------|
| GET | `/api/v1/internal/auth/me` | Member | Get current user info + profile summary |
| POST | `/api/v1/internal/auth/logout` | Member | Destroy session |
| GET | `/api/v1/internal/me/profile` | Member | Get own profile |
| PUT | `/api/v1/internal/me/profile` | Member | Create/update own profile |
| GET | `/api/v1/internal/events` | Member | List events with extended_info |
| POST | `/api/v1/internal/reports` | Member | Submit a report |
| POST | `/api/v1/internal/admin/clubs` | Staff | Create a club |
| POST | `/api/v1/internal/admin/clubs/{id}/gallery` | Staff | Upload gallery image |
| PATCH | `/api/v1/internal/admin/gallery/{image_id}/status` | Staff | Approve/reject gallery image |
| GET | `/api/v1/internal/admin/users` | Admin | List all users (admin view) |
| PATCH | `/api/v1/internal/admin/users/{id}/role` | Admin | Change user role |

### System API

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/system/events` | Upsert event from external source |
| POST | `/api/v1/system/sync/users/leave` | Handle member Discord leave |

### Auth API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/auth/discord/login` | Start OAuth2 flow |
| GET | `/api/v1/auth/discord/callback` | OAuth2 callback handler |

### Infrastructure

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness + DB connectivity check |
| GET | `/metrics` | Prometheus metrics endpoint |
