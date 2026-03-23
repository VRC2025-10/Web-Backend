CREATE TABLE IF NOT EXISTS admin_managed_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    discord_role_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    can_view_dashboard BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_users BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_roles BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_events BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_tags BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_reports BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_galleries BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_clubs BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_admin_managed_roles_discord_role_id
    ON admin_managed_roles (discord_role_id);