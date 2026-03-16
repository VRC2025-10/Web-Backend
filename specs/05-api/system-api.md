# System API Endpoints

System endpoints are called by external automated systems (Google Apps Script, Discord Bot). Authentication is via a shared Bearer token (`SYSTEM_API_TOKEN` env var) compared in constant-time (`subtle::ConstantTimeEq`).

Rate limited at 30 req/min globally.

---

## POST `/api/v1/system/events`

Upsert an event from the external Google Sheets / GAS pipeline. If an event with the same `external_id` exists, it is updated; otherwise, a new event is created.

### Authentication

```
Authorization: Bearer <SYSTEM_API_TOKEN>
```

### Request Body — `application/json`

```json
{
  "external_id": "gas_event_001",
  "title": "10月クラス会 プレイベント",
  "description_markdown": "# Pre-Event\nJoin us for the pre-event!",
  "status": "published",
  "host_discord_id": "123456789012345678",
  "start_time": "2025-10-04T20:00:00Z",
  "end_time": "2025-10-04T23:00:00Z",
  "location": "https://vrchat.com/home/world/wrld_xxx",
  "tags": ["pre-event", "official"]
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `external_id` | string | Yes | 1–100 chars, unique per source |
| `title` | string | Yes | 1–200 chars |
| `description_markdown` | string \| null | No | max 2000 chars |
| `status` | string | Yes | `draft` / `published` / `cancelled` / `archived` |
| `host_discord_id` | string \| null | No | Discord ID of the host |
| `start_time` | string | Yes | ISO 8601 datetime |
| `end_time` | string \| null | No | ISO 8601 datetime, must be > `start_time` |
| `location` | string \| null | No | VRChat world/instance URL or description, max 200 chars |
| `tags` | string[] | No | each tag 1–50 chars, max 10 tags |

### Response — `200 OK` (Updated)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "external_id": "gas_event_001",
  "title": "10月クラス会 プレイベント",
  "status": "published",
  "action": "updated",
  "updated_at": "2025-06-15T10:00:00Z"
}
```

### Response — `201 Created` (New)

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440001",
  "external_id": "gas_event_002",
  "title": "10月クラス会 本番",
  "status": "published",
  "action": "created",
  "created_at": "2025-06-15T10:00:00Z"
}
```

### Response — `401 Unauthorized` — `ERR-SYNC-001`

Missing or invalid Bearer token.

### Response — `400 Bad Request` — `ERR-SYNC-002`

Validation failure.

### Rust Types

```rust
#[derive(Deserialize)]
pub struct EventUpsertRequest {
    pub external_id: String,
    pub title: String,
    pub description_markdown: Option<String>,
    pub status: EventStatus,
    pub host_discord_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct EventUpsertResponse {
    pub id: Uuid,
    pub external_id: String,
    pub title: String,
    pub status: EventStatus,
    pub action: UpsertAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpsertAction {
    Created,
    Updated,
}
```

### Server-Side Processing

1. Verify Bearer token via constant-time comparison
2. Validate all fields
3. Convert `description_markdown` → `description_html` (same pipeline as profile bio)
4. Begin transaction:
   - `INSERT INTO events ... ON CONFLICT (external_id) DO UPDATE ...`
   - Delete existing tag mappings for this event
   - Upsert tags into `event_tags` table
   - Insert new tag mappings into `event_tag_mappings`
5. Commit transaction
6. Return 201 if inserted, 200 if updated (detected via `xmax` in PostgreSQL)

---

## POST `/api/v1/system/sync/users/leave`

Handle a member leaving the Discord server. Atomically suspends the user, deactivates their sessions, makes their profile non-public, and removes them from all clubs.

### Authentication

```
Authorization: Bearer <SYSTEM_API_TOKEN>
```

### Request Body — `application/json`

```json
{
  "discord_id": "123456789012345678"
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `discord_id` | string | Yes | 1–20 chars, numeric |

### Response — `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "discord_id": "123456789012345678",
  "previous_status": "active",
  "new_status": "suspended",
  "sessions_invalidated": 2,
  "clubs_removed": 3,
  "profile_set_private": true
}
```

### Response — `204 No Content`

User with given `discord_id` not found in the system. This is not an error — the user may never have registered.
```

### Response — `401 Unauthorized` — `ERR-SYNC-001`

### Rust Types

```rust
#[derive(Deserialize)]
pub struct MemberLeaveRequest {
    pub discord_id: String,
}

#[derive(Serialize)]
pub struct MemberLeaveResponse {
    pub user_id: Uuid,
    pub discord_id: String,
    pub previous_status: UserStatus,
    pub new_status: UserStatus,
    pub sessions_invalidated: i64,
    pub clubs_removed: i64,
    pub profile_set_private: bool,
}
```

### Server-Side Processing (Single Transaction)

```sql
BEGIN;

-- 1. Lock and update user status
UPDATE users SET status = 'suspended', updated_at = NOW()
WHERE discord_id = $1 AND status = 'active'
RETURNING id, status;

-- 2. Delete all sessions for this user
DELETE FROM sessions WHERE user_id = $user_id;

-- 3. Set profile to non-public
UPDATE profiles SET is_public = false, updated_at = NOW()
WHERE user_id = $user_id;

-- 4. Remove from all clubs
DELETE FROM club_members WHERE user_id = $user_id;

COMMIT;
```

All four operations happen atomically. If any step fails, the entire transaction rolls back.

Fire background task: send Discord webhook notification to admin channel with leave details.
