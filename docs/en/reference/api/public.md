# Public API Reference

> **Navigation**: [Docs Home](../../README.md) > [Reference](../README.md) > [API](README.md) > Public API

The Public API provides read-only access to community data without authentication. All endpoints are rate-limited to **60 requests/min per IP** and served with `Cache-Control: public, max-age=30`.

---

## Members

### List Members

```
GET /api/v1/public/members
```

Returns a paginated list of public member summaries.

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
      "discord_id": "123456789012345678",
      "nickname": "PlayerOne",
      "avatar_url": "https://cdn.discordapp.com/avatars/...",
      "joined_at": "2025-01-15T09:30:00Z"
    }
  ],
  "total_count": 42,
  "total_pages": 3
}
```

**Response Headers**

| Header | Example |
|--------|---------|
| `x-total-count` | `42` |
| `x-total-pages` | `3` |

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-VALIDATION` | 400 | Invalid pagination parameters |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

### Get Member Detail

```
GET /api/v1/public/members/{discord_id}
```

Returns the public profile of a specific member.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `discord_id` | string | Discord user ID |

**Response** `200 OK`

```json
{
  "discord_id": "123456789012345678",
  "nickname": "PlayerOne",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "player_one",
  "bio_markdown": "Hello! I'm a VRChat enthusiast.",
  "joined_at": "2025-01-15T09:30:00Z",
  "role": "member"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-PROF-003` | 404 | Member not found or profile is not public |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Events

### List Events

```
GET /api/v1/public/events
```

Returns a paginated list of events. Defaults to published events.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number (min: 1) |
| `per_page` | integer | 20 | Items per page (min: 1, max: 100) |
| `status` | string | `published` | Filter by event status |

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
  "total_count": 15,
  "total_pages": 1
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-VALIDATION` | 400 | Invalid pagination or status parameter |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

### Get Event Detail

```
GET /api/v1/public/events/{event_id}
```

Returns the details of a specific event.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `event_id` | string (UUID) | Event identifier |

**Response** `200 OK`

```json
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
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-EVT-001` | 404 | Event not found |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Clubs

### List Clubs

```
GET /api/v1/public/clubs
```

Returns a paginated list of clubs.

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
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "name": "Photography Club",
      "description_markdown": "A club for VRChat photographers.",
      "cover_image_url": "https://example.com/images/photo-club.jpg",
      "member_count": 12,
      "created_at": "2025-06-01T12:00:00Z"
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-VALIDATION` | 400 | Invalid pagination parameters |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

### Get Club Detail

```
GET /api/v1/public/clubs/{id}
```

Returns the details of a specific club.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Club identifier |

**Response** `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "Photography Club",
  "description_markdown": "A club for VRChat photographers.\n\n## Activities\n- Weekly photo walks\n- Monthly contests",
  "cover_image_url": "https://example.com/images/photo-club.jpg",
  "member_count": 12,
  "created_at": "2025-06-01T12:00:00Z",
  "updated_at": "2026-02-10T08:30:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-CLUB-001` | 404 | Club not found |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

### List Club Gallery

```
GET /api/v1/public/clubs/{id}/gallery
```

Returns a paginated list of approved gallery images for a club.

**Path Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string (UUID) | Club identifier |

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
      "image_id": "550e8400-e29b-41d4-a716-446655440010",
      "image_url": "https://example.com/gallery/photo1.jpg",
      "caption": "Sunset over the virtual mountains",
      "uploaded_at": "2026-01-15T14:30:00Z"
    }
  ],
  "total_count": 8,
  "total_pages": 1
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-CLUB-001` | 404 | Club not found |
| `ERR-VALIDATION` | 400 | Invalid pagination parameters |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Related Documents

- [API Overview](README.md) — Base URL, auth methods, pagination conventions
- [Internal API](internal.md) — Authenticated endpoints for logged-in users
- [Error Catalog](../errors.md) — Complete error code reference
