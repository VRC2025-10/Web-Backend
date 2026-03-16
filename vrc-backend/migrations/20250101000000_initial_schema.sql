-- Initial schema for VRC Class Reunion Backend
-- Creates all ENUMs, tables, and indexes for Phase 0-2

-- ==========================================
-- ENUMs
-- ==========================================

CREATE TYPE user_role AS ENUM ('member', 'staff', 'admin', 'super_admin');
CREATE TYPE user_status AS ENUM ('active', 'suspended');
CREATE TYPE event_status AS ENUM ('draft', 'published', 'cancelled', 'archived');
CREATE TYPE report_status AS ENUM ('pending', 'reviewed', 'dismissed');
CREATE TYPE report_target_type AS ENUM ('profile', 'event', 'club', 'gallery_image');
CREATE TYPE gallery_image_status AS ENUM ('pending', 'approved', 'rejected');

-- ==========================================
-- users
-- ==========================================

CREATE TABLE users (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_id       TEXT        NOT NULL UNIQUE,
    discord_username TEXT        NOT NULL,
    discord_display_name TEXT    NOT NULL DEFAULT '',
    discord_avatar_hash TEXT,
    avatar_url       TEXT,
    role             user_role   NOT NULL DEFAULT 'member',
    status           user_status NOT NULL DEFAULT 'active',
    joined_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_users_discord_id ON users (discord_id);
CREATE INDEX idx_users_role ON users (role);
CREATE INDEX idx_users_status ON users (status);

-- ==========================================
-- profiles
-- ==========================================

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

CREATE INDEX idx_profiles_is_public ON profiles (is_public) WHERE is_public = true;

-- ==========================================
-- sessions
-- ==========================================

CREATE TABLE sessions (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA       NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions (token_hash);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

-- ==========================================
-- event_tags
-- ==========================================

CREATE TABLE event_tags (
    id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name  TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#6B7280'
);

CREATE UNIQUE INDEX idx_event_tags_name ON event_tags (name);

-- ==========================================
-- events
-- ==========================================

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

CREATE UNIQUE INDEX idx_events_external_source_id ON events (external_source_id)
    WHERE external_source_id IS NOT NULL;
CREATE INDEX idx_events_status ON events (event_status);
CREATE INDEX idx_events_start_time ON events (start_time DESC);
CREATE INDEX idx_events_archival ON events (event_status, end_time)
    WHERE event_status NOT IN ('cancelled', 'archived');

-- ==========================================
-- event_tag_mappings
-- ==========================================

CREATE TABLE event_tag_mappings (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id   UUID NOT NULL REFERENCES event_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, tag_id)
);

CREATE INDEX idx_event_tag_mappings_tag_id ON event_tag_mappings (tag_id);

-- ==========================================
-- reports
-- ==========================================

CREATE TABLE reports (
    id               UUID               PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_user_id UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type      report_target_type  NOT NULL,
    target_id        UUID               NOT NULL,
    reason           TEXT               NOT NULL,
    status           report_status      NOT NULL DEFAULT 'pending',
    created_at       TIMESTAMPTZ        NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_reports_unique_per_user ON reports (reporter_user_id, target_type, target_id);
CREATE INDEX idx_reports_target ON reports (target_type, target_id);

-- ==========================================
-- clubs
-- ==========================================

CREATE TABLE clubs (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name                 TEXT        NOT NULL,
    description_markdown TEXT        NOT NULL DEFAULT '',
    description_html     TEXT        NOT NULL DEFAULT '',
    cover_image_url      TEXT,
    owner_user_id        UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ==========================================
-- club_members
-- ==========================================

CREATE TABLE club_members (
    club_id   UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role      TEXT NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (club_id, user_id)
);

CREATE INDEX idx_club_members_user_id ON club_members (user_id);

-- ==========================================
-- gallery_images
-- ==========================================

CREATE TABLE gallery_images (
    id                 UUID                 PRIMARY KEY DEFAULT gen_random_uuid(),
    club_id            UUID                 NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    uploaded_by_user_id UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    image_url          TEXT                 NOT NULL,
    caption            TEXT,
    status             gallery_image_status NOT NULL DEFAULT 'pending',
    created_at         TIMESTAMPTZ          NOT NULL DEFAULT now()
);

CREATE INDEX idx_gallery_images_club_approved ON gallery_images (club_id, created_at DESC)
    WHERE status = 'approved';
CREATE INDEX idx_gallery_images_club_all ON gallery_images (club_id, created_at DESC);
