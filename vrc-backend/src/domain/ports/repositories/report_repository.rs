use uuid::Uuid;

use crate::domain::entities::report::{Report, ReportTargetType};
use crate::errors::infrastructure::InfraError;

pub trait ReportRepository: Send + Sync {
    fn create(
        &self,
        reporter_user_id: Uuid,
        target_type: ReportTargetType,
        target_id: Uuid,
        reason: &str,
    ) -> impl std::future::Future<Output = Result<Report, InfraError>> + Send;

    fn exists(
        &self,
        reporter_user_id: Uuid,
        target_type: ReportTargetType,
        target_id: Uuid,
    ) -> impl std::future::Future<Output = Result<bool, InfraError>> + Send;
}
