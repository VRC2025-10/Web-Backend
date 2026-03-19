use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

/// Maximum backoff duration between retries on transient database errors.
const MAX_BACKOFF: Duration = Duration::from_secs(300);

/// Initial backoff duration after a database error.
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);

/// Spawn background tasks that run on a periodic schedule.
pub fn spawn(pool: PgPool, session_cleanup_interval_secs: u64, event_archival_interval_secs: u64) {
    tokio::spawn(session_cleanup_loop(
        pool.clone(),
        session_cleanup_interval_secs,
    ));
    tokio::spawn(event_archival_loop(pool, event_archival_interval_secs));
}

/// Delete expired sessions on the configured interval.
///
/// Uses exponential backoff on database errors to avoid hammering a
/// temporarily unavailable database.
async fn session_cleanup_loop(pool: PgPool, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut backoff = INITIAL_BACKOFF;
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
                backoff = INITIAL_BACKOFF;
            }
            Err(e) => {
                error!(
                    error = %e,
                    retry_after_secs = backoff.as_secs(),
                    "Failed to clean up expired sessions, will retry"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

/// Archive published events that are older than their retention threshold.
///
/// Events with an `end_time` are archived 30 days after `end_time`.
/// Events without `end_time` (open-ended) are archived 60 days after `start_time`
/// to prevent indefinite accumulation in the active event list.
///
/// Interval is configurable via `EVENT_ARCHIVAL_INTERVAL_SECS` (default: 3600).
/// Uses exponential backoff on failures.
async fn event_archival_loop(pool: PgPool, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut backoff = INITIAL_BACKOFF;
    interval.tick().await;

    loop {
        interval.tick().await;

        match sqlx::query!(
            r#"
            UPDATE events
            SET event_status = 'archived', updated_at = now()
            WHERE event_status = 'published'
              AND (
                  (end_time IS NOT NULL AND end_time < now() - INTERVAL '30 days')
                  OR
                  (end_time IS NULL AND start_time < now() - INTERVAL '60 days')
              )
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
                backoff = INITIAL_BACKOFF;
            }
            Err(e) => {
                error!(
                    error = %e,
                    retry_after_secs = backoff.as_secs(),
                    "Failed to archive events, will retry"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}
