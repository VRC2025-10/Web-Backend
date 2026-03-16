# Functional Requirements

## Authentication & Session Management

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-AUTH-001 | Users authenticate via Discord OAuth2 (Authorization Code + PKCE-optional) | Login redirects to Discord, callback exchanges code for token, guild membership is verified, session cookie is issued |
| FR-AUTH-002 | OAuth2 state parameter prevents CSRF | Random 32-byte `oauth_state` stored in HttpOnly cookie, compared against `state` query param on callback |
| FR-AUTH-003 | Only members of the configured Discord guild can log in | If user is not in guild, callback redirects to frontend with `?error=login_failed` |
| FR-AUTH-004 | Sessions are server-side, identified by UUID in HttpOnly cookie | `session_id` cookie is HttpOnly, SameSite=Lax, Secure (in production), with configurable max-age (default 7 days) |
| FR-AUTH-005 | Session cleanup runs on a configurable interval | Background Tokio task deletes expired sessions every `SESSION_CLEANUP_INTERVAL_SECS` (default 3600) |
| FR-AUTH-006 | `GET /auth/me` returns current user info or 401 | Response includes user ID, Discord username, avatar URL, role, and profile summary (nullable) |
| FR-AUTH-007 | `POST /auth/logout` destroys session and clears cookie | Server deletes session row from DB, response sets `session_id` cookie with `Max-Age=0` |
| FR-AUTH-008 | SuperAdmin bootstrap via `SUPER_ADMIN_DISCORD_ID` env var | On startup, if env var is set, upsert user with `super_admin` role and dummy profile |

## Profile Management

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-PROF-001 | Users can view their own profile (even if non-public) via `GET /me/profile` | Returns full profile including `is_public`, `bio_markdown`, `bio_html` |
| FR-PROF-002 | Users can create/update their profile via `PUT /me/profile` | Upsert semantics; validates `vrc_id` (regex `^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`), `x_id` (regex `^[a-zA-Z0-9_]{1,15}$`), `bio_markdown` (≤2000 chars, XSS-scanned), `is_public` (bool) |
| FR-PROF-003 | Markdown bio is rendered to sanitized HTML server-side | `bio_html` is produced by parsing Markdown (pulldown-cmark) and sanitizing with ammonia (allowlist: `h1-h6`, `p`, `a`, `strong`, `em`, `ul`, `ol`, `li`, `code`, `pre`, `blockquote`, `img` with src allowlist) |
| FR-PROF-004 | Public profiles are visible via `GET /public/members` and `GET /public/members/{id}` | Only profiles with `is_public=true` and user status `active` are returned |
| FR-PROF-005 | Member list supports offset pagination | `page` (default 1) and `per_page` (default 20, max 100); response includes `total_count` and `total_pages` |
| FR-PROF-006 | `bio_summary` in list view is plain-text truncation of bio | Strip Markdown, take first 120 chars, append `...` if truncated |

## Event Management

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-EVT-001 | Events are synced from external sources via `POST /system/events` (upsert) | `external_source_id` is the dedup key; new → `201`, existing → `200`; idempotent |
| FR-EVT-002 | New event creation triggers Discord webhook notification | Webhook POST with embed containing title, host, time, location |
| FR-EVT-003 | Events support status lifecycle: `draft` → `published` → `cancelled` / `archived` | Status is an enum; public API can filter by status |
| FR-EVT-004 | Events have tags (many-to-many) | Tags are pre-defined in `event_tags` table; sync API matches by tag name |
| FR-EVT-005 | Event list supports pagination and status filter | Same pagination as members; `?status=published` filter |
| FR-EVT-006 | Internal event list includes `extended_info` field | Extra metadata for logged-in user (future: participation status) |
| FR-EVT-007 | Automatic archival of past events on configurable interval | Background task sets `archived` status on events where `end_time < now()` |
| FR-EVT-008 | `host_discord_id` resolves to a registered user if possible | If a user with matching `discord_id` exists, set `host_user_id` FK; otherwise leave null and use `host_name` |

## Club & Gallery Management

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-CLUB-001 | Staff+ can create clubs via `POST /admin/clubs` | Request body: `name`, `description`, `cover_image_url` (optional) |
| FR-CLUB-002 | Club list and detail are publicly accessible | `GET /public/clubs` (array), `GET /public/clubs/{id}` (single) |
| FR-CLUB-003 | Staff+ can upload gallery images to a club | `POST /admin/clubs/{id}/gallery` with `image_url`; initial status `pending` |
| FR-CLUB-004 | Staff+ can approve/reject gallery images | `PATCH /admin/gallery/{image_id}/status` with `pending`/`approved`/`rejected` |
| FR-CLUB-005 | Public gallery endpoint returns only `approved` images | `GET /public/clubs/{id}/gallery` with pagination |

## Moderation

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-MOD-001 | Members can report profiles, events, clubs, or gallery images | `POST /reports` with `target_type` (`profile`/`event`/`club`/`gallery_image`), `target_id`, `reason` (10-1000 chars) |
| FR-MOD-002 | Duplicate reports from same user are rejected | 409 Conflict with `ERR-MOD-002` |
| FR-MOD-003 | New reports trigger Discord webhook notification | Webhook POST with report details to moderation channel |
| FR-MOD-004 | Target existence is validated before accepting report | 404 if target does not exist |

## User Administration

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-ADMIN-001 | Admin+ can list all users with role/status filters | `GET /admin/users` with `?role=...&status=...` and pagination |
| FR-ADMIN-002 | Admin+ can change user roles with hierarchical constraints | See role change security matrix (Architecture §2) |
| FR-ADMIN-003 | Only `super_admin` can grant/revoke `admin` or `super_admin` roles | Enforced server-side; 403 with specific error codes |

## System Integration

| ID | Requirement | Acceptance Criteria |
|----|------------|-------------------|
| FR-SYS-001 | System API authenticates via Bearer token (SHA-256 + constant-time compare) | Token ≥64 chars; hashed before comparison; timing-safe |
| FR-SYS-002 | Member leave sync suspends user atomically | `POST /system/sync/users/leave`: status→suspended, sessions deleted, profile→private; single transaction |
| FR-SYS-003 | Unknown Discord IDs on leave sync return 204 (no-op) | Not an error — user may never have registered |
