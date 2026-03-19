use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::report::{Report, ReportTargetType};
use crate::domain::ports::repositories::report_repository::ReportRepository;
use crate::errors::infrastructure::InfraError;

pub struct PgReportRepository {
    pool: PgPool,
}

impl PgReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ReportRepository for PgReportRepository {
    async fn create(
        &self,
        reporter_user_id: Uuid,
        target_type: ReportTargetType,
        target_id: &str,
        reason: &str,
    ) -> Result<Report, InfraError> {
        let row = sqlx::query_as::<_, Report>(
            r#"
            INSERT INTO reports (reporter_user_id, target_type, target_id, reason)
            VALUES ($1, $2, $3, $4)
            RETURNING id, reporter_user_id, target_type, target_id, reason, status, created_at
            "#,
        )
        .bind(reporter_user_id)
        .bind(target_type)
        .bind(target_id)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn exists(
        &self,
        reporter_user_id: Uuid,
        target_type: ReportTargetType,
        target_id: &str,
    ) -> Result<bool, InfraError> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM reports
                WHERE reporter_user_id = $1 AND target_type = $2 AND target_id = $3
            )
            "#,
        )
        .bind(reporter_user_id)
        .bind(target_type)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }
}
