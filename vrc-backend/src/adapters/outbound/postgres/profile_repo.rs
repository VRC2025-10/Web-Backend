use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::profile::Profile;
use crate::domain::ports::repositories::profile_repository::{
    ProfileRepository, PublicMemberDetailRow, PublicMemberRow,
};
use crate::errors::infrastructure::InfraError;

pub struct PgProfileRepository {
    pool: PgPool,
}

impl PgProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ProfileRepository for PgProfileRepository {
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Profile>, InfraError> {
        let row = sqlx::query_as!(
            Profile,
            r#"
            SELECT user_id, nickname, vrc_id, x_id, bio_markdown, bio_html,
                   avatar_url, is_public, updated_at
            FROM profiles WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert(&self, profile: &Profile) -> Result<Profile, InfraError> {
        let row = sqlx::query_as!(
            Profile,
            r#"
            INSERT INTO profiles (user_id, nickname, vrc_id, x_id, bio_markdown, bio_html, avatar_url, is_public, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                nickname = EXCLUDED.nickname,
                vrc_id = EXCLUDED.vrc_id,
                x_id = EXCLUDED.x_id,
                bio_markdown = EXCLUDED.bio_markdown,
                bio_html = EXCLUDED.bio_html,
                avatar_url = EXCLUDED.avatar_url,
                is_public = EXCLUDED.is_public,
                updated_at = NOW()
            RETURNING user_id, nickname, vrc_id, x_id, bio_markdown, bio_html, avatar_url, is_public, updated_at
            "#,
            profile.user_id,
            profile.nickname,
            profile.vrc_id,
            profile.x_id,
            profile.bio_markdown,
            profile.bio_html,
            profile.avatar_url,
            profile.is_public,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_public(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<PublicMemberRow>, i64), InfraError> {
        let rows = sqlx::query_as!(
            PublicMemberRow,
            r#"
            SELECT u.id as user_id, u.discord_id, u.discord_display_name,
                   u.discord_avatar_hash, u.joined_at,
                   p.nickname, p.avatar_url as profile_avatar_url, p.bio_html,
                   p.vrc_id, p.x_id
            FROM users u
            LEFT JOIN profiles p ON p.user_id = u.id AND p.is_public = true
            WHERE u.status = 'active'
            ORDER BY u.joined_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM users
            WHERE status = 'active'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, count))
    }

    async fn find_public_by_discord_id(
        &self,
        discord_id: &str,
    ) -> Result<Option<PublicMemberDetailRow>, InfraError> {
        let row = sqlx::query_as!(
            PublicMemberDetailRow,
            r#"
            SELECT u.id as user_id, u.discord_id, u.discord_display_name,
                   u.discord_avatar_hash, u.joined_at,
                   p.nickname, p.avatar_url as profile_avatar_url, p.bio_html,
                   p.vrc_id, p.x_id, p.updated_at as profile_updated_at
            FROM users u
            LEFT JOIN profiles p ON p.user_id = u.id AND p.is_public = true
            WHERE u.discord_id = $1 AND u.status = 'active'
            "#,
            discord_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn set_private(&self, user_id: Uuid) -> Result<(), InfraError> {
        sqlx::query!(
            "UPDATE profiles SET is_public = false, updated_at = NOW() WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
