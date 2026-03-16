use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub reporter_user_id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub reason: String,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "report_target_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportTargetType {
    Profile,
    Event,
    Club,
    GalleryImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "report_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pending,
    Reviewed,
    Dismissed,
}
