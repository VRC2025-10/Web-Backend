# Internal API Reference

> **Navigation**: [Docs Home](../../README.md) > [Reference](../README.md) > [API](README.md) > Internal API

The Internal API provides endpoints for authenticated users. All requests require a valid **session cookie** set after Discord OAuth2 login. Rate limit: **120 requests/min per user**. CSRF protection: `Origin` header must match `FRONTEND_ORIGIN`.

---

## Authentication

### Get Current User

```
GET /api/v1/internal/auth/me
```

Returns the current authenticated user's information and profile summary.

**Response** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "discord_id": "123456789012345678",
  "discord_username": "PlayerOne",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "role": "member",
  "status": "active",
  "joined_at": "2025-01-15T09:30:00Z",
  "profile": {
    "nickname": "PlayerOne",
    "bio_markdown": "Hello!",
    "is_public": true
  }
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-AUTH-004` | 403 | Account suspended |

---

### Logout

```
POST /api/v1/internal/auth/logout
```

Destroys the current session and clears the session cookie.

**Response** `204 No Content`

No response body.

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-CSRF-001` | 403 | CSRF validation failed |

---

## Profile

### Get Own Profile

```
GET /api/v1/internal/me/profile
```

Returns the full profile of the authenticated user.

**Response** `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "PlayerOne",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "player_one",
  "bio_markdown": "VRChat enthusiast and world explorer.",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true,
  "updated_at": "2026-03-01T10:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-PROF-003` | 404 | Profile not found |

---

### Update Own Profile

```
PUT /api/v1/internal/me/profile
```

Updates the authenticated user's profile. All fields in the request body are optional — only provided fields are updated.

**Request Body**

```json
{
  "nickname": "NewNickname",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "new_x_handle",
  "bio_markdown": "Updated bio text.",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true
}
```

**Field Validation Rules**

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `nickname` | string \| null | No | 1–50 characters when provided; trimmed; no leading/trailing whitespace |
| `vrc_id` | string \| null | No | Must match VRChat user ID format: `usr_` followed by a UUID (`usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`) |
| `x_id` | string \| null | No | 1–15 characters; alphanumeric and underscores only (`[a-zA-Z0-9_]`) |
| `bio_markdown` | string \| null | No | Maximum 2000 characters; scanned for dangerous content (scripts, data URIs, etc.) |
| `avatar_url` | string \| null | No | Must be a valid HTTPS URL; only allowed domains (Discord CDN); SSRF prevention applied |
| `is_public` | boolean | No | Controls whether the profile is visible via the Public API |

**Response** `200 OK`

Returns the updated profile with the same shape as [Get Own Profile](#get-own-profile).

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-CSRF-001` | 403 | CSRF validation failed |
| `ERR-PROF-001` | 400 | Profile validation failed (field-level details in `error.details`) |
| `ERR-PROF-002` | 400 | Dangerous content detected in `bio_markdown` |
| `ERR-PROF-003` | 404 | Profile not found |

**Example Error Response (Validation)**

```json
{
  "error": {
    "code": "ERR-PROF-001",
    "message": "プロフィールのバリデーションに失敗しました",
    "details": {
      "nickname": "50文字以内で入力してください",
      "vrc_id": "VRChat IDの形式が正しくありません"
    }
  }
}
```

---

## Events

### List Events (Internal)

```
GET /api/v1/internal/events
```

Returns a paginated list of events visible to authenticated users. May include additional statuses not available via the Public API.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number (min: 1) |
| `per_page` | integer | 20 | Items per page (min: 1, max: 100) |
| `status` | string | — | Filter by event status |

**Response** `200 OK`

```json
{
  "items": [
    {
      "event_id": "550e8400-e29b-41d4-a716-446655440000",
      "external_id": "evt-discord-001",
      "title": "VRChat Weekly Meetup",
      "description_markdown": "Join us for our weekly community meetup!",
      "status": "published",
      "host_discord_id": "123456789012345678",
      "start_time": "2026-03-20T20:00:00Z",
      "end_time": "2026-03-20T22:00:00Z",
      "location": "VRChat - Community World",
      "tags": ["meetup", "weekly"]
    }
  ],
  "total_count": 20,
  "total_pages": 1
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-VALIDATION` | 400 | Invalid query parameters |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Reports

### Submit a Report

```
POST /api/v1/internal/reports
```

Submits a report against a user, event, club, or other entity. Duplicate reports (same reporter + same target) are rejected.

**Request Body**

```json
{
  "target_type": "user",
  "target_id": "550e8400-e29b-41d4-a716-446655440000",
  "reason": "Inappropriate behavior during the event."
}
```

**Field Validation Rules**

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `target_type` | string | Yes | One of: `user`, `event`, `club`, `gallery_image` |
| `target_id` | string (UUID) | Yes | Must reference an existing entity |
| `reason` | string | Yes | 10–1000 characters |

**Response** `201 Created`

```json
{
  "report_id": "550e8400-e29b-41d4-a716-446655440099",
  "target_type": "user",
  "target_id": "550e8400-e29b-41d4-a716-446655440000",
  "reason": "Inappropriate behavior during the event.",
  "status": "pending",
  "created_at": "2026-03-19T12:00:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-AUTH-003` | 401 | Invalid or expired session |
| `ERR-CSRF-001` | 403 | CSRF validation failed |
| `ERR-MOD-001` | 404 | Report target not found |
| `ERR-MOD-002` | 409 | Duplicate report (same reporter and target) |
| `ERR-MOD-003` | 400 | Reason length out of range (10–1000 characters) |
| `ERR-VALIDATION` | 400 | Invalid target_type or missing required fields |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Related Documents

- [API Overview](README.md) — Base URL, pagination, auth methods
- [Admin API](admin.md) — Administrative endpoints (requires Admin+ or Staff+)
- [Auth API](auth.md) — Discord OAuth2 login flow
- [Error Catalog](../errors.md) — Complete error code reference
