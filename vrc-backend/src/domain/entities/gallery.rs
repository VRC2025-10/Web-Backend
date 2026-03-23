use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryImage {
    pub id: Uuid,
    pub target_type: GalleryTargetType,
    pub club_id: Option<Uuid>,
    pub uploaded_by_user_id: Uuid,
    pub image_url: String,
    pub caption: Option<String>,
    pub status: GalleryImageStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "gallery_target_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GalleryTargetType {
    Community,
    Club,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "gallery_image_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GalleryImageStatus {
    Pending,
    Approved,
    Rejected,
}
