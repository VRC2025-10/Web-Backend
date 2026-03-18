use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub discord_id: String,
    pub discord_username: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn role_level(&self) -> u8 {
        self.role.level()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Member,
    Staff,
    Admin,
    SuperAdmin,
}

impl UserRole {
    pub fn level(self) -> u8 {
        match self {
            Self::Member => 0,
            Self::Staff => 1,
            Self::Admin => 2,
            Self::SuperAdmin => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Staff => "staff",
            Self::Admin => "admin",
            Self::SuperAdmin => "super_admin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_levels_are_ordered() {
        assert!(UserRole::Member.level() < UserRole::Staff.level());
        assert!(UserRole::Staff.level() < UserRole::Admin.level());
        assert!(UserRole::Admin.level() < UserRole::SuperAdmin.level());
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(UserRole::Member.as_str(), "member");
        assert_eq!(UserRole::Staff.as_str(), "staff");
        assert_eq!(UserRole::Admin.as_str(), "admin");
        assert_eq!(UserRole::SuperAdmin.as_str(), "super_admin");
    }

    #[test]
    fn test_role_level_values() {
        assert_eq!(UserRole::Member.level(), 0);
        assert_eq!(UserRole::Staff.level(), 1);
        assert_eq!(UserRole::Admin.level(), 2);
        assert_eq!(UserRole::SuperAdmin.level(), 3);
    }
}
