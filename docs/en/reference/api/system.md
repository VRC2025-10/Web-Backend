# System API Reference

> **Navigation**: [Docs Home](../../README.md) > [Reference](../README.md) > [API](README.md) > System API

The System API provides machine-to-machine endpoints for external services (e.g., Discord bots, Google Apps Script) to synchronize data with the backend. All requests require **Bearer token** authentication. Rate limit: **30 requests/min global**.

---

## Authentication

All System API requests must include the `Authorization` header:

```
Authorization: Bearer <SYSTEM_API_TOKEN>
```

The token is configured via the `SYSTEM_API_TOKEN` environment variable and must be at least 64 characters.

**Authentication Errors**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-SYNC-001` | 401 | Missing, invalid, or mismatched Bearer token |

---

## Events

### Upsert Event

```
POST /api/v1/system/events
```

Creates a new event or updates an existing one. Events are matched by `external_id` — if an event with the given `external_id` exists, it is updated; otherwise, a new event is created.

**Request Body**

```json
{
  "external_id": "evt-discord-001",
  "title": "VRChat Weekly Meetup",
  "description_markdown": "Join us for our weekly community meetup!\n\n## Details\n- Voice chat in Discord\n- Meet in-world",
  "status": "published",
  "host_discord_id": "123456789012345678",
  "start_time": "2026-03-20T20:00:00Z",
  "end_time": "2026-03-20T22:00:00Z",
  "location": "VRChat - Community World",
  "tags": ["meetup", "weekly"]
}
```

**Field Validation Rules**

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `external_id` | string | Yes | 1–255 characters; unique identifier from the source system |
| `title` | string | Yes | 1–200 characters; trimmed |
| `description_markdown` | string \| null | No | Maximum 5000 characters |
| `status` | string | Yes | One of: `draft`, `published`, `cancelled`, `archived` |
| `host_discord_id` | string \| null | No | Valid Discord user ID format (snowflake) |
| `start_time` | string (ISO 8601) | Yes | Must be a valid UTC datetime |
| `end_time` | string (ISO 8601) \| null | No | Must be after `start_time` when provided |
| `location` | string \| null | No | Maximum 500 characters |
| `tags` | string[] \| null | No | Each tag: 1–50 characters; maximum 20 tags per event |

**Response** `200 OK` (updated) / `201 Created` (new)

```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "external_id": "evt-discord-001",
  "title": "VRChat Weekly Meetup",
  "description_markdown": "Join us for our weekly community meetup!\n\n## Details\n- Voice chat in Discord\n- Meet in-world",
  "status": "published",
  "host_discord_id": "123456789012345678",
  "start_time": "2026-03-20T20:00:00Z",
  "end_time": "2026-03-20T22:00:00Z",
  "location": "VRChat - Community World",
  "tags": ["meetup", "weekly"],
  "created_at": "2026-03-15T10:00:00Z",
  "updated_at": "2026-03-19T08:30:00Z"
}
```

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-SYNC-001` | 401 | Invalid system API token |
| `ERR-SYNC-002` | 400 | Validation failed (details in `error.details`) |
| `ERR-VALIDATION` | 400 | Malformed request body |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

**Example Error Response**

```json
{
  "error": {
    "code": "ERR-SYNC-002",
    "message": "システムAPIのバリデーションに失敗しました",
    "details": {
      "title": "タイトルは必須です",
      "end_time": "終了時刻は開始時刻より後である必要があります"
    }
  }
}
```

---

## User Sync

### Process Member Leave

```
POST /api/v1/system/sync/users/leave
```

Processes a member leaving the Discord guild. This is an **atomic operation** that:

1. Marks the user as inactive
2. Hides their public profile
3. Records the leave reason and timestamp

If the user does not exist in the system, the request is treated as a no-op and returns `204`.

**Request Body**

```json
{
  "discord_id": "123456789012345678",
  "reason": "left_guild"
}
```

**Field Validation Rules**

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `discord_id` | string | Yes | Valid Discord user ID format (snowflake) |
| `reason` | string \| null | No | Maximum 500 characters; one of: `left_guild`, `kicked`, `banned`, or free text |

**Response** `200 OK` (user found and processed)

```json
{
  "discord_id": "123456789012345678",
  "status": "inactive",
  "processed_at": "2026-03-19T12:00:00Z"
}
```

**Response** `204 No Content` (user not found — no-op)

No response body.

**Error Codes**

| Code | Status | Cause |
|------|--------|-------|
| `ERR-SYNC-001` | 401 | Invalid system API token |
| `ERR-SYNC-002` | 400 | Validation failed |
| `ERR-RATELIMIT-001` | 429 | Rate limit exceeded |

---

## Related Documents

- [API Overview](README.md) — Base URL, auth methods, rate limiting
- [Configuration Reference](../configuration.md) — `SYSTEM_API_TOKEN` configuration
- [Error Catalog](../errors.md) — Complete error code reference
