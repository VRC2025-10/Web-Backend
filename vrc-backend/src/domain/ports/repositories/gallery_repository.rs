use uuid::Uuid;

use crate::errors::infrastructure::InfraError;

pub trait GalleryRepository: Send + Sync {
    fn list_approved(
        &self,
        club_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> impl std::future::Future<Output = Result<(Vec<GalleryImageRow>, i64), InfraError>> + Send;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GalleryImageRow {
    pub id: Uuid,
    pub club_id: Uuid,
    pub image_url: String,
    pub caption: Option<String>,
    pub uploaded_by_user_id: Uuid,
    pub uploader_discord_id: String,
    pub uploader_discord_display_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
