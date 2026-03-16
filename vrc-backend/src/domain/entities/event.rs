use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub external_source_id: Option<String>,
    pub title: String,
    pub description_markdown: String,
    pub description_html: String,
    pub host_user_id: Option<Uuid>,
    pub host_name: String,
    pub event_status: EventStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Draft,
    Published,
    Cancelled,
    Archived,
}

/// Computed display status for API consumers.
/// Derives from EventStatus + current time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayStatus {
    Draft,
    Upcoming,
    Ongoing,
    Ended,
    Cancelled,
    Archived,
}

impl Event {
    pub fn display_status(&self, now: DateTime<Utc>) -> DisplayStatus {
        match self.event_status {
            EventStatus::Draft => DisplayStatus::Draft,
            EventStatus::Cancelled => DisplayStatus::Cancelled,
            EventStatus::Archived => DisplayStatus::Archived,
            EventStatus::Published => {
                if self.start_time > now {
                    DisplayStatus::Upcoming
                } else if let Some(end_time) = self.end_time {
                    if now <= end_time {
                        DisplayStatus::Ongoing
                    } else {
                        DisplayStatus::Ended
                    }
                } else {
                    // No end_time set, started but we don't know when it ends
                    DisplayStatus::Ongoing
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTag {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}
