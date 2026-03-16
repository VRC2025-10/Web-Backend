use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::event::{Event, EventStatus, EventTag};
use crate::domain::ports::repositories::event_repository::EventRepository;
use crate::errors::infrastructure::InfraError;

pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl EventRepository for PgEventRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Event>, InfraError> {
        let row = sqlx::query_as!(
            Event,
            r#"
            SELECT id, external_source_id, title, description_markdown, description_html,
                   host_user_id, host_name, event_status as "event_status: EventStatus",
                   start_time, end_time, location, created_at, updated_at
            FROM events WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        status_filter: Option<EventStatus>,
    ) -> Result<(Vec<Event>, i64), InfraError> {
        let events = sqlx::query_as!(
            Event,
            r#"
            SELECT id, external_source_id, title, description_markdown, description_html,
                   host_user_id, host_name, event_status as "event_status: EventStatus",
                   start_time, end_time, location, created_at, updated_at
            FROM events
            WHERE ($1::event_status IS NULL OR event_status = $1)
            ORDER BY start_time DESC
            LIMIT $2 OFFSET $3
            "#,
            status_filter as Option<EventStatus>,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM events
            WHERE ($1::event_status IS NULL OR event_status = $1)
            "#,
            status_filter as Option<EventStatus>,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((events, count))
    }

    async fn get_tags_for_event(&self, event_id: Uuid) -> Result<Vec<EventTag>, InfraError> {
        let tags = sqlx::query_as!(
            EventTag,
            r#"
            SELECT t.id, t.name, t.color
            FROM event_tags t
            JOIN event_tag_mappings m ON m.tag_id = t.id
            WHERE m.event_id = $1
            "#,
            event_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    async fn get_tags_for_events(
        &self,
        event_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, EventTag)>, InfraError> {
        let rows = sqlx::query!(
            r#"
            SELECT m.event_id, t.id, t.name, t.color
            FROM event_tags t
            JOIN event_tag_mappings m ON m.tag_id = t.id
            WHERE m.event_id = ANY($1)
            "#,
            event_ids
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.event_id,
                    EventTag {
                        id: r.id,
                        name: r.name,
                        color: r.color,
                    },
                )
            })
            .collect())
    }
}
