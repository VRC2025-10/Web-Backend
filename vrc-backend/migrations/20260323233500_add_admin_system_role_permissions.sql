CREATE TABLE IF NOT EXISTS admin_system_role_permissions (
    role user_role PRIMARY KEY,
    can_view_dashboard BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_users BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_roles BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_events BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_tags BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_reports BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_galleries BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_clubs BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO admin_system_role_permissions (
    role,
    can_view_dashboard,
    can_manage_users,
    can_manage_roles,
    can_manage_events,
    can_manage_tags,
    can_manage_reports,
    can_manage_galleries,
    can_manage_clubs
)
VALUES
    ('member', FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE),
    ('staff', TRUE, FALSE, FALSE, FALSE, FALSE, TRUE, TRUE, TRUE),
    ('admin', TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE, TRUE)
ON CONFLICT (role) DO NOTHING;