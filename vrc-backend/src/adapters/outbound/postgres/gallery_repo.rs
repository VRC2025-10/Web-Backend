use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ports::repositories::gallery_repository::{GalleryImageRow, GalleryRepository};
use crate::errors::infrastructure::InfraError;

pub struct PgGalleryRepository {
    pool: PgPool,
}

impl PgGalleryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl GalleryRepository for PgGalleryRepository {
    async fn list_approved(
        &self,
        club_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<GalleryImageRow>, i64), InfraError> {
        let rows = sqlx::query_as!(
            GalleryImageRow,
            r#"
            SELECT g.id, g.club_id, g.image_url, g.caption, g.uploaded_by_user_id,
                   u.discord_id as uploader_discord_id,
                   u.discord_display_name as uploader_discord_display_name,
                   g.created_at
            FROM gallery_images g
            JOIN users u ON u.id = g.uploaded_by_user_id
            WHERE g.club_id = $1 AND g.status = 'approved'
            ORDER BY g.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            club_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM gallery_images
            WHERE club_id = $1 AND status = 'approved'
            "#,
            club_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, count))
    }
}
