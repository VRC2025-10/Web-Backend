use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub user_id: Uuid,
    pub nickname: Option<String>,
    pub vrc_id: Option<String>,
    pub x_id: Option<String>,
    pub bio_markdown: String,
    pub bio_html: String,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub updated_at: DateTime<Utc>,
}
