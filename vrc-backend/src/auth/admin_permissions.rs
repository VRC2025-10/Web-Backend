use serde::{Deserialize, Serialize};

use crate::domain::entities::user::UserRole;
use crate::errors::api::ApiError;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminPermissionSet {
    pub view_dashboard: bool,
    pub manage_users: bool,
    pub manage_roles: bool,
    pub manage_events: bool,
    pub manage_tags: bool,
    pub manage_reports: bool,
    pub manage_galleries: bool,
    pub manage_clubs: bool,
}

impl AdminPermissionSet {
    pub const fn merge(self, other: Self) -> Self {
        Self {
            view_dashboard: self.view_dashboard || other.view_dashboard,
            manage_users: self.manage_users || other.manage_users,
            manage_roles: self.manage_roles || other.manage_roles,
            manage_events: self.manage_events || other.manage_events,
            manage_tags: self.manage_tags || other.manage_tags,
            manage_reports: self.manage_reports || other.manage_reports,
            manage_galleries: self.manage_galleries || other.manage_galleries,
            manage_clubs: self.manage_clubs || other.manage_clubs,
        }
    }

    pub const fn has_any(self) -> bool {
        self.view_dashboard
            || self.manage_users
            || self.manage_roles
            || self.manage_events
            || self.manage_tags
            || self.manage_reports
            || self.manage_galleries
            || self.manage_clubs
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AdminPermissionRow {
    view_dashboard: bool,
    manage_users: bool,
    manage_roles: bool,
    manage_events: bool,
    manage_tags: bool,
    manage_reports: bool,
    manage_galleries: bool,
    manage_clubs: bool,
}

fn default_admin_permissions(role: UserRole) -> AdminPermissionSet {
    match role {
        UserRole::SuperAdmin | UserRole::Admin => AdminPermissionSet {
            view_dashboard: true,
            manage_users: true,
            manage_roles: true,
            manage_events: true,
            manage_tags: true,
            manage_reports: true,
            manage_galleries: true,
            manage_clubs: true,
        },
        UserRole::Staff => AdminPermissionSet {
            view_dashboard: true,
            manage_users: false,
            manage_roles: false,
            manage_events: false,
            manage_tags: false,
            manage_reports: true,
            manage_galleries: true,
            manage_clubs: true,
        },
        UserRole::Member => AdminPermissionSet::default(),
    }
}

pub async fn load_system_admin_permissions(
    db_pool: &sqlx::PgPool,
    role: UserRole,
) -> Result<AdminPermissionSet, ApiError> {
    if role == UserRole::SuperAdmin {
        return Ok(default_admin_permissions(UserRole::SuperAdmin));
    }

    let row = sqlx::query_as::<_, AdminPermissionRow>(
        r#"
        SELECT
            can_view_dashboard AS view_dashboard,
            can_manage_users AS manage_users,
            can_manage_roles AS manage_roles,
            can_manage_events AS manage_events,
            can_manage_tags AS manage_tags,
            can_manage_reports AS manage_reports,
            can_manage_galleries AS manage_galleries,
            can_manage_clubs AS manage_clubs
        FROM admin_system_role_permissions
        WHERE role = $1
        "#,
    )
    .bind(role)
    .fetch_optional(db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(match row {
        Some(row) => AdminPermissionSet {
            view_dashboard: row.view_dashboard,
            manage_users: row.manage_users,
            manage_roles: row.manage_roles,
            manage_events: row.manage_events,
            manage_tags: row.manage_tags,
            manage_reports: row.manage_reports,
            manage_galleries: row.manage_galleries,
            manage_clubs: row.manage_clubs,
        },
        None => default_admin_permissions(role),
    })
}

pub async fn load_managed_admin_permissions(
    db_pool: &sqlx::PgPool,
    discord_role_ids: &[String],
) -> Result<AdminPermissionSet, ApiError> {
    if discord_role_ids.is_empty() {
        return Ok(AdminPermissionSet::default());
    }

    let row = sqlx::query_as::<_, AdminPermissionRow>(
        r#"
        SELECT
            COALESCE(BOOL_OR(can_view_dashboard), FALSE) AS view_dashboard,
            COALESCE(BOOL_OR(can_manage_users), FALSE) AS manage_users,
            COALESCE(BOOL_OR(can_manage_roles), FALSE) AS manage_roles,
            COALESCE(BOOL_OR(can_manage_events), FALSE) AS manage_events,
            COALESCE(BOOL_OR(can_manage_tags), FALSE) AS manage_tags,
            COALESCE(BOOL_OR(can_manage_reports), FALSE) AS manage_reports,
            COALESCE(BOOL_OR(can_manage_galleries), FALSE) AS manage_galleries,
            COALESCE(BOOL_OR(can_manage_clubs), FALSE) AS manage_clubs
        FROM admin_managed_roles
        WHERE discord_role_id = ANY($1)
        "#,
    )
    .bind(discord_role_ids)
    .fetch_one(db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(AdminPermissionSet {
        view_dashboard: row.view_dashboard,
        manage_users: row.manage_users,
        manage_roles: row.manage_roles,
        manage_events: row.manage_events,
        manage_tags: row.manage_tags,
        manage_reports: row.manage_reports,
        manage_galleries: row.manage_galleries,
        manage_clubs: row.manage_clubs,
    })
}

pub async fn resolve_admin_permissions(
    db_pool: &sqlx::PgPool,
    role: UserRole,
    discord_role_ids: &[String],
) -> Result<AdminPermissionSet, ApiError> {
    let base = load_system_admin_permissions(db_pool, role).await?;
    let managed = load_managed_admin_permissions(db_pool, discord_role_ids).await?;
    Ok(base.merge(managed))
}