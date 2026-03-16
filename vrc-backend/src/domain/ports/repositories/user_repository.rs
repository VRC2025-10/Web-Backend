use uuid::Uuid;

use crate::domain::entities::user::{User, UserRole, UserStatus};
use crate::domain::value_objects::pagination::PageRequest;
use crate::errors::infrastructure::InfraError;

pub trait UserRepository: Send + Sync {
    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<User>, InfraError>> + Send;

    fn find_by_discord_id(
        &self,
        discord_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<User>, InfraError>> + Send;

    fn upsert_from_discord(
        &self,
        discord_id: &str,
        discord_username: &str,
        avatar_url: Option<&str>,
    ) -> impl std::future::Future<Output = Result<User, InfraError>> + Send;

    fn update_role(
        &self,
        user_id: Uuid,
        new_role: UserRole,
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;

    fn update_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;

    fn list_all(
        &self,
        page: &PageRequest,
        status_filter: Option<UserStatus>,
        role_filter: Option<UserRole>,
    ) -> impl std::future::Future<Output = Result<(Vec<User>, i64), InfraError>> + Send;
}
