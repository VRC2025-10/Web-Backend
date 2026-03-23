-- Schedule support for internal planning board.

ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS discord_access_token TEXT,
    ADD COLUMN IF NOT EXISTS discord_refresh_token TEXT,
    ADD COLUMN IF NOT EXISTS discord_token_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS discord_role_ids TEXT[] NOT NULL DEFAULT '{}';

CREATE TABLE IF NOT EXISTS schedule_managed_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_role_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    can_manage_roles BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_events BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_templates BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    can_view_restricted_events BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS schedule_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    start_at TIMESTAMPTZ NOT NULL,
    end_at TIMESTAMPTZ NOT NULL,
    visibility_mode TEXT NOT NULL DEFAULT 'public',
    auto_notify_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (visibility_mode IN ('public', 'restricted')),
    CHECK (end_at > start_at)
);

CREATE TABLE IF NOT EXISTS schedule_event_visible_roles (
    event_id UUID NOT NULL REFERENCES schedule_events(id) ON DELETE CASCADE,
    discord_role_id TEXT NOT NULL,
    PRIMARY KEY (event_id, discord_role_id)
);

CREATE TABLE IF NOT EXISTS schedule_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_schedule_templates_default
    ON schedule_templates (is_default)
    WHERE is_default = TRUE;

CREATE TABLE IF NOT EXISTS schedule_notification_settings (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    webhook_url TEXT NOT NULL,
    updated_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (id = TRUE)
);

CREATE TABLE IF NOT EXISTS schedule_notification_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    schedule_type TEXT NOT NULL,
    offset_minutes INTEGER,
    time_of_day_minutes INTEGER,
    window_start_minutes INTEGER,
    window_end_minutes INTEGER,
    body_template TEXT NOT NULL,
    list_item_template TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (schedule_type IN ('before_event', 'daily_at'))
);

CREATE TABLE IF NOT EXISTS schedule_notification_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID NOT NULL REFERENCES schedule_notification_rules(id) ON DELETE CASCADE,
    event_id UUID REFERENCES schedule_events(id) ON DELETE CASCADE,
    delivery_key TEXT NOT NULL UNIQUE,
    scheduled_for TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT NOT NULL DEFAULT '',
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('pending', 'sent', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_schedule_events_range
    ON schedule_events (start_at, end_at);

CREATE INDEX IF NOT EXISTS idx_schedule_events_creator
    ON schedule_events (created_by_user_id, start_at DESC);

CREATE INDEX IF NOT EXISTS idx_schedule_managed_roles_discord_role_id
    ON schedule_managed_roles (discord_role_id);

CREATE INDEX IF NOT EXISTS idx_schedule_notification_rules_enabled
    ON schedule_notification_rules (enabled, schedule_type);

CREATE INDEX IF NOT EXISTS idx_schedule_notification_deliveries_due
    ON schedule_notification_deliveries (status, scheduled_for);