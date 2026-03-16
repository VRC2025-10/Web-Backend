use uuid::Uuid;

use crate::errors::infrastructure::InfraError;

pub trait SessionRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        max_age_secs: i64,
    ) -> impl std::future::Future<Output = Result<Uuid, InfraError>> + Send;

    fn delete_by_token_hash(
        &self,
        token_hash: &[u8],
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;

    fn delete_all_for_user(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;

    fn cleanup_expired(
        &self,
    ) -> impl std::future::Future<Output = Result<u64, InfraError>> + Send;
}
