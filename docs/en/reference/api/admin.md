# Admin API Reference

> **Navigation**: [Docs Home](../../README.md) > [Reference](../README.md) > [API](README.md) > Admin API

The Admin API provides administrative endpoints nested under `/api/v1/internal/admin`. All requests require a valid **session cookie** and an appropriate **role level**. Rate limit: **120 requests/min per user** (shared with Internal API). CSRF protection: `Origin` header validation.

---

## Role Requirements

| Role | Level | Can Access |
|------|-------|------------|
| `member` | 0 | — (no admin access) |
| `staff` | 1 | Reports, Clubs, Gallery |
| `admin` | 2 | All Staff endpoints + User management |
| `super_admin` | 3 | All Admin endpoints (no restrictions) |

---

## User Management

### List Users

```
GET /api/v1/internal/admin/users
```

**Required Role**: Admin+ (level ≥ 2)

Returns a paginated list of all users with administrative details.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number (min: 1) |
| `per_page` | integer | 20 | Items per page (min: 1, max: 100) |
| `status` | string | — | Filter by user status (e.g., `active`, `inactive`, `suspended`) |
| `role` | string | — | Filter by role (e.g., `member`, `staff`, `admin`, `super_admin`) |

**Response** `200 OK`

```json
{
  "items": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "discord_id": "123456789012345678",
      "discord_username": "PlayerOne",
      "role": "member",
      "status": "active",
      "joined_at": "2025-01-15T09:30:00Z",
      "last_login_at": "2026-03-18T14:00:00Z"
    }
  ],
  "total_count": 150,
  "total_pages": 8
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role (requires Admin+) |
| `ERR-VALIDATION` | 400 | Invalid query parameters |

---

### Change User Role

```
PATCH /api/v1/internal/admin/users/{id}/role
```

**Required Role**: Admin+ (level ≥ 2)

Changes the role of a user. Subject to the role authorization matrix below.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Target user ID |

**Request Body**

```json
{
  "new_role": "staff"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `new_role` | string | Yes | One of: `member`, `staff`, `admin`, `super_admin` |

**Role Change Authorization Matrix**

| Actor Role | Can Set → member | Can Set → staff | Can Set → admin | Can Set → super_admin |
|------------|:----------------:|:---------------:|:---------------:|:---------------------:|
| `admin` | ✅ | ✅ | ❌ | ❌ |
| `super_admin` | ✅ | ✅ | ✅ | ❌ |

Additional constraints:
- **Self-demotion forbidden** — An admin cannot change their own role (`ERR-PERM-002`)
- **Admin escalation by non-super_admin** — Only `super_admin` can promote to `admin` (`ERR-ROLE-001`)
- **Super admin escalation** — No one can promote to `super_admin` via API (`ERR-ROLE-002`)
- **Super admin protected** — The `super_admin` role cannot be demoted via API (`ERR-ROLE-003`)
- **Level check** — Actor's role level must be higher than the target's current level (`ERR-ROLE-004`)

**Response** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "previous_role": "member",
  "new_role": "staff",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T12:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-PERM-002` | 403 | Cannot change own role |
| `ERR-ROLE-001` | 403 | Admin escalation not allowed by non-super_admin |
| `ERR-ROLE-002` | 403 | Cannot promote to super_admin via API |
| `ERR-ROLE-003` | 403 | Super admin role is protected |
| `ERR-ROLE-004` | 403 | Actor role level insufficient for target |
| `ERR-USER-001` | 404 | User not found |
| `ERR-CSRF-001` | 403 | CSRF validation failed |

---

### Change User Status

```
PATCH /api/v1/internal/admin/users/{id}/status
```

**Required Role**: Admin+ (level ≥ 2)

Changes the status of a user (e.g., suspend or reactivate).

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Target user ID |

**Request Body**

```json
{
  "new_status": "suspended"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `new_status` | string | Yes | One of: `active`, `inactive`, `suspended` |

**Response** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "previous_status": "active",
  "new_status": "suspended",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T12:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-USER-001` | 404 | User not found |
| `ERR-CSRF-001` | 403 | CSRF validation failed |

---

## Reports

### List Reports

```
GET /api/v1/internal/admin/reports
```

**Required Role**: Staff+ (level ≥ 1)

Returns a paginated list of all reports.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number (min: 1) |
| `per_page` | integer | 20 | Items per page (min: 1, max: 100) |

**Response** `200 OK`

```json
{
  "items": [
    {
      "report_id": "550e8400-e29b-41d4-a716-446655440099",
      "reporter_id": "550e8400-e29b-41d4-a716-446655440010",
      "reporter_discord_username": "ReporterUser",
      "target_type": "user",
      "target_id": "550e8400-e29b-41d4-a716-446655440000",
      "reason": "Inappropriate behavior during the event.",
      "status": "pending",
      "created_at": "2026-03-19T12:00:00Z",
      "resolved_at": null,
      "resolved_by": null
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role (requires Staff+) |

---

### Resolve Report

```
PATCH /api/v1/internal/admin/reports/{id}
```

**Required Role**: Staff+ (level ≥ 1)

Updates the status of a report (e.g., resolve or dismiss).

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Report ID |

**Request Body**

```json
{
  "new_status": "resolved"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `new_status` | string | Yes | One of: `resolved`, `dismissed` |

**Response** `200 OK`

```json
{
  "report_id": "550e8400-e29b-41d4-a716-446655440099",
  "previous_status": "pending",
  "new_status": "resolved",
  "resolved_by": "550e8400-e29b-41d4-a716-446655440001",
  "resolved_at": "2026-03-19T14:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-CSRF-001` | 403 | CSRF validation failed |

---

## Clubs

### Create Club

```
POST /api/v1/internal/admin/clubs
```

**Required Role**: Staff+ (level ≥ 1)

Creates a new club.

**Request Body**

```json
{
  "name": "Photography Club",
  "description_markdown": "A club for VRChat photographers.",
  "cover_image_url": "https://cdn.discordapp.com/attachments/.../photo-club.jpg"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | string | Yes | 1–100 characters; trimmed |
| `description_markdown` | string | Yes | 1–5000 characters |
| `cover_image_url` | string \| null | No | Must be a valid HTTPS URL; SSRF prevention applied (see below) |

**SSRF Prevention for URLs**

All user-supplied URLs are validated:
- Must use `https://` scheme
- Domain must be on an allowlist (e.g., `cdn.discordapp.com`)
- No private/internal IP ranges (`10.x`, `172.16.x`, `192.168.x`, `127.x`, `::1`)
- No redirects followed during validation

**Response** `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "Photography Club",
  "description_markdown": "A club for VRChat photographers.",
  "cover_image_url": "https://cdn.discordapp.com/attachments/.../photo-club.jpg",
  "created_at": "2026-03-19T12:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-CSRF-001` | 403 | CSRF validation failed |
| `ERR-VALIDATION` | 400 | Validation failed |

---

## Gallery

### Upload Gallery Image

```
POST /api/v1/internal/admin/clubs/{id}/gallery
```

**Required Role**: Staff+ (level ≥ 1)

Adds an image to a club's gallery.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Club ID |

**Request Body**

```json
{
  "image_url": "https://cdn.discordapp.com/attachments/.../gallery-photo.jpg",
  "caption": "Sunset over the virtual mountains"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `image_url` | string | Yes | Must be a valid HTTPS URL; SSRF prevention applied |
| `caption` | string \| null | No | Maximum 500 characters |

**Response** `201 Created`

```json
{
  "image_id": "550e8400-e29b-41d4-a716-446655440010",
  "club_id": "550e8400-e29b-41d4-a716-446655440001",
  "image_url": "https://cdn.discordapp.com/attachments/.../gallery-photo.jpg",
  "caption": "Sunset over the virtual mountains",
  "status": "pending",
  "uploaded_at": "2026-03-19T12:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-CLUB-001` | 404 | Club not found |
| `ERR-CSRF-001` | 403 | CSRF validation failed |
| `ERR-VALIDATION` | 400 | Validation failed |

---

### Change Gallery Image Status

```
PATCH /api/v1/internal/admin/gallery/{image_id}/status
```

**Required Role**: Staff+ (level ≥ 1)

Approves, rejects, or removes a gallery image.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `image_id` | string (UUID) | Gallery image ID |

**Request Body**

```json
{
  "new_status": "approved"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `new_status` | string | Yes | One of: `approved`, `rejected`, `removed` |

**Response** `200 OK`

```json
{
  "image_id": "550e8400-e29b-41d4-a716-446655440010",
  "previous_status": "pending",
  "new_status": "approved",
  "changed_by": "550e8400-e29b-41d4-a716-446655440001",
  "changed_at": "2026-03-19T14:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PERM-001` | 403 | Insufficient role |
| `ERR-GAL-001` | 404 | Gallery image not found |
| `ERR-GAL-002` | 400 | Invalid status transition (e.g., `approved` → `pending`) |
| `ERR-CSRF-001` | 403 | CSRF validation failed |

---

## Related Documents

- [API Overview](README.md) — Base URL, auth methods, rate limiting
- [Internal API](internal.md) — Non-admin authenticated endpoints
- [Error Catalog](../errors.md) — Complete error code reference
