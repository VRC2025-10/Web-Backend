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
/// Derives from `EventStatus` + current time.
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

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn sample_event(status: EventStatus, start_time: DateTime<Utc>, end_time: Option<DateTime<Utc>>) -> Event {
        Event {
            id: Uuid::nil(),
            external_source_id: Some("external-id".to_owned()),
            title: "Event".to_owned(),
            description_markdown: String::new(),
            description_html: String::new(),
            host_user_id: None,
            host_name: String::new(),
            event_status: status,
            start_time,
            end_time,
            location: None,
            created_at: Utc.timestamp_opt(0, 0).single().expect("timestamp must be valid"),
            updated_at: Utc.timestamp_opt(0, 0).single().expect("timestamp must be valid"),
        }
    }

    // Spec refs: public-api.md event display_status contract.
    // Coverage: all display state transitions derived from status and time windows.

    #[test]
    fn test_display_status_for_draft_event_is_draft() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(EventStatus::Draft, now + Duration::hours(1), None);

        assert!(matches!(event.display_status(now), DisplayStatus::Draft));
    }

    #[test]
    fn test_display_status_for_cancelled_event_is_cancelled() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(EventStatus::Cancelled, now - Duration::hours(1), None);

        assert!(matches!(event.display_status(now), DisplayStatus::Cancelled));
    }

    #[test]
    fn test_display_status_for_archived_event_is_archived() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(EventStatus::Archived, now - Duration::hours(1), None);

        assert!(matches!(event.display_status(now), DisplayStatus::Archived));
    }

    #[test]
    fn test_display_status_for_published_future_event_is_upcoming() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(
            EventStatus::Published,
            now + Duration::hours(2),
            Some(now + Duration::hours(3)),
        );

        assert!(matches!(event.display_status(now), DisplayStatus::Upcoming));
    }

    #[test]
    fn test_display_status_for_published_event_during_window_is_ongoing() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(
            EventStatus::Published,
            now - Duration::minutes(30),
            Some(now + Duration::minutes(30)),
        );

        assert!(matches!(event.display_status(now), DisplayStatus::Ongoing));
    }

    #[test]
    fn test_display_status_for_published_event_after_end_is_ended() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(
            EventStatus::Published,
            now - Duration::hours(2),
            Some(now - Duration::minutes(1)),
        );

        assert!(matches!(event.display_status(now), DisplayStatus::Ended));
    }

    #[test]
    fn test_display_status_for_published_event_without_end_time_is_ongoing_after_start() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().expect("timestamp must be valid");
        let event = sample_event(EventStatus::Published, now - Duration::minutes(5), None);

        assert!(matches!(event.display_status(now), DisplayStatus::Ongoing));
    }
}
