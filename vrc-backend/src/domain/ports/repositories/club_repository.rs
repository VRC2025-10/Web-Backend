use uuid::Uuid;

use crate::domain::entities::club::Club;
use crate::errors::infrastructure::InfraError;

pub trait ClubRepository: Send + Sync {
    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<Club>, InfraError>> + Send;

    fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<ClubListRow>, i64), InfraError>> + Send;

    fn get_detail(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<ClubDetailRow>, InfraError>> + Send;

    fn get_members(
        &self,
        club_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<ClubMemberRow>, InfraError>> + Send;

    fn get_clubs_for_user(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<UserClubRow>, InfraError>> + Send;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClubListRow {
    pub id: Uuid,
    pub name: String,
    pub description_html: String,
    pub cover_image_url: Option<String>,
    pub owner_user_id: Uuid,
    pub owner_discord_id: String,
    pub owner_discord_display_name: String,
    pub member_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClubDetailRow {
    pub id: Uuid,
    pub name: String,
    pub description_html: String,
    pub cover_image_url: Option<String>,
    pub owner_user_id: Uuid,
    pub owner_discord_id: String,
    pub owner_discord_display_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClubMemberRow {
    pub user_id: Uuid,
    pub discord_id: String,
    pub discord_display_name: String,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub is_owner: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserClubRow {
    pub id: Uuid,
    pub name: String,
    pub is_owner: bool,
}
