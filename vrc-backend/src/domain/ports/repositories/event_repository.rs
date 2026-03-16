use uuid::Uuid;

use crate::domain::entities::event::{Event, EventStatus, EventTag};
use crate::errors::infrastructure::InfraError;

pub trait EventRepository: Send + Sync {
    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<Event>, InfraError>> + Send;

    fn list(
        &self,
        limit: i64,
        offset: i64,
        status_filter: Option<EventStatus>,
    ) -> impl std::future::Future<Output = Result<(Vec<Event>, i64), InfraError>> + Send;

    fn get_tags_for_event(
        &self,
        event_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<EventTag>, InfraError>> + Send;

    fn get_tags_for_events(
        &self,
        event_ids: &[Uuid],
    ) -> impl std::future::Future<Output = Result<Vec<(Uuid, EventTag)>, InfraError>> + Send;
}
