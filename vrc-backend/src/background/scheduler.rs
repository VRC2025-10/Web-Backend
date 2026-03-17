use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

/// Spawn background tasks that run on a periodic schedule.
pub fn spawn(pool: PgPool, session_cleanup_interval_secs: u64) {
    tokio::spawn(session_cleanup_loop(
        pool.clone(),
        session_cleanup_interval_secs,
    ));
    tokio::spawn(event_archival_loop(pool));
}

/// Delete expired sessions on the configured interval.
async fn session_cleanup_loop(pool: PgPool, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    // Skip the first immediate tick
    interval.tick().await;

    loop {
        interval.tick().await;

        match sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
            .execute(&pool)
            .await
        {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    info!(deleted = count, "Cleaned up expired sessions");
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to clean up expired sessions");
            }
        }
    }
}

/// Archive published events whose `end_time` is older than 30 days.
/// Runs once per hour.
async fn event_archival_loop(pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
    interval.tick().await;

    loop {
        interval.tick().await;

        match sqlx::query!(
            r#"
            UPDATE events
            SET event_status = 'archived', updated_at = now()
            WHERE event_status = 'published'
              AND end_time IS NOT NULL
              AND end_time < now() - INTERVAL '30 days'
            "#
        )
        .execute(&pool)
        .await
        {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    info!(archived = count, "Archived old events");
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to archive events");
            }
        }
    }
}
