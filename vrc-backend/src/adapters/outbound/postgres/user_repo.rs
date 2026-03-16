use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::user::{User, UserRole, UserStatus};
use crate::domain::ports::repositories::user_repository::UserRepository;
use crate::domain::value_objects::pagination::PageRequest;
use crate::errors::infrastructure::InfraError;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, InfraError> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, discord_id, discord_username, discord_display_name,
                   discord_avatar_hash, avatar_url,
                   role as "role: UserRole", status as "status: UserStatus",
                   joined_at, created_at, updated_at
            FROM users WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_by_discord_id(&self, discord_id: &str) -> Result<Option<User>, InfraError> {
        let row = sqlx::query_as!(
            User,
            r#"
            SELECT id, discord_id, discord_username, discord_display_name,
                   discord_avatar_hash, avatar_url,
                   role as "role: UserRole", status as "status: UserStatus",
                   joined_at, created_at, updated_at
            FROM users WHERE discord_id = $1
            "#,
            discord_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_from_discord(
        &self,
        discord_id: &str,
        discord_username: &str,
        avatar_url: Option<&str>,
    ) -> Result<User, InfraError> {
        let row = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (discord_id, discord_username, discord_display_name, avatar_url)
            VALUES ($1, $2, $2, $3)
            ON CONFLICT (discord_id) DO UPDATE SET
                discord_username = EXCLUDED.discord_username,
                discord_display_name = EXCLUDED.discord_display_name,
                avatar_url = EXCLUDED.avatar_url,
                updated_at = NOW()
            RETURNING id, discord_id, discord_username, discord_display_name,
                      discord_avatar_hash, avatar_url,
                      role as "role: UserRole", status as "status: UserStatus",
                      joined_at, created_at, updated_at
            "#,
            discord_id,
            discord_username,
            avatar_url
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_role(&self, user_id: Uuid, new_role: UserRole) -> Result<(), InfraError> {
        sqlx::query!(
            "UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2",
            new_role as UserRole,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_status(
        &self,
        user_id: Uuid,
        new_status: UserStatus,
    ) -> Result<(), InfraError> {
        sqlx::query!(
            "UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2",
            new_status as UserStatus,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_all(
        &self,
        page: &PageRequest,
        status_filter: Option<UserStatus>,
        role_filter: Option<UserRole>,
    ) -> Result<(Vec<User>, i64), InfraError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, discord_id, discord_username, discord_display_name,
                   discord_avatar_hash, avatar_url,
                   role as "role: UserRole", status as "status: UserStatus",
                   joined_at, created_at, updated_at
            FROM users
            WHERE ($1::user_status IS NULL OR status = $1)
              AND ($2::user_role IS NULL OR role = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            status_filter as Option<UserStatus>,
            role_filter as Option<UserRole>,
            page.limit(),
            page.offset()
        )
        .fetch_all(&self.pool)
        .await?;

        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM users
            WHERE ($1::user_status IS NULL OR status = $1)
              AND ($2::user_role IS NULL OR role = $2)
            "#,
            status_filter as Option<UserStatus>,
            role_filter as Option<UserRole>,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((users, count))
    }
}
