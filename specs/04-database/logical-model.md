# Logical Model — Complete Schema

## PostgreSQL ENUMs

```sql
-- Migration 01: ENUMs
CREATE TYPE user_role AS ENUM ('member', 'staff', 'admin', 'super_admin');
CREATE TYPE user_status AS ENUM ('active', 'suspended');
CREATE TYPE event_status AS ENUM ('draft', 'published', 'cancelled', 'archived');
CREATE TYPE report_status AS ENUM ('pending', 'reviewed', 'dismissed');
CREATE TYPE report_target_type AS ENUM ('profile', 'event', 'club', 'gallery_image');
CREATE TYPE gallery_image_status AS ENUM ('pending', 'approved', 'rejected');
```

## Tables

### users

```sql
-- Migration 02: users
CREATE TABLE users (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_id      TEXT        NOT NULL UNIQUE,
    discord_username TEXT       NOT NULL,
    avatar_url      TEXT,
    role            user_role   NOT NULL DEFAULT 'member',
    status          user_status NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes
CREATE UNIQUE INDEX idx_users_discord_id ON users (discord_id);
-- ↑ Also serves as the lookup key for OAuth2 login + member leave sync

CREATE INDEX idx_users_role ON users (role);
-- ↑ Used by admin user list filter: WHERE role = $1

CREATE INDEX idx_users_status ON users (status);
-- ↑ Used by admin user list filter: WHERE status = $1
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| id | UUID | No | `gen_random_uuid()` | PK | Internal identifier |
| discord_id | TEXT | No | — | UNIQUE | Discord snowflake ID (17-20 digit string) |
| discord_username | TEXT | No | — | — | Updated on each login from Discord API |
| avatar_url | TEXT | Yes | — | — | Discord CDN URL, null if no custom avatar |
| role | user_role | No | `'member'` | — | Hierarchical: member < staff < admin < super_admin |
| status | user_status | No | `'active'` | — | `suspended` = left Discord server |
| created_at | TIMESTAMPTZ | No | `now()` | — | First login time |
| updated_at | TIMESTAMPTZ | No | `now()` | — | Last profile/role change |

### profiles

```sql
-- Migration 03: profiles
CREATE TABLE profiles (
    user_id      UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    nickname     TEXT,
    vrc_id       TEXT,
    x_id         TEXT,
    bio_markdown TEXT        NOT NULL DEFAULT '',
    bio_html     TEXT        NOT NULL DEFAULT '',
    avatar_url   TEXT,
    is_public    BOOLEAN     NOT NULL DEFAULT true,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX idx_profiles_is_public ON profiles (is_public) WHERE is_public = true;
-- ↑ Partial index: only index public profiles for the public member list query
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| user_id | UUID | No | — | PK, FK → users(id) CASCADE | 1:1 with users |
| nickname | TEXT | Yes | — | — | Display name (1–50 chars, app-enforced) |
| vrc_id | TEXT | Yes | — | — | VRChat user ID (e.g., `usr_abc123`) |
| x_id | TEXT | Yes | — | — | X/Twitter handle (e.g., `aqua_x`, without @) |
| bio_markdown | TEXT | No | `''` | — | User-written Markdown (max 2000 chars, app-enforced) |
| bio_html | TEXT | No | `''` | — | Server-rendered sanitized HTML |
| avatar_url | TEXT | Yes | — | — | Custom avatar URL (HTTPS only, max 500 chars) |
| is_public | BOOLEAN | No | `true` | — | Controls visibility in Public API |
| updated_at | TIMESTAMPTZ | No | `now()` | — | Last edit time |

### sessions

```sql
-- Migration 04: sessions
CREATE TABLE sessions (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA       NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes
CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions (token_hash);
-- ↑ Used for session lookup: SHA-256(cookie_value) → session row

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
-- ↑ Used for "delete all sessions for user" (member leave)

CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
-- ↑ Used by session cleanup background task: DELETE WHERE expires_at < now()
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| id | UUID | No | `gen_random_uuid()` | PK | Internal identifier |
| user_id | UUID | No | — | FK → users(id) CASCADE | Session owner |
| token_hash | BYTEA | No | — | UNIQUE | SHA-256 hash of the raw session token (32 bytes) |
| expires_at | TIMESTAMPTZ | No | — | — | `created_at + SESSION_MAX_AGE_SECS` |
| created_at | TIMESTAMPTZ | No | `now()` | — | Session creation time |

### event_tags

```sql
-- Migration 05: event_tags
CREATE TABLE event_tags (
    id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name  TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#6B7280'
);

CREATE UNIQUE INDEX idx_event_tags_name ON event_tags (name);
-- ↑ Used by event sync to resolve tag names to IDs
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| id | UUID | No | `gen_random_uuid()` | PK | — |
| name | TEXT | No | — | UNIQUE | e.g., "Social", "Beginner", "Meetup" |
| color | TEXT | No | `'#6B7280'` | — | Hex color code for frontend display |

### events

```sql
-- Migration 06: events
CREATE TABLE events (
    id                    UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    external_source_id    TEXT          UNIQUE,
    title                 TEXT          NOT NULL,
    description_markdown  TEXT          NOT NULL DEFAULT '',
    description_html      TEXT          NOT NULL DEFAULT '',
    host_user_id          UUID          REFERENCES users(id) ON DELETE SET NULL,
    host_name             TEXT          NOT NULL,
    event_status          event_status  NOT NULL DEFAULT 'draft',
    start_time            TIMESTAMPTZ   NOT NULL,
    end_time              TIMESTAMPTZ,
    location              TEXT,
    created_at            TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ   NOT NULL DEFAULT now()
);

-- Indexes
CREATE UNIQUE INDEX idx_events_external_source_id ON events (external_source_id)
    WHERE external_source_id IS NOT NULL;
-- ↑ Partial unique index: allows NULL (events created without external source)

CREATE INDEX idx_events_status ON events (event_status);
-- ↑ Used by public event list filter: WHERE event_status = $1

CREATE INDEX idx_events_start_time ON events (start_time DESC);
-- ↑ Used for ordering events by date (most recent first)

CREATE INDEX idx_events_archival ON events (event_status, end_time)
    WHERE event_status NOT IN ('cancelled', 'archived');
-- ↑ Partial index for archival background task efficiency
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| id | UUID | No | `gen_random_uuid()` | PK | — |
| external_source_id | TEXT | Yes | — | UNIQUE (partial) | Dedup key from GAS sync |
| title | TEXT | No | — | — | 1-100 chars (app-enforced) |
| description_markdown | TEXT | No | `''` | — | Max 2000 chars (app-enforced) |
| description_html | TEXT | No | `''` | — | Server-rendered sanitized HTML |
| host_user_id | UUID | Yes | — | FK → users(id) SET NULL | Resolved from `host_discord_id` if user exists |
| host_name | TEXT | No | — | — | Display name fallback (1-50 chars) |
| event_status | event_status | No | `'draft'` | — | Lifecycle state |
| start_time | TIMESTAMPTZ | No | — | — | Event start (UTC) |
| end_time | TIMESTAMPTZ | Yes | — | — | Must be after start_time (app-enforced) |
| location | TEXT | Yes | — | — | VRChat world/instance (max 200 chars) |
| created_at | TIMESTAMPTZ | No | `now()` | — | — |
| updated_at | TIMESTAMPTZ | No | `now()` | — | — |

### event_tag_mappings

```sql
-- Migration 07: event_tag_mappings
CREATE TABLE event_tag_mappings (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id   UUID NOT NULL REFERENCES event_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, tag_id)
);

CREATE INDEX idx_event_tag_mappings_tag_id ON event_tag_mappings (tag_id);
-- ↑ Reverse lookup: "which events have this tag?"
```

### reports

```sql
-- Migration 08: reports
CREATE TABLE reports (
    id               UUID               PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_user_id UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type      report_target_type  NOT NULL,
    target_id        UUID               NOT NULL,
    reason           TEXT               NOT NULL,
    status           report_status      NOT NULL DEFAULT 'pending',
    created_at       TIMESTAMPTZ        NOT NULL DEFAULT now()
);

-- Indexes
CREATE UNIQUE INDEX idx_reports_unique_per_user ON reports (reporter_user_id, target_type, target_id);
-- ↑ Prevents duplicate reports from the same user for the same target

CREATE INDEX idx_reports_target ON reports (target_type, target_id);
-- ↑ Used to look up all reports for a given target
```

| Column | Type | Nullable | Default | Constraints | Notes |
|--------|------|----------|---------|-------------|-------|
| id | UUID | No | `gen_random_uuid()` | PK | — |
| reporter_user_id | UUID | No | — | FK → users(id) CASCADE | Who reported |
| target_type | report_target_type | No | — | — | `'profile'` or `'event'` |
| target_id | UUID | No | — | — | UUID of the reported entity (no FK — polymorphic) |
| reason | TEXT | No | — | — | 10-1000 chars (app-enforced) |
| status | report_status | No | `'pending'` | — | Moderation workflow state |
| created_at | TIMESTAMPTZ | No | `now()` | — | — |

### clubs

```sql
-- Migration 10: clubs
CREATE TABLE clubs (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT        NOT NULL,
    description     TEXT        NOT NULL DEFAULT '',
    cover_image_url TEXT,
    created_by      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### club_members

```sql
-- Migration 11: club_members
CREATE TABLE club_members (
    club_id   UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (club_id, user_id)
);

CREATE INDEX idx_club_members_user_id ON club_members (user_id);
-- ↑ "Which clubs does this user belong to?"
```

### gallery_images

```sql
-- Migration 12: gallery_images
CREATE TABLE gallery_images (
    id          UUID                 PRIMARY KEY DEFAULT gen_random_uuid(),
    club_id     UUID                 NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    uploaded_by UUID                 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    image_url   TEXT                 NOT NULL,
    caption     TEXT,
    status      gallery_image_status NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ          NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX idx_gallery_images_club_approved ON gallery_images (club_id, created_at DESC)
    WHERE status = 'approved';
-- ↑ Partial index: only approved images for public gallery query (most recent first)

CREATE INDEX idx_gallery_images_club_all ON gallery_images (club_id, created_at DESC);
-- ↑ Admin view: all images for a club regardless of status
```
