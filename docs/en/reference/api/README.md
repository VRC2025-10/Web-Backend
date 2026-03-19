# API Overview

> **Navigation**: [Docs Home](../../README.md) > [Reference](../README.md) > API Overview

Complete overview of the VRC Web-Backend REST API including base URL, response format, authentication methods, pagination, error handling, rate limiting, and security headers.

---

## Base URL

```
https://your-domain.example.com/api/v1
```

In local development:

```
http://localhost:3000/api/v1
```

Infrastructure endpoints (`/health`, `/metrics`) are served at the root without the `/api/v1` prefix.

---

## Response Format

All API responses use JSON (`Content-Type: application/json`) unless otherwise noted.

### Success Response

Endpoints return the resource directly in the response body:

```json
{
  "id": "...",
  "field": "value"
}
```

### Paginated Response

List endpoints return a `PageResponse<T>` wrapper:

```json
{
  "items": [...],
  "total_count": 42,
  "total_pages": 3
}
```

Response headers:

| Header | Description |
|--------|-------------|
| `x-total-count` | Total number of items across all pages |
| `x-total-pages` | Total number of pages |

### Error Response

All errors follow a consistent envelope:

```json
{
  "error": {
    "code": "ERR-XXX-NNN",
    "message": "Human-readable message in Japanese",
    "details": {}
  }
}
```

The `details` field is optional and provides additional context (e.g., field-level validation errors).

---

## Pagination

All list endpoints accept:

| Parameter | Type | Default | Min | Max | Description |
|-----------|------|---------|-----|-----|-------------|
| `page` | integer | 1 | 1 | — | Page number |
| `per_page` | integer | 20 | 1 | 100 | Items per page |

---

## Authentication Methods

The API uses three distinct authentication mechanisms depending on the endpoint category:

| Method | Used By | Mechanism |
|--------|---------|-----------|
| **None** | Public API, Auth API | No authentication required |
| **Session Cookie** | Internal API (including Admin) | `session_id` cookie set after Discord OAuth2 login |
| **Bearer Token** | System API, Metrics | `Authorization: Bearer <SYSTEM_API_TOKEN>` header |

### Session Cookie Authentication

After successful Discord OAuth2 login, the server sets an HTTP-only, secure session cookie. The cookie is automatically sent with subsequent requests. CSRF protection is enforced via `Origin` header validation.

### Bearer Token Authentication

System-to-system endpoints require a pre-shared token in the `Authorization` header:

```
Authorization: Bearer <token>
```

---

## Security Headers

All responses include:

| Header | Value |
|--------|-------|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |

### CORS Policy

- **Allowed Origin**: Value of `FRONTEND_ORIGIN` environment variable
- **Allowed Methods**: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`
- **Allowed Headers**: `Content-Type`, `Authorization`
- **Credentials**: `true` (cookies are sent cross-origin)

### CSRF Protection

Internal API endpoints validate the `Origin` header against `FRONTEND_ORIGIN`. Requests without a matching `Origin` are rejected with `ERR-CSRF-001` (403).

---

## Rate Limiting

Rate limits are enforced per tier:

| Tier | Scope | Limit | Applied To |
|------|-------|-------|------------|
| Public | Per IP | 60 requests/min | Public API |
| Auth | Per IP | 10 requests/min | Auth API |
| Internal | Per user | 120 requests/min | Internal API (including Admin) |
| System | Global | 30 requests/min | System API |

When a rate limit is exceeded, the server returns `429 Too Many Requests` with error code `ERR-RATELIMIT-001`.

### Rate Limit Headers

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed in the window |
| `X-RateLimit-Remaining` | Remaining requests in the current window |
| `X-RateLimit-Reset` | Unix timestamp when the window resets |
| `Retry-After` | Seconds until the next request is allowed (on 429 only) |

---

## Caching

| Tier | Cache-Control |
|------|--------------|
| Public API | `public, max-age=30` |
| Internal API | `private, no-cache` |
| System API | `no-store` |

---

## Endpoint Summary

### Infrastructure

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | None | Health check |
| GET | `/metrics` | Bearer | Prometheus metrics |

### Auth API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/auth/discord/login` | None | Initiate Discord OAuth2 |
| GET | `/api/v1/auth/discord/callback` | None | OAuth2 callback |

### Public API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/public/members` | None | List members |
| GET | `/api/v1/public/members/{discord_id}` | None | Get member detail |
| GET | `/api/v1/public/events` | None | List events |
| GET | `/api/v1/public/events/{event_id}` | None | Get event detail |
| GET | `/api/v1/public/clubs` | None | List clubs |
| GET | `/api/v1/public/clubs/{id}` | None | Get club detail |
| GET | `/api/v1/public/clubs/{id}/gallery` | None | List club gallery images |

### Internal API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/internal/auth/me` | Session | Get current user info |
| POST | `/api/v1/internal/auth/logout` | Session | Logout |
| GET | `/api/v1/internal/me/profile` | Session | Get own profile |
| PUT | `/api/v1/internal/me/profile` | Session | Update own profile |
| GET | `/api/v1/internal/events` | Session | List events (internal) |
| POST | `/api/v1/internal/reports` | Session | Submit a report |

### Admin API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/internal/admin/users` | Session (Admin+) | List users |
| PATCH | `/api/v1/internal/admin/users/{id}/role` | Session (Admin+) | Change user role |
| PATCH | `/api/v1/internal/admin/users/{id}/status` | Session (Admin+) | Change user status |
| GET | `/api/v1/internal/admin/reports` | Session (Staff+) | List reports |
| PATCH | `/api/v1/internal/admin/reports/{id}` | Session (Staff+) | Resolve report |
| POST | `/api/v1/internal/admin/clubs` | Session (Staff+) | Create club |
| POST | `/api/v1/internal/admin/clubs/{id}/gallery` | Session (Staff+) | Upload gallery image |
| PATCH | `/api/v1/internal/admin/gallery/{image_id}/status` | Session (Staff+) | Change gallery status |

### System API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/system/events` | Bearer | Upsert event |
| POST | `/api/v1/system/sync/users/leave` | Bearer | Process member leave |

---

## Related Documents

- [Public API](public.md) — Unauthenticated public endpoints
- [Internal API](internal.md) — Session-authenticated user endpoints
- [Admin API](admin.md) — Administrative endpoints
- [System API](system.md) — Machine-to-machine endpoints
- [Auth API](auth.md) — Discord OAuth2 flow
- [Error Catalog](../errors.md) — All error codes and resolutions
- [Configuration Reference](../configuration.md) — Environment variables
