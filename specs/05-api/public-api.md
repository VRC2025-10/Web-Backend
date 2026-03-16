# Public API Endpoints

All endpoints are unauthenticated, cached at edge, and rate-limited at 60 req/min/IP.

Common response headers:

```
Cache-Control: public, max-age=30, stale-while-revalidate=60
```

---

## GET `/api/v1/public/members`

List public member profiles (users with `is_public = true` and `status != 'suspended'`).

### Query Parameters

| Param | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `page` | u32 | No | 1 | ≥ 1 |
| `per_page` | u32 | No | 20 | 1–100 |

### Response — `200 OK`

```json
{
  "items": [
    {
      "user_id": "123456789012345678",
      "discord_display_name": "Aqua",
      "discord_avatar_hash": "abc123def456",
      "joined_at": "2024-10-01T00:00:00Z",
      "profile": {
        "nickname": "あくあ",
        "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        "x_id": "aqua_x",
        "bio_html": "<p>Hello world</p>",
        "avatar_url": "https://cdn.discordapp.com/avatars/..."
      }
    }
  ],
  "total_count": 42,
  "total_pages": 3
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct PublicMemberSummary {
    pub user_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub profile: Option<PublicProfileSummary>,
}

#[derive(Serialize)]
pub struct PublicProfileSummary {
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub bio_html: Option<String>,
    pub avatar_url: Option<String>,
}
```

### SQL (simplified)

```sql
SELECT u.discord_id, u.discord_display_name, u.discord_avatar_hash, u.joined_at,
       p.nickname, p.vrc_id, p.x_id, p.bio_html, p.avatar_url
FROM users u
LEFT JOIN profiles p ON p.user_id = u.id AND p.is_public = true
WHERE u.status = 'active'
ORDER BY u.joined_at DESC
LIMIT $1 OFFSET $2;
```

---

## GET `/api/v1/public/members/{user_id}`

Get a single member's public profile. `{user_id}` is `discord_id` (string).

### Path Parameters

| Param | Type | Description |
|-------|------|-------------|
| `user_id` | String | Discord user ID |

### Response — `200 OK`

```json
{
  "user_id": "123456789012345678",
  "discord_display_name": "Aqua",
  "discord_avatar_hash": "abc123def456",
  "joined_at": "2024-10-01T00:00:00Z",
  "profile": {
    "nickname": "あくあ",
    "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    "x_id": "aqua_x",
    "bio_html": "<p>Hello world</p>",
    "avatar_url": "https://cdn.discordapp.com/avatars/...",
    "updated_at": "2025-06-01T12:00:00Z"
  },
  "clubs": [
    {
      "id": 1,
      "name": "VRC Photography Club",
      "role": "member"
    }
  ]
}
```

### Response — `404 Not Found`

User does not exist, is suspended, or has no public profile.

```json
{
  "error": "ERR-PROF-004",
  "message": "プロフィールが見つかりません",
  "details": null
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct PublicMemberDetail {
    pub user_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub profile: Option<PublicProfileDetail>,
    pub clubs: Vec<ClubMembership>,
}

#[derive(Serialize)]
pub struct PublicProfileDetail {
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub bio_html: Option<String>,
    pub avatar_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ClubMembership {
    pub id: Uuid,
    pub name: String,
    pub role: String, // "owner" | "member"
}
```

---

## GET `/api/v1/public/events`

List events, optionally filtered by status.

### Query Parameters

| Param | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `page` | u32 | No | 1 | ≥ 1 |
| `per_page` | u32 | No | 20 | 1–100 |
| `status` | string | No | (all) | `draft` / `published` / `cancelled` / `archived` |

### Response — `200 OK`

Note: The `display_status` field is computed from `event_status` (DB) + current time: `published` events with `start_time > now()` = `upcoming`, `start_time <= now() <= end_time` = `ongoing`, `end_time < now()` = `ended`. Non-published statuses map directly (`draft`, `cancelled`, `archived`).

```json
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "10月クラス会 本番",
      "description_html": "<p>Main event description</p>",
      "status": "published",
      "display_status": "upcoming",
      "start_time": "2025-10-11T20:00:00Z",
      "end_time": "2025-10-11T23:00:00Z",
      "location": "https://vrchat.com/home/world/wrld_xxx",
      "tags": ["main", "official"],
      "created_at": "2025-06-01T00:00:00Z"
    }
  ],
  "total_count": 5,
  "total_pages": 1
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct EventSummary {
    pub id: Uuid,
    pub title: String,
    pub description_html: Option<String>,
    pub status: EventStatus,
    pub display_status: DisplayStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Database event_status enum
#[derive(Serialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
pub enum EventStatus {
    Draft,
    Published,
    Cancelled,
    Archived,
}

/// Computed display status for API consumers
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayStatus {
    Draft,
    Upcoming,
    Ongoing,
    Ended,
    Cancelled,
    Archived,
}
```

---

## GET `/api/v1/public/events/{event_id}`

Get single event detail.

### Path Parameters

| Param | Type | Description |
|-------|------|-------------|
| `event_id` | Uuid | Event ID |

### Response — `200 OK`

Same schema as list item but with full `description_html`.

### Response — `404 Not Found`

```json
{
  "error": "ERR-NOT-FOUND",
  "message": "イベントが見つかりません",
  "details": null
}
```

---

## GET `/api/v1/public/clubs`

List all clubs.

### Query Parameters

| Param | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `page` | u32 | No | 1 | ≥ 1 |
| `per_page` | u32 | No | 20 | 1–100 |

### Response — `200 OK`

```json
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "VRC Photography Club",
      "description_html": "<p>A club for photography enthusiasts</p>",
      "owner": {
        "user_id": "123456789012345678",
        "discord_display_name": "Aqua"
      },
      "member_count": 12,
      "created_at": "2025-06-01T00:00:00Z"
    }
  ],
  "total_count": 3,
  "total_pages": 1
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct ClubSummary {
    pub id: Uuid,
    pub name: String,
    pub description_html: Option<String>,
    pub owner: UserBrief,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserBrief {
    pub user_id: String,
    pub discord_display_name: String,
}
```

---

## GET `/api/v1/public/clubs/{id}`

Get single club detail with members.

### Response — `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "VRC Photography Club",
  "description_html": "<p>A club for photography enthusiasts</p>",
  "owner": {
    "user_id": "123456789012345678",
    "discord_display_name": "Aqua"
  },
  "members": [
    {
      "user_id": "123456789012345678",
      "discord_display_name": "Aqua",
      "role": "owner",
      "joined_at": "2025-06-01T00:00:00Z"
    }
  ],
  "created_at": "2025-06-01T00:00:00Z"
}
```

### Response — `404 Not Found`

```json
{
  "error": "ERR-NOT-FOUND",
  "message": "部活が見つかりません",
  "details": null
}
```

---

## GET `/api/v1/public/clubs/{id}/gallery`

List approved gallery images for a club.

### Query Parameters

| Param | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `page` | u32 | No | 1 | ≥ 1 |
| `per_page` | u32 | No | 20 | 1–100 |

### Response — `200 OK`

```json
{
  "items": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "image_url": "https://example.com/gallery/photo1.webp",
      "caption": "Beautiful sunset in VRChat",
      "uploaded_by": {
        "user_id": "123456789012345678",
        "discord_display_name": "Aqua"
      },
      "created_at": "2025-06-15T14:30:00Z"
    }
  ],
  "total_count": 8,
  "total_pages": 1
}
```

Only images with `status = 'approved'` are returned.

### Rust Types

```rust
#[derive(Serialize)]
pub struct GalleryImagePublic {
    pub id: Uuid,
    pub image_url: String,
    pub caption: Option<String>,
    pub uploaded_by: UserBrief,
    pub created_at: DateTime<Utc>,
}
```
