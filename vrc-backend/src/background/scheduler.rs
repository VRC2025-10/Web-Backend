use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, TimeZone, Timelike, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

/// Maximum backoff duration between retries on transient database errors.
const MAX_BACKOFF: Duration = Duration::from_secs(300);

/// Initial backoff duration after a database error.
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);

const NOTIFICATION_INTERVAL: Duration = Duration::from_secs(60);
const NOTIFICATION_LOOKBACK_MINUTES: i64 = 2;
const NOTIFICATION_MAX_ATTEMPTS: i32 = 5;
const DISCORD_MESSAGE_LIMIT: usize = 2_000;
const JST_OFFSET_SECONDS: i32 = 9 * 60 * 60;

/// Spawn background tasks that run on a periodic schedule.
pub fn spawn(
    pool: PgPool,
    http_client: reqwest::Client,
    session_cleanup_interval_secs: u64,
    event_archival_interval_secs: u64,
) {
    tokio::spawn(session_cleanup_loop(
        pool.clone(),
        session_cleanup_interval_secs,
    ));
    tokio::spawn(event_archival_loop(pool.clone(), event_archival_interval_secs));
    tokio::spawn(schedule_notification_loop(pool, http_client));
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

#[derive(Debug, Clone, sqlx::FromRow)]
struct ScheduleNotificationSettingRow {
    webhook_url: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ScheduleNotificationRuleRow {
    id: uuid::Uuid,
    name: String,
    enabled: bool,
    schedule_type: String,
    offset_minutes: Option<i32>,
    time_of_day_minutes: Option<i32>,
    window_start_minutes: Option<i32>,
    window_end_minutes: Option<i32>,
    body_template: String,
    list_item_template: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ScheduleNotificationDeliveryRow {
    id: uuid::Uuid,
    rule_id: uuid::Uuid,
    event_id: Option<uuid::Uuid>,
    scheduled_for: DateTime<Utc>,
    attempt_count: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ScheduleEventRow {
    id: uuid::Uuid,
    title: String,
    description: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    visibility_mode: String,
    auto_notify_enabled: bool,
}

async fn schedule_notification_loop(pool: PgPool, http_client: reqwest::Client) {
    let mut interval = tokio::time::interval(NOTIFICATION_INTERVAL);
    let mut backoff = INITIAL_BACKOFF;
    interval.tick().await;

    loop {
        interval.tick().await;

        match dispatch_schedule_notifications(&pool, &http_client).await {
            Ok(()) => {
                backoff = INITIAL_BACKOFF;
            }
            Err(error_value) => {
                error!(
                    error = %error_value,
                    retry_after_secs = backoff.as_secs(),
                    "Failed to process schedule notifications, will retry"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

async fn dispatch_schedule_notifications(
    pool: &PgPool,
    http_client: &reqwest::Client,
) -> Result<(), sqlx::Error> {
    let Some(setting) = load_schedule_notification_setting(pool).await? else {
        return Ok(());
    };

    let now = Utc::now();
    let since = now - ChronoDuration::minutes(NOTIFICATION_LOOKBACK_MINUTES);
    let rules = load_enabled_schedule_rules(pool).await?;

    for rule in &rules {
        match rule.schedule_type.as_str() {
            "before_event" => schedule_before_event_deliveries(pool, rule, since, now).await?,
            "daily_at" => schedule_daily_deliveries(pool, rule, since, now).await?,
            _ => {}
        }
    }

    let deliveries = load_retryable_deliveries(pool).await?;
    let rules_by_id = rules
        .into_iter()
        .map(|rule| (rule.id, rule))
        .collect::<HashMap<_, _>>();

    for delivery in deliveries {
        let Some(rule) = rules_by_id.get(&delivery.rule_id) else {
            mark_delivery_sent(pool, delivery.id).await?;
            continue;
        };

        if !rule.enabled {
            mark_delivery_sent(pool, delivery.id).await?;
            continue;
        }

        let rendered = render_delivery_message(pool, rule, &delivery).await;
        let message = match rendered {
            Ok(Some(message)) => message,
            Ok(None) => {
                mark_delivery_sent(pool, delivery.id).await?;
                continue;
            }
            Err(error_value) => {
                mark_delivery_failed(pool, delivery.id, delivery.attempt_count + 1, &error_value.to_string()).await?;
                continue;
            }
        };

        match send_schedule_webhook(http_client, &setting.webhook_url, &message).await {
            Ok(()) => {
                mark_delivery_sent(pool, delivery.id).await?;
            }
            Err(error_value) => {
                mark_delivery_failed(pool, delivery.id, delivery.attempt_count + 1, &error_value).await?;
            }
        }
    }

    Ok(())
}

async fn load_schedule_notification_setting(
    pool: &PgPool,
) -> Result<Option<ScheduleNotificationSettingRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleNotificationSettingRow>(
        "SELECT webhook_url FROM schedule_notification_settings WHERE id = TRUE",
    )
    .fetch_optional(pool)
    .await
}

async fn load_enabled_schedule_rules(
    pool: &PgPool,
) -> Result<Vec<ScheduleNotificationRuleRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleNotificationRuleRow>(
        r#"
        SELECT id, name, enabled, schedule_type, offset_minutes, time_of_day_minutes,
               window_start_minutes, window_end_minutes, body_template, list_item_template
        FROM schedule_notification_rules
        WHERE enabled = TRUE
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

async fn schedule_before_event_deliveries(
    pool: &PgPool,
    rule: &ScheduleNotificationRuleRow,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let Some(offset_minutes) = rule.offset_minutes else {
        return Ok(());
    };

    let from = since + ChronoDuration::minutes(i64::from(offset_minutes));
    let to = now + ChronoDuration::minutes(i64::from(offset_minutes));
    let events = list_public_auto_notify_events_in_range(pool, from, to).await?;

    for event in events {
        let scheduled_for = (event.start_at - ChronoDuration::minutes(i64::from(offset_minutes)))
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .unwrap_or(event.start_at);
        let delivery_key = format!("before_event:{}:{}:{}", rule.id, event.id, scheduled_for.to_rfc3339());

        create_delivery(pool, rule.id, Some(event.id), &delivery_key, scheduled_for).await?;
    }

    Ok(())
}

async fn schedule_daily_deliveries(
    pool: &PgPool,
    rule: &ScheduleNotificationRuleRow,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let Some(time_of_day_minutes) = rule.time_of_day_minutes else {
        return Ok(());
    };

    for scheduled_for in due_daily_schedule_times(time_of_day_minutes, since, now) {
        let delivery_key = format!("daily_at:{}:{}", rule.id, scheduled_for.to_rfc3339());
        create_delivery(pool, rule.id, None, &delivery_key, scheduled_for).await?;
    }

    Ok(())
}

fn due_daily_schedule_times(
    time_of_day_minutes: i32,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let offset = jst_offset();
    let mut current_day = since.with_timezone(&offset).date_naive();
    let end_day = now.with_timezone(&offset).date_naive();
    let hour = time_of_day_minutes.div_euclid(60);
    let minute = time_of_day_minutes.rem_euclid(60);
    let mut scheduled = Vec::new();

    while current_day <= end_day {
        if let Some(local_time) = current_day.and_hms_opt(hour as u32, minute as u32, 0)
            && let Some(local_date_time) = offset.from_local_datetime(&local_time).single()
        {
            let scheduled_for = local_date_time.with_timezone(&Utc);
            if scheduled_for > since && scheduled_for <= now {
                scheduled.push(scheduled_for);
            }
        }

        let Some(next_day) = current_day.succ_opt() else {
            break;
        };
        current_day = next_day;
    }

    scheduled
}

async fn create_delivery(
    pool: &PgPool,
    rule_id: uuid::Uuid,
    event_id: Option<uuid::Uuid>,
    delivery_key: &str,
    scheduled_for: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO schedule_notification_deliveries (rule_id, event_id, delivery_key, scheduled_for)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (delivery_key) DO NOTHING
        "#,
    )
    .bind(rule_id)
    .bind(event_id)
    .bind(delivery_key)
    .bind(scheduled_for)
    .execute(pool)
    .await?;

    Ok(())
}

async fn load_retryable_deliveries(
    pool: &PgPool,
) -> Result<Vec<ScheduleNotificationDeliveryRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleNotificationDeliveryRow>(
        r#"
        SELECT id, rule_id, event_id, scheduled_for, attempt_count
        FROM schedule_notification_deliveries
        WHERE status IN ('pending', 'failed')
          AND scheduled_for <= now()
          AND attempt_count < $1
        ORDER BY scheduled_for ASC
        LIMIT 100
        "#,
    )
    .bind(NOTIFICATION_MAX_ATTEMPTS)
    .fetch_all(pool)
    .await
}

async fn render_delivery_message(
    pool: &PgPool,
    rule: &ScheduleNotificationRuleRow,
    delivery: &ScheduleNotificationDeliveryRow,
) -> Result<Option<String>, sqlx::Error> {
    match rule.schedule_type.as_str() {
        "before_event" => render_before_event_message(pool, rule, delivery).await,
        "daily_at" => render_daily_message(pool, rule, delivery).await,
        _ => Ok(None),
    }
}

async fn render_before_event_message(
    pool: &PgPool,
    rule: &ScheduleNotificationRuleRow,
    delivery: &ScheduleNotificationDeliveryRow,
) -> Result<Option<String>, sqlx::Error> {
    let Some(event_id) = delivery.event_id else {
        return Ok(None);
    };

    let Some(event) = load_schedule_event(pool, event_id).await? else {
        return Ok(None);
    };
    if event.visibility_mode != "public" || !event.auto_notify_enabled {
        return Ok(None);
    }

    let message = truncate_discord_message(&replace_placeholders(
        &rule.body_template,
        &event_placeholder_values(&rule.name, &event),
    ));
    Ok(Some(message))
}

async fn render_daily_message(
    pool: &PgPool,
    rule: &ScheduleNotificationRuleRow,
    delivery: &ScheduleNotificationDeliveryRow,
) -> Result<Option<String>, sqlx::Error> {
    let Some(window_start_minutes) = rule.window_start_minutes else {
        return Ok(None);
    };
    let Some(window_end_minutes) = rule.window_end_minutes else {
        return Ok(None);
    };

    let window_start = delivery.scheduled_for + ChronoDuration::minutes(i64::from(window_start_minutes));
    let window_end = delivery.scheduled_for + ChronoDuration::minutes(i64::from(window_end_minutes));
    let events = list_public_auto_notify_events_in_range(pool, window_start, window_end).await?;
    if events.is_empty() {
        return Ok(None);
    }

    let list_template = if rule.list_item_template.trim().is_empty() {
        "- {{start_at}} {{title}}"
    } else {
        rule.list_item_template.as_str()
    };
    let events_list = events
        .iter()
        .map(|event| replace_placeholders(list_template, &event_placeholder_values(&rule.name, event)))
        .collect::<Vec<_>>()
        .join("\n");

    let values = HashMap::from([
        ("event_count", events.len().to_string()),
        ("events_list", events_list),
        ("rule_name", rule.name.clone()),
        ("window_start", format_date_time(window_start)),
        ("window_end", format_date_time(window_end)),
    ]);
    let message = truncate_discord_message(&replace_placeholders(&rule.body_template, &values));

    Ok(Some(message))
}

async fn list_public_auto_notify_events_in_range(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<ScheduleEventRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleEventRow>(
        r#"
        SELECT id, title, description, start_at, end_at, visibility_mode, auto_notify_enabled
        FROM schedule_events
        WHERE visibility_mode = 'public'
          AND auto_notify_enabled = TRUE
          AND start_at >= $1
          AND start_at <= $2
        ORDER BY start_at ASC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
}

async fn load_schedule_event(
    pool: &PgPool,
    event_id: uuid::Uuid,
) -> Result<Option<ScheduleEventRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleEventRow>(
        r#"
        SELECT id, title, description, start_at, end_at, visibility_mode, auto_notify_enabled
        FROM schedule_events
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .fetch_optional(pool)
    .await
}

async fn mark_delivery_sent(pool: &PgPool, delivery_id: uuid::Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE schedule_notification_deliveries
        SET status = 'sent',
            delivered_at = now(),
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn mark_delivery_failed(
    pool: &PgPool,
    delivery_id: uuid::Uuid,
    next_attempt_count: i32,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    let next_status = if next_attempt_count >= NOTIFICATION_MAX_ATTEMPTS {
        "failed"
    } else {
        "pending"
    };

    sqlx::query(
        r#"
        UPDATE schedule_notification_deliveries
        SET status = $2,
            attempt_count = $3,
            last_error = $4,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(next_status)
    .bind(next_attempt_count)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

async fn send_schedule_webhook(
    http_client: &reqwest::Client,
    webhook_url: &str,
    message: &str,
) -> Result<(), String> {
    let response = http_client
        .post(webhook_url)
        .json(&json!({ "content": message }))
        .send()
        .await
        .map_err(|error_value| error_value.to_string())?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!("Discord returned {status}: {body}"))
}

fn event_placeholder_values(
    rule_name: &str,
    event: &ScheduleEventRow,
) -> HashMap<&'static str, String> {
    HashMap::from([
        ("description", event.description.clone()),
        ("duration", format_duration(event.start_at, event.end_at)),
        ("end_at", format_date_time(event.end_at)),
        ("end_date", format_date(event.end_at)),
        ("end_time", format_time(event.end_at)),
        ("rule_name", rule_name.to_owned()),
        ("start_at", format_date_time(event.start_at)),
        ("start_date", format_date(event.start_at)),
        ("start_time", format_time(event.start_at)),
        ("title", event.title.clone()),
    ])
}

fn replace_placeholders(template: &str, values: &HashMap<&'static str, String>) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

fn truncate_discord_message(message: &str) -> String {
    if message.len() <= DISCORD_MESSAGE_LIMIT {
        return message.to_owned();
    }

    let truncated = &message[..DISCORD_MESSAGE_LIMIT.saturating_sub(1)];
    format!("{truncated}…")
}

fn format_duration(start_at: DateTime<Utc>, end_at: DateTime<Utc>) -> String {
    let total_minutes = (end_at - start_at).num_minutes().max(0);
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    match (hours, minutes) {
        (0, minutes_only) => format!("{minutes_only}m"),
        (hours_only, 0) => format!("{hours_only}h"),
        (hours_only, minutes_only) => format!("{hours_only}h {minutes_only}m"),
    }
}

fn format_date_time(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&jst_offset())
        .format("%Y-%m-%d %H:%M JST")
        .to_string()
}

fn format_date(value: DateTime<Utc>) -> String {
    value.with_timezone(&jst_offset()).format("%Y-%m-%d").to_string()
}

fn format_time(value: DateTime<Utc>) -> String {
    value.with_timezone(&jst_offset()).format("%H:%M").to_string()
}

fn jst_offset() -> FixedOffset {
    FixedOffset::east_opt(JST_OFFSET_SECONDS).expect("valid JST offset")
}
