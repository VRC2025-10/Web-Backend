use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ports::repositories::session_repository::SessionRepository;
use crate::errors::infrastructure::InfraError;

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SessionRepository for PgSessionRepository {
    async fn create(
        &self,
        user_id: Uuid,
        token_hash: &[u8],
        max_age_secs: i64,
    ) -> Result<Uuid, InfraError> {
        let row = sqlx::query_scalar!(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, NOW() + make_interval(secs => $3::double precision))
            RETURNING id
            "#,
            user_id,
            token_hash,
            max_age_secs as f64
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_by_token_hash(&self, token_hash: &[u8]) -> Result<(), InfraError> {
        sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), InfraError> {
        sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<u64, InfraError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}
