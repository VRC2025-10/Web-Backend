use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::club::Club;
use crate::domain::ports::repositories::club_repository::{
    ClubDetailRow, ClubListRow, ClubMemberRow, ClubRepository, UserClubRow,
};
use crate::errors::infrastructure::InfraError;

pub struct PgClubRepository {
    pool: PgPool,
}

impl PgClubRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ClubRepository for PgClubRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Club>, InfraError> {
        let row = sqlx::query_as!(
            Club,
            r#"
            SELECT id, name, description_markdown, description_html,
                   cover_image_url, owner_user_id, created_at, updated_at
            FROM clubs WHERE id = $1
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
    ) -> Result<(Vec<ClubListRow>, i64), InfraError> {
        let rows = sqlx::query_as!(
            ClubListRow,
            r#"
            SELECT c.id, c.name, c.description_html, c.cover_image_url, c.owner_user_id,
                   u.discord_id as owner_discord_id, u.discord_display_name as owner_discord_display_name,
                   (SELECT COUNT(*) FROM club_members cm WHERE cm.club_id = c.id) as "member_count!: i64",
                   c.created_at
            FROM clubs c
            JOIN users u ON u.id = c.owner_user_id
            ORDER BY c.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!: i64" FROM clubs"#)
            .fetch_one(&self.pool)
            .await?;

        Ok((rows, count))
    }

    async fn get_detail(&self, id: Uuid) -> Result<Option<ClubDetailRow>, InfraError> {
        let row = sqlx::query_as!(
            ClubDetailRow,
            r#"
            SELECT c.id, c.name, c.description_html, c.cover_image_url, c.owner_user_id,
                   u.discord_id as owner_discord_id, u.discord_display_name as owner_discord_display_name,
                   c.created_at
            FROM clubs c
            JOIN users u ON u.id = c.owner_user_id
            WHERE c.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_members(&self, club_id: Uuid) -> Result<Vec<ClubMemberRow>, InfraError> {
        let rows = sqlx::query_as!(
            ClubMemberRow,
            r#"
            SELECT cm.user_id, u.discord_id, u.discord_display_name, cm.role, cm.joined_at,
                   (c.owner_user_id = cm.user_id) as "is_owner!: bool"
            FROM club_members cm
            JOIN users u ON u.id = cm.user_id
            JOIN clubs c ON c.id = cm.club_id
            WHERE cm.club_id = $1
            ORDER BY cm.joined_at ASC
            "#,
            club_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_clubs_for_user(&self, user_id: Uuid) -> Result<Vec<UserClubRow>, InfraError> {
        let rows = sqlx::query_as!(
            UserClubRow,
            r#"
            SELECT c.id, c.name, (c.owner_user_id = $1) as "is_owner!: bool"
            FROM clubs c
            JOIN club_members cm ON cm.club_id = c.id
            WHERE cm.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
