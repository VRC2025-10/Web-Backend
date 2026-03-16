# Internal API Endpoints

All endpoints require a valid session cookie. CSRF check (`Origin` header) is enforced on all state-changing methods (POST, PUT, PATCH, DELETE). Rate limited at 120 req/min/user.

Common response headers:

```
Cache-Control: private, no-store
```

---

## GET `/api/v1/internal/auth/me`

Get current authenticated user info. Primary endpoint called by frontend after page load to check session validity.

### Required Role: `member`+

### Response — `200 OK`

```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "discord_id": "123456789012345678",
    "discord_display_name": "Aqua",
    "discord_avatar_hash": "abc123def456",
    "role": "member",
    "status": "active",
    "joined_at": "2024-10-01T00:00:00Z"
  },
  "has_profile": true,
  "profile_summary": {
    "nickname": "あくあ",
    "avatar_url": "https://cdn.discordapp.com/avatars/..."
  }
}
```

### Response — `401 Unauthorized`

```json
{
  "error": "ERR-AUTH-003",
  "message": "セッションが無効です",
  "details": null
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct MeResponse {
    pub user: UserInfo,
    pub has_profile: bool,
    pub profile_summary: Option<ProfileSummary>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub discord_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ProfileSummary {
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}
```

---

## POST `/api/v1/internal/auth/logout`

Destroy the current session. The session row is deleted from `sessions` table and the cookie is cleared.

### Required Role: `member`+

### Request Body: (empty)

### Response — `204 No Content`

Response headers:

```
Set-Cookie: session_id=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0
```

---

## GET `/api/v1/internal/me/profile`

Get the current user's own profile (including non-public data).

### Required Role: `member`+

### Response — `200 OK`

```json
{
  "nickname": "あくあ",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "aqua_x",
  "bio_markdown": "# Hello\nThis is my **bio**.",
  "bio_html": "<h1>Hello</h1>\n<p>This is my <strong>bio</strong>.</p>",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true,
  "updated_at": "2025-06-01T12:00:00Z"
}
```

### Response — `404 Not Found`

User has not created a profile yet (first login, no PUT performed).

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
pub struct OwnProfile {
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub bio_markdown: Option<String>,
    pub bio_html: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub updated_at: DateTime<Utc>,
}
```

---

## PUT `/api/v1/internal/me/profile`

Create or update the current user's profile (UPSERT). The server converts `bio_markdown` → `bio_html` using `pulldown-cmark` + `ammonia` sanitization.

### Required Role: `member`+

### Request Body — `application/json`

```json
{
  "nickname": "あくあ",
  "vrc_id": "usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "x_id": "aqua_x",
  "bio_markdown": "# Hello\nThis is my **bio**.",
  "avatar_url": "https://cdn.discordapp.com/avatars/...",
  "is_public": true
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `nickname` | string \| null | No | 1–50 chars |
| `vrc_id` | string \| null | No | must match `^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$` |
| `x_id` | string \| null | No | must match `^[a-zA-Z0-9_]{1,15}$` |
| `bio_markdown` | string \| null | No | max 2000 chars |
| `avatar_url` | string \| null | No | must be a valid HTTPS URL, max 500 chars |
| `is_public` | bool | Yes | — |

### Response — `200 OK`

Returns the saved profile (same schema as GET `/me/profile`).

### Response — `400 Bad Request`

```json
{
  "error": "ERR-PROF-001",
  "message": "プロフィールのバリデーションに失敗しました",
  "details": {
    "vrc_id": "VRC IDの形式が正しくありません",
    "bio_markdown": "2000文字以内で入力してください"
  }
}
```

### Rust Types

```rust
#[derive(Deserialize)]
pub struct ProfileUpdateRequest {
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub bio_markdown: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
}
```

### Server-Side Processing

1. Validate all fields
2. If `bio_markdown` is set:
   - Parse Markdown → HTML via `pulldown-cmark`
   - Sanitize HTML via `ammonia` (allowlist: `p, h1-h6, strong, em, a[href], ul, ol, li, code, pre, blockquote, br, img[src,alt]`)
   - Check sanitized HTML for suspicious patterns (post-sanitization XSS check) → `ERR-PROF-002` if detected
   - Store both `bio_markdown` and `bio_html`
3. UPSERT into `profiles` table via `INSERT ... ON CONFLICT (user_id) DO UPDATE`

---

## GET `/api/v1/internal/events`

List events with extended information (same as public but includes additional metadata visible to authenticated members).

### Required Role: `member`+

### Query Parameters

Same as public events endpoint.

### Response — `200 OK`

Same schema as public events but may include internal notes or additional fields in the future. Currently identical.

---

## POST `/api/v1/internal/reports`

Submit a moderation report.

### Required Role: `member`+

### Request Body — `application/json`

```json
{
  "target_type": "profile",
  "target_id": "123",
  "reason": "不適切なプロフィール画像が含まれています。コミュニティガイドラインに違反していると思います。"
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `target_type` | string | Yes | `profile` / `club` / `gallery_image` |
| `target_id` | string | Yes | Must resolve to an existing entity |
| `reason` | string | Yes | 10–1000 chars |

### Response — `201 Created`

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "target_type": "profile",
  "target_id": "123",
  "status": "open",
  "created_at": "2025-06-15T10:00:00Z"
}
```

### Response — `404 Not Found` — `ERR-MOD-001`

Target entity does not exist.

### Response — `409 Conflict` — `ERR-MOD-002`

Duplicate report (same reporter + same target).

### Response — `400 Bad Request` — `ERR-MOD-003`

`reason` length outside allowed range.

### Rust Types

```rust
#[derive(Deserialize)]
pub struct CreateReportRequest {
    pub target_type: ReportTargetType,
    pub target_id: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ReportResponse {
    pub id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: String,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
}
```

### Server-Side Processing

1. Validate fields
2. Verify target entity exists (query `profiles`/`clubs`/`gallery_images` depending on `target_type`)
3. Check for duplicate report (same `reporter_id` + `target_type` + `target_id`) → 409 if exists
4. Insert into `reports` table
5. Fire background task: send Discord webhook notification to staff channel

---

## POST `/api/v1/internal/admin/clubs`

Create a new club.

### Required Role: `staff`+

### Request Body — `application/json`

```json
{
  "name": "VRC Photography Club",
  "description_markdown": "# Photography Club\nA place for VRC photography.",
  "owner_user_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `name` | string | Yes | 1–100 chars, unique |
| `description_markdown` | string \| null | No | max 2000 chars |
| `owner_user_id` | Uuid (string) | Yes | must be an active user |

### Response — `201 Created`

```json
{
  "id": "c1d2e3f4-a5b6-7890-cdef-123456789abc",
  "name": "VRC Photography Club",
  "description_html": "<h1>Photography Club</h1>\n<p>A place for VRC photography.</p>",
  "owner": {
    "user_id": "123456789012345678",
    "discord_display_name": "Aqua"
  },
  "created_at": "2025-06-15T10:00:00Z"
}
```

### Server-Side Processing

1. Validate fields
2. Convert `description_markdown` → `description_html` (same pipeline as profile bio)
3. Begin transaction:
   - Insert into `clubs` table
   - Insert owner into `club_members` with `role = 'owner'`
4. Commit transaction

---

## POST `/api/v1/internal/admin/clubs/{id}/gallery`

Upload a gallery image URL for a club (image hosting is external; we only store the URL).

### Required Role: `staff`+

### Request Body — `application/json`

```json
{
  "image_url": "https://example.com/gallery/photo1.webp",
  "caption": "Beautiful sunset in VRChat"
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `image_url` | string | Yes | Valid HTTPS URL, max 500 chars |
| `caption` | string \| null | No | max 200 chars |

### Response — `201 Created`

```json
{
  "id": "d1e2f3a4-b5c6-7890-defg-234567890abc",
  "club_id": "c1d2e3f4-a5b6-7890-cdef-123456789abc",
  "image_url": "https://example.com/gallery/photo1.webp",
  "caption": "Beautiful sunset in VRChat",
  "status": "pending",
  "uploaded_by": {
    "user_id": "123456789012345678",
    "discord_display_name": "Aqua"
  },
  "created_at": "2025-06-15T14:30:00Z"
}
```

### Response — `404 Not Found`

Club does not exist.

---

## PATCH `/api/v1/internal/admin/gallery/{image_id}/status`

Approve or reject a gallery image.

### Required Role: `staff`+

### Request Body — `application/json`

```json
{
  "status": "approved"
}
```

### Field Validation

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `status` | string | Yes | `approved` / `rejected` |

### Response — `200 OK`

```json
{
  "id": "d1e2f3a4-b5c6-7890-defg-234567890abc",
  "status": "approved",
  "reviewed_by": {
    "user_id": "123456789012345678",
    "discord_display_name": "Aqua"
  },
  "reviewed_at": "2025-06-15T15:00:00Z"
}
```

### Response — `404 Not Found` — Gallery image does not exist.

### Response — `400 Bad Request` — `ERR-GALLERY-003` — Invalid status value.

---

## GET `/api/v1/internal/admin/users`

List all users (admin view with full details).

### Required Role: `admin`+

### Query Parameters

| Param | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `page` | u32 | No | 1 | ≥ 1 |
| `per_page` | u32 | No | 20 | 1–100 |
| `status` | string | No | (all) | `active` / `suspended` |
| `role` | string | No | (all) | `member` / `staff` / `admin` / `super_admin` |

### Response — `200 OK`

```json
{
  "items": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "discord_id": "123456789012345678",
      "discord_display_name": "Aqua",
      "discord_avatar_hash": "abc123def456",
      "role": "member",
      "status": "active",
      "joined_at": "2024-10-01T00:00:00Z",
      "last_login_at": "2025-06-15T10:00:00Z"
    }
  ],
  "total_count": 100,
  "total_pages": 5
}
```

### Rust Types

```rust
#[derive(Serialize)]
pub struct AdminUserView {
    pub id: Uuid,
    pub discord_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}
```

---

## PATCH `/api/v1/internal/admin/users/{id}/role`

Change a user's role. Complex authorization rules apply.

### Required Role: `admin`+

### Path Parameters

| Param | Type | Description |
|-------|------|-------------|
| `id` | Uuid | Target user's internal ID |

### Request Body — `application/json`

```json
{
  "role": "staff"
}
```

### Authorization Rules

| Actor Role | Can Set Target To | Condition |
|------------|------------------|-----------|
| `admin` | `member`, `staff` | Target is NOT `super_admin` or `admin` |
| `super_admin` | `member`, `staff`, `admin`, `super_admin` | No restrictions |

Violations:
- `admin` → set `admin` → `ERR-ROLE-001`
- `admin` → set `super_admin` → `ERR-ROLE-002`
- `admin` → modify `super_admin` target → `ERR-ROLE-003`
- `member`/`staff` → any role change → `ERR-ROLE-004`

### Response — `200 OK`

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "role": "staff",
  "updated_at": "2025-06-15T15:30:00Z"
}
```

### Response — `404 Not Found` — `ERR-USER-001` — Target user not found.

### Response — `403 Forbidden` — `ERR-ROLE-*` — Authorization violation.
