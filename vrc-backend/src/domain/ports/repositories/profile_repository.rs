use uuid::Uuid;

use crate::domain::entities::profile::Profile;
use crate::errors::infrastructure::InfraError;

pub trait ProfileRepository: Send + Sync {
    fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<Profile>, InfraError>> + Send;

    fn upsert(
        &self,
        profile: &Profile,
    ) -> impl std::future::Future<Output = Result<Profile, InfraError>> + Send;

    fn list_public(
        &self,
        limit: i64,
        offset: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<PublicMemberRow>, i64), InfraError>> + Send;

    fn find_public_by_discord_id(
        &self,
        discord_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<PublicMemberDetailRow>, InfraError>> + Send;

    fn set_private(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), InfraError>> + Send;
}

/// Joined row for public member list
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicMemberRow {
    pub user_id: Uuid,
    pub discord_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub nickname: Option<String>,
    pub profile_avatar_url: Option<String>,
    pub bio_html: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
}

/// Joined row for public member detail
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicMemberDetailRow {
    pub user_id: Uuid,
    pub discord_id: String,
    pub discord_display_name: String,
    pub discord_avatar_hash: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub nickname: Option<String>,
    pub profile_avatar_url: Option<String>,
    pub bio_html: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub profile_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
