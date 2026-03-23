//! Internal schedule board routes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};
use reqwest::Url;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::adapters::inbound::extractors::ValidatedQuery;
use crate::adapters::outbound::discord::client::ReqwestDiscordClient;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::roles::Member;
use crate::domain::entities::user::{User, UserRole};
use crate::domain::ports::services::discord_client::DiscordClient;
use crate::errors::api::ApiError;

const JST_OFFSET_SECONDS: i32 = 9 * 60 * 60;
const MAX_TIMELINE_DAYS: u32 = 62;

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct SchedulePermissionSet {
    manage_roles: bool,
    manage_events: bool,
    manage_templates: bool,
    manage_notifications: bool,
    view_restricted_events: bool,
}

impl SchedulePermissionSet {
    fn merge(self, other: Self) -> Self {
        Self {
            manage_roles: self.manage_roles || other.manage_roles,
            manage_events: self.manage_events || other.manage_events,
            manage_templates: self.manage_templates || other.manage_templates,
            manage_notifications: self.manage_notifications || other.manage_notifications,
            view_restricted_events: self.view_restricted_events || other.view_restricted_events,
        }
    }

    fn has_any(self) -> bool {
        self.manage_roles
            || self.manage_events
            || self.manage_templates
            || self.manage_notifications
            || self.view_restricted_events
    }
}

#[derive(Debug, Clone)]
struct ScheduleViewer {
    user: User,
    discord_role_ids: Vec<String>,
    permissions: SchedulePermissionSet,
}

#[derive(Debug, Deserialize)]
struct ScheduleBootstrapQuery {
    from: Option<String>,
    days: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ScheduleBootstrapResponse {
    viewer: ScheduleViewerResponse,
    timeline: ScheduleTimelineResponse,
    managed_roles: Vec<ScheduleManagedRoleResponse>,
    templates: Vec<ScheduleTemplateResponse>,
    notifications: Option<ScheduleNotificationStateResponse>,
}

#[derive(Debug, Serialize)]
struct ScheduleViewerResponse {
    id: Uuid,
    discord_id: String,
    discord_display_name: String,
    avatar_url: Option<String>,
    role: UserRole,
    discord_role_ids: Vec<String>,
    permissions: SchedulePermissionSet,
    schedule_access: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ScheduleManagedRoleRow {
    id: Uuid,
    discord_role_id: String,
    display_name: String,
    description: String,
    can_manage_roles: bool,
    can_manage_events: bool,
    can_manage_templates: bool,
    can_manage_notifications: bool,
    can_view_restricted_events: bool,
}

#[derive(Debug, Serialize)]
struct ScheduleManagedRoleResponse {
    id: Uuid,
    discord_role_id: String,
    display_name: String,
    description: String,
    can_manage_roles: bool,
    can_manage_events: bool,
    can_manage_templates: bool,
    can_manage_notifications: bool,
    can_view_restricted_events: bool,
}

#[derive(Debug, Deserialize)]
struct ScheduleRolePayload {
    discord_role_id: String,
    display_name: String,
    description: String,
    can_manage_roles: bool,
    can_manage_events: bool,
    can_manage_templates: bool,
    can_manage_notifications: bool,
    can_view_restricted_events: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct ScheduleTemplateRow {
    id: Uuid,
    name: String,
    title: String,
    description: String,
    is_default: bool,
}

#[derive(Debug, Serialize)]
struct ScheduleTemplateResponse {
    id: Uuid,
    name: String,
    title: String,
    description: String,
    is_default: bool,
}

#[derive(Debug, Deserialize)]
struct ScheduleTemplatePayload {
    name: String,
    title: String,
    description: String,
    is_default: bool,
}

#[derive(Debug, Deserialize)]
struct ScheduleEventPayload {
    title: String,
    description: String,
    start_at: String,
    end_at: String,
    visibility_mode: String,
    auto_notify_enabled: bool,
    visible_role_ids: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ScheduleEventRow {
    id: Uuid,
    created_by_user_id: Uuid,
    title: String,
    description: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    visibility_mode: String,
    auto_notify_enabled: bool,
    visible_role_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ScheduleTimelineResponse {
    from: String,
    days: u32,
    timezone: &'static str,
    timeline: Vec<ScheduleTimelineDay>,
}

#[derive(Debug, Serialize)]
struct ScheduleTimelineDay {
    date: String,
    events: Vec<ScheduleTimelineEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct ScheduleTimelineEvent {
    id: Option<Uuid>,
    display_mode: &'static str,
    title: Option<String>,
    description: Option<String>,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    visibility_mode: String,
    auto_notify_enabled: Option<bool>,
    visible_role_ids: Vec<String>,
    created_by_viewer: bool,
    editable: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct ScheduleNotificationSettingRow {
    webhook_url: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ScheduleNotificationRuleRow {
    id: Uuid,
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

#[derive(Debug, Deserialize)]
struct ScheduleNotificationWebhookPayload {
    webhook_url: String,
}

#[derive(Debug, Deserialize)]
struct ScheduleNotificationRulePayload {
    name: String,
    enabled: bool,
    schedule_type: String,
    offset_minutes: Option<i32>,
    time_of_day: Option<String>,
    window_start_minutes: Option<i32>,
    window_end_minutes: Option<i32>,
    body_template: String,
    list_item_template: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScheduleNotificationRuleResponse {
    id: Uuid,
    name: String,
    enabled: bool,
    schedule_type: String,
    offset_minutes: Option<i32>,
    time_of_day: Option<String>,
    window_start_minutes: Option<i32>,
    window_end_minutes: Option<i32>,
    body_template: String,
    list_item_template: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScheduleNotificationPlaceholderCatalog {
    before_event: Vec<&'static str>,
    daily_body: Vec<&'static str>,
    daily_item: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ScheduleNotificationStateResponse {
    webhook_url: String,
    rules: Vec<ScheduleNotificationRuleResponse>,
    placeholders: ScheduleNotificationPlaceholderCatalog,
}

const BEFORE_EVENT_PLACEHOLDERS: &[&str] = &[
    "description",
    "duration",
    "end_at",
    "end_date",
    "end_time",
    "rule_name",
    "start_at",
    "start_date",
    "start_time",
    "title",
];
const DAILY_BODY_PLACEHOLDERS: &[&str] = &[
    "event_count",
    "events_list",
    "rule_name",
    "window_end",
    "window_start",
];
const DAILY_ITEM_PLACEHOLDERS: &[&str] = &[
    "description",
    "duration",
    "end_at",
    "end_date",
    "end_time",
    "rule_name",
    "start_at",
    "start_date",
    "start_time",
    "title",
];

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/schedule/bootstrap", role = Member, rate_limit = "internal", summary = "Get schedule board state")]
async fn get_schedule_bootstrap(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    ValidatedQuery(query): ValidatedQuery<ScheduleBootstrapQuery>,
) -> Result<Json<ScheduleBootstrapResponse>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_schedule_access(&viewer)?;

    let (from_date, days) = parse_schedule_window(query.from.as_deref(), query.days)?;
    let managed_roles = load_managed_roles(&state).await?;
    let visible_roles = if viewer.permissions.manage_roles {
        managed_roles.clone()
    } else {
        managed_roles
            .iter()
            .filter(|role| role.can_view_restricted_events)
            .cloned()
            .collect()
    };

    let timeline = load_timeline(&state, &viewer, from_date, days).await?;
    let templates = load_templates(&state).await?;
    let notifications = if viewer.permissions.manage_notifications {
        Some(load_notification_state(&state).await?)
    } else {
        None
    };

    Ok(Json(ScheduleBootstrapResponse {
        viewer: ScheduleViewerResponse {
            id: viewer.user.id,
            discord_id: viewer.user.discord_id,
            discord_display_name: viewer.user.discord_display_name,
            avatar_url: viewer.user.avatar_url,
            role: viewer.user.role,
            discord_role_ids: viewer.discord_role_ids,
            permissions: viewer.permissions,
            schedule_access: true,
        },
        timeline,
        managed_roles: visible_roles.into_iter().map(role_to_response).collect(),
        templates: templates.into_iter().map(template_to_response).collect(),
        notifications,
    }))
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/schedule/events", role = Member, rate_limit = "internal", summary = "Create schedule event")]
async fn create_schedule_event(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(payload): Json<ScheduleEventPayload>,
) -> Result<(StatusCode, Json<ScheduleTimelineEvent>), ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_schedule_access(&viewer)?;
    let validated = validate_event_payload(&state, &viewer, payload).await?;

    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO schedule_events (
            id, created_by_user_id, title, description, start_at, end_at, visibility_mode, auto_notify_enabled
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(event_id)
    .bind(viewer.user.id)
    .bind(&validated.title)
    .bind(&validated.description)
    .bind(validated.start_at)
    .bind(validated.end_at)
    .bind(&validated.visibility_mode)
    .bind(validated.auto_notify_enabled)
    .execute(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    replace_visible_roles(&state, event_id, &validated.visible_role_ids).await?;
    let event = load_schedule_event(&state, event_id).await?;

    Ok((StatusCode::CREATED, Json(event_to_response(&viewer, &event))))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/schedule/events/{event_id}", role = Member, rate_limit = "internal", summary = "Update schedule event")]
async fn update_schedule_event(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<ScheduleEventPayload>,
) -> Result<Json<ScheduleTimelineEvent>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_schedule_access(&viewer)?;
    let existing = load_schedule_event(&state, event_id).await?;
    ensure_can_edit_event(&viewer, &existing)?;
    let validated = validate_event_payload(&state, &viewer, payload).await?;

    sqlx::query(
        r#"
        UPDATE schedule_events
        SET title = $2,
            description = $3,
            start_at = $4,
            end_at = $5,
            visibility_mode = $6,
            auto_notify_enabled = $7,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(event_id)
    .bind(&validated.title)
    .bind(&validated.description)
    .bind(validated.start_at)
    .bind(validated.end_at)
    .bind(&validated.visibility_mode)
    .bind(validated.auto_notify_enabled)
    .execute(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    replace_visible_roles(&state, event_id, &validated.visible_role_ids).await?;
    let event = load_schedule_event(&state, event_id).await?;

    Ok(Json(event_to_response(&viewer, &event)))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/schedule/events/{event_id}", role = Member, rate_limit = "internal", summary = "Delete schedule event")]
async fn delete_schedule_event(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(event_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_schedule_access(&viewer)?;
    let existing = load_schedule_event(&state, event_id).await?;
    ensure_can_edit_event(&viewer, &existing)?;

    sqlx::query("DELETE FROM schedule_events WHERE id = $1")
        .bind(event_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/schedule/roles", role = Member, rate_limit = "internal", summary = "Create managed schedule role")]
async fn create_schedule_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(payload): Json<ScheduleRolePayload>,
) -> Result<(StatusCode, Json<ScheduleManagedRoleResponse>), ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_roles, &viewer.user, "staff")?;
    let payload = validate_role_payload(payload)?;

    let row = sqlx::query_as::<_, ScheduleManagedRoleRow>(
        r#"
        INSERT INTO schedule_managed_roles (
            discord_role_id, display_name, description, can_manage_roles, can_manage_events,
            can_manage_templates, can_manage_notifications, can_view_restricted_events
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, discord_role_id, display_name, description, can_manage_roles,
                  can_manage_events, can_manage_templates, can_manage_notifications,
                  can_view_restricted_events, created_at, updated_at
        "#,
    )
    .bind(&payload.discord_role_id)
    .bind(&payload.display_name)
    .bind(&payload.description)
    .bind(payload.can_manage_roles)
    .bind(payload.can_manage_events)
    .bind(payload.can_manage_templates)
    .bind(payload.can_manage_notifications)
    .bind(payload.can_view_restricted_events)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((StatusCode::CREATED, Json(role_to_response(row))))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/schedule/roles/{role_id}", role = Member, rate_limit = "internal", summary = "Update managed schedule role")]
async fn update_schedule_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(role_id): Path<Uuid>,
    Json(payload): Json<ScheduleRolePayload>,
) -> Result<Json<ScheduleManagedRoleResponse>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_roles, &viewer.user, "staff")?;
    let payload = validate_role_payload(payload)?;

    let row = sqlx::query_as::<_, ScheduleManagedRoleRow>(
        r#"
        UPDATE schedule_managed_roles
        SET discord_role_id = $2,
            display_name = $3,
            description = $4,
            can_manage_roles = $5,
            can_manage_events = $6,
            can_manage_templates = $7,
            can_manage_notifications = $8,
            can_view_restricted_events = $9,
            updated_at = now()
        WHERE id = $1
        RETURNING id, discord_role_id, display_name, description, can_manage_roles,
                  can_manage_events, can_manage_templates, can_manage_notifications,
                  can_view_restricted_events, created_at, updated_at
        "#,
    )
    .bind(role_id)
    .bind(&payload.discord_role_id)
    .bind(&payload.display_name)
    .bind(&payload.description)
    .bind(payload.can_manage_roles)
    .bind(payload.can_manage_events)
    .bind(payload.can_manage_templates)
    .bind(payload.can_manage_notifications)
    .bind(payload.can_view_restricted_events)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or(ApiError::ValidationError(HashMap::from([(
        "role_id".to_owned(),
        "Managed role was not found".to_owned(),
    )])))?;

    Ok(Json(role_to_response(row)))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/schedule/roles/{role_id}", role = Member, rate_limit = "internal", summary = "Delete managed schedule role")]
async fn delete_schedule_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(role_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_roles, &viewer.user, "staff")?;

    sqlx::query("DELETE FROM schedule_managed_roles WHERE id = $1")
        .bind(role_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/schedule/templates", role = Member, rate_limit = "internal", summary = "Create schedule template")]
async fn create_schedule_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(payload): Json<ScheduleTemplatePayload>,
) -> Result<(StatusCode, Json<ScheduleTemplateResponse>), ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_templates, &viewer.user, "staff")?;
    let payload = validate_template_payload(payload)?;
    maybe_clear_default_template(&state, payload.is_default).await?;

    let row = sqlx::query_as::<_, ScheduleTemplateRow>(
        r#"
        INSERT INTO schedule_templates (name, title, description, is_default)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, title, description, is_default, created_at, updated_at
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.is_default)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((StatusCode::CREATED, Json(template_to_response(row))))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/schedule/templates/{template_id}", role = Member, rate_limit = "internal", summary = "Update schedule template")]
async fn update_schedule_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(template_id): Path<Uuid>,
    Json(payload): Json<ScheduleTemplatePayload>,
) -> Result<Json<ScheduleTemplateResponse>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_templates, &viewer.user, "staff")?;
    let payload = validate_template_payload(payload)?;
    maybe_clear_default_template(&state, payload.is_default).await?;

    let row = sqlx::query_as::<_, ScheduleTemplateRow>(
        r#"
        UPDATE schedule_templates
        SET name = $2,
            title = $3,
            description = $4,
            is_default = $5,
            updated_at = now()
        WHERE id = $1
        RETURNING id, name, title, description, is_default, created_at, updated_at
        "#,
    )
    .bind(template_id)
    .bind(&payload.name)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.is_default)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or(ApiError::ValidationError(HashMap::from([(
        "template_id".to_owned(),
        "Template was not found".to_owned(),
    )])))?;

    Ok(Json(template_to_response(row)))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/schedule/templates/{template_id}", role = Member, rate_limit = "internal", summary = "Delete schedule template")]
async fn delete_schedule_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(template_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_templates, &viewer.user, "staff")?;

    sqlx::query("DELETE FROM schedule_templates WHERE id = $1")
        .bind(template_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[vrc_macros::handler(method = PUT, path = "/api/v1/internal/schedule/notifications/webhook", role = Member, rate_limit = "internal", summary = "Update schedule webhook")]
async fn update_schedule_webhook(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(payload): Json<ScheduleNotificationWebhookPayload>,
) -> Result<Json<ScheduleNotificationStateResponse>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_notifications, &viewer.user, "staff")?;
    let webhook_url = validate_webhook_url(&payload.webhook_url)?;

    sqlx::query(
        r#"
        INSERT INTO schedule_notification_settings (id, webhook_url, updated_by_user_id)
        VALUES (TRUE, $1, $2)
        ON CONFLICT (id) DO UPDATE
        SET webhook_url = EXCLUDED.webhook_url,
            updated_by_user_id = EXCLUDED.updated_by_user_id,
            updated_at = now()
        "#,
    )
    .bind(webhook_url)
    .bind(viewer.user.id)
    .execute(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(Json(load_notification_state(&state).await?))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/schedule/notifications/webhook", role = Member, rate_limit = "internal", summary = "Delete schedule webhook")]
async fn delete_schedule_webhook(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
) -> Result<StatusCode, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_notifications, &viewer.user, "staff")?;

    sqlx::query("DELETE FROM schedule_notification_settings WHERE id = TRUE")
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/schedule/notifications/rules", role = Member, rate_limit = "internal", summary = "Create schedule notification rule")]
async fn create_schedule_notification_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(payload): Json<ScheduleNotificationRulePayload>,
) -> Result<(StatusCode, Json<ScheduleNotificationRuleResponse>), ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_notifications, &viewer.user, "staff")?;
    let payload = validate_notification_rule_payload(payload)?;

    let row = sqlx::query_as::<_, ScheduleNotificationRuleRow>(
        r#"
        INSERT INTO schedule_notification_rules (
            name, enabled, schedule_type, offset_minutes, time_of_day_minutes,
            window_start_minutes, window_end_minutes, body_template, list_item_template
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, name, enabled, schedule_type, offset_minutes, time_of_day_minutes,
                  window_start_minutes, window_end_minutes, body_template, list_item_template,
                  created_at, updated_at
        "#,
    )
    .bind(&payload.name)
    .bind(payload.enabled)
    .bind(&payload.schedule_type)
    .bind(payload.offset_minutes)
    .bind(payload.time_of_day_minutes)
    .bind(payload.window_start_minutes)
    .bind(payload.window_end_minutes)
    .bind(&payload.body_template)
    .bind(&payload.list_item_template)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((StatusCode::CREATED, Json(notification_rule_to_response(row))))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/schedule/notifications/rules/{rule_id}", role = Member, rate_limit = "internal", summary = "Update schedule notification rule")]
async fn update_schedule_notification_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(rule_id): Path<Uuid>,
    Json(payload): Json<ScheduleNotificationRulePayload>,
) -> Result<Json<ScheduleNotificationRuleResponse>, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_notifications, &viewer.user, "staff")?;
    let payload = validate_notification_rule_payload(payload)?;

    let row = sqlx::query_as::<_, ScheduleNotificationRuleRow>(
        r#"
        UPDATE schedule_notification_rules
        SET name = $2,
            enabled = $3,
            schedule_type = $4,
            offset_minutes = $5,
            time_of_day_minutes = $6,
            window_start_minutes = $7,
            window_end_minutes = $8,
            body_template = $9,
            list_item_template = $10,
            updated_at = now()
        WHERE id = $1
        RETURNING id, name, enabled, schedule_type, offset_minutes, time_of_day_minutes,
                  window_start_minutes, window_end_minutes, body_template, list_item_template,
                  created_at, updated_at
        "#,
    )
    .bind(rule_id)
    .bind(&payload.name)
    .bind(payload.enabled)
    .bind(&payload.schedule_type)
    .bind(payload.offset_minutes)
    .bind(payload.time_of_day_minutes)
    .bind(payload.window_start_minutes)
    .bind(payload.window_end_minutes)
    .bind(&payload.body_template)
    .bind(&payload.list_item_template)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or(ApiError::ValidationError(HashMap::from([(
        "rule_id".to_owned(),
        "Notification rule was not found".to_owned(),
    )])))?;

    Ok(Json(notification_rule_to_response(row)))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/schedule/notifications/rules/{rule_id}", role = Member, rate_limit = "internal", summary = "Delete schedule notification rule")]
async fn delete_schedule_notification_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Path(rule_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let viewer = resolve_schedule_viewer(&state, &auth).await?;
    ensure_permission(viewer.permissions.manage_notifications, &viewer.user, "staff")?;

    sqlx::query("DELETE FROM schedule_notification_rules WHERE id = $1")
        .bind(rule_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

fn schedule_offset() -> Result<FixedOffset, ApiError> {
    FixedOffset::east_opt(JST_OFFSET_SECONDS)
        .ok_or_else(|| ApiError::Internal("Failed to build JST timezone".to_owned()))
}

fn base_schedule_permissions(role: UserRole) -> SchedulePermissionSet {
    match role {
        UserRole::SuperAdmin | UserRole::Admin => SchedulePermissionSet {
            manage_roles: true,
            manage_events: true,
            manage_templates: true,
            manage_notifications: true,
            view_restricted_events: true,
        },
        UserRole::Staff => SchedulePermissionSet {
            manage_roles: false,
            manage_events: true,
            manage_templates: false,
            manage_notifications: false,
            view_restricted_events: true,
        },
        UserRole::Member => SchedulePermissionSet::default(),
    }
}

async fn resolve_schedule_viewer(
    state: &Arc<AppState>,
    auth: &AuthenticatedUser<Member>,
) -> Result<ScheduleViewer, ApiError> {
    let discord_role_ids = resolve_role_snapshot(state, auth).await?;
    let managed_permissions = load_permissions_for_discord_roles(&state.db_pool, &discord_role_ids).await?;
    let permissions = base_schedule_permissions(auth.user.role).merge(managed_permissions);

    Ok(ScheduleViewer {
        user: auth.user.clone(),
        discord_role_ids,
        permissions,
    })
}

async fn resolve_role_snapshot(
    state: &Arc<AppState>,
    auth: &AuthenticatedUser<Member>,
) -> Result<Vec<String>, ApiError> {
    let mut access_token = auth.discord_access_token.clone();
    let mut refresh_token = auth.discord_refresh_token.clone();
    let mut token_expires_at = auth.discord_token_expires_at;

    let discord = ReqwestDiscordClient::new(
        state.http_client.clone(),
        state.config.discord_client_id.clone(),
        state.config.discord_client_secret.expose_secret().to_owned(),
    );

    let needs_refresh = token_expires_at
        .map(|expiry| expiry <= Utc::now() + Duration::seconds(30))
        .unwrap_or(false);

    if needs_refresh
        && let Some(refresh_value) = refresh_token.clone()
        && let Ok(refreshed) = discord.refresh_token(&refresh_value).await
    {
        access_token = Some(refreshed.access_token.clone());
        if let Some(next_refresh) = refreshed.refresh_token {
            refresh_token = Some(next_refresh);
        }
        token_expires_at = Some(Utc::now() + Duration::seconds(refreshed.expires_in));

        sqlx::query(
            r#"
            UPDATE sessions
            SET discord_access_token = $2,
                discord_refresh_token = $3,
                discord_token_expires_at = $4
            WHERE id = $1
            "#,
        )
        .bind(auth.session_id)
        .bind(access_token.as_deref())
        .bind(refresh_token.as_deref())
        .bind(token_expires_at)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    }

    if let Some(access_value) = access_token {
        match discord
            .get_current_guild_member(&access_value, &state.config.discord_guild_id)
            .await
        {
            Ok(member) => {
                let role_ids = dedupe_role_ids(member.roles);
                if role_ids != auth.discord_role_ids {
                    sqlx::query("UPDATE sessions SET discord_role_ids = $2 WHERE id = $1")
                        .bind(auth.session_id)
                        .bind(&role_ids)
                        .execute(&state.db_pool)
                        .await
                        .map_err(|error| ApiError::Internal(error.to_string()))?;
                }
                return Ok(role_ids);
            }
            Err(error) => {
                tracing::warn!(error = %error, "Falling back to stored Discord role snapshot");
            }
        }
    }

    Ok(auth.discord_role_ids.clone())
}

fn dedupe_role_ids(role_ids: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for role_id in role_ids {
        if seen.insert(role_id.clone()) {
            unique.push(role_id);
        }
    }
    unique
}

async fn load_permissions_for_discord_roles(
    pool: &sqlx::PgPool,
    role_ids: &[String],
) -> Result<SchedulePermissionSet, ApiError> {
    if role_ids.is_empty() {
        return Ok(SchedulePermissionSet::default());
    }

    #[derive(sqlx::FromRow)]
    struct PermissionRow {
        manage_roles: bool,
        manage_events: bool,
        manage_templates: bool,
        manage_notifications: bool,
        view_restricted_events: bool,
    }

    let row = sqlx::query_as::<_, PermissionRow>(
        r#"
        SELECT
            COALESCE(BOOL_OR(can_manage_roles), FALSE) AS manage_roles,
            COALESCE(BOOL_OR(can_manage_events), FALSE) AS manage_events,
            COALESCE(BOOL_OR(can_manage_templates), FALSE) AS manage_templates,
            COALESCE(BOOL_OR(can_manage_notifications), FALSE) AS manage_notifications,
            COALESCE(BOOL_OR(can_view_restricted_events), FALSE) AS view_restricted_events
        FROM schedule_managed_roles
        WHERE discord_role_id = ANY($1)
        "#,
    )
    .bind(role_ids)
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(SchedulePermissionSet {
        manage_roles: row.manage_roles,
        manage_events: row.manage_events,
        manage_templates: row.manage_templates,
        manage_notifications: row.manage_notifications,
        view_restricted_events: row.view_restricted_events,
    })
}

fn ensure_schedule_access(viewer: &ScheduleViewer) -> Result<(), ApiError> {
    ensure_permission(
        viewer.permissions.has_any() || viewer.user.role.level() >= UserRole::Staff.level(),
        &viewer.user,
        "staff",
    )
}

fn ensure_permission(allowed: bool, user: &User, required: &'static str) -> Result<(), ApiError> {
    if allowed {
        Ok(())
    } else {
        Err(ApiError::InsufficientRole {
            required,
            actual: user.role.as_str().to_owned(),
        })
    }
}

fn parse_schedule_window(from: Option<&str>, days: Option<u32>) -> Result<(NaiveDate, u32), ApiError> {
    let days = days.unwrap_or(31).clamp(1, MAX_TIMELINE_DAYS);
    let offset = schedule_offset()?;
    let default_from = Utc::now().with_timezone(&offset).date_naive();
    let from_date = match from {
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
            ApiError::ValidationError(HashMap::from([(
                "from".to_owned(),
                "from must be in YYYY-MM-DD format".to_owned(),
            )]))
        })?,
        None => default_from,
    };

    Ok((from_date, days))
}

async fn load_managed_roles(state: &Arc<AppState>) -> Result<Vec<ScheduleManagedRoleRow>, ApiError> {
    sqlx::query_as::<_, ScheduleManagedRoleRow>(
        r#"
        SELECT id, discord_role_id, display_name, description, can_manage_roles,
               can_manage_events, can_manage_templates, can_manage_notifications,
               can_view_restricted_events, created_at, updated_at
        FROM schedule_managed_roles
        ORDER BY display_name ASC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))
}

async fn load_templates(state: &Arc<AppState>) -> Result<Vec<ScheduleTemplateRow>, ApiError> {
    sqlx::query_as::<_, ScheduleTemplateRow>(
        r#"
        SELECT id, name, title, description, is_default, created_at, updated_at
        FROM schedule_templates
        ORDER BY is_default DESC, name ASC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))
}

async fn load_timeline(
    state: &Arc<AppState>,
    viewer: &ScheduleViewer,
    from_date: NaiveDate,
    days: u32,
) -> Result<ScheduleTimelineResponse, ApiError> {
    let offset = schedule_offset()?;
    let from_dt = from_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| ApiError::Internal("Failed to build start of day".to_owned()))?
        .and_local_timezone(offset)
        .single()
        .ok_or_else(|| ApiError::Internal("Failed to localize start date".to_owned()))?
        .with_timezone(&Utc);
    let to_dt = from_dt + Duration::days(i64::from(days));

    let events = sqlx::query_as::<_, ScheduleEventRow>(
        r#"
        SELECT e.id, e.created_by_user_id, e.title, e.description, e.start_at, e.end_at,
               e.visibility_mode, e.auto_notify_enabled,
               COALESCE(array_remove(array_agg(sevr.discord_role_id), NULL), '{}') AS visible_role_ids
        FROM schedule_events e
        LEFT JOIN schedule_event_visible_roles sevr ON sevr.event_id = e.id
        WHERE e.start_at < $2 AND e.end_at > $1
        GROUP BY e.id
        ORDER BY e.start_at ASC, e.end_at ASC
        "#,
    )
    .bind(from_dt)
    .bind(to_dt)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    let mut by_day = BTreeMap::<String, Vec<ScheduleTimelineEvent>>::new();
    for offset_days in 0..days {
        let day = from_date + Duration::days(i64::from(offset_days));
        by_day.insert(day.format("%Y-%m-%d").to_string(), Vec::new());
    }

    for event in events {
        let response = event_to_response(viewer, &event);
        for day_key in overlap_date_keys(&event, from_date, days, offset)? {
            if let Some(items) = by_day.get_mut(&day_key) {
                items.push(response.clone());
            }
        }
    }

    Ok(ScheduleTimelineResponse {
        from: from_date.format("%Y-%m-%d").to_string(),
        days,
        timezone: "Asia/Tokyo",
        timeline: by_day
            .into_iter()
            .map(|(date, events)| ScheduleTimelineDay { date, events })
            .collect(),
    })
}

fn overlap_date_keys(
    event: &ScheduleEventRow,
    from_date: NaiveDate,
    days: u32,
    offset: FixedOffset,
) -> Result<Vec<String>, ApiError> {
    let mut keys = Vec::new();
    let start = event.start_at.with_timezone(&offset);
    let end = event.end_at.with_timezone(&offset);

    for offset_days in 0..days {
        let day = from_date + Duration::days(i64::from(offset_days));
        let day_start = day
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ApiError::Internal("Failed to build day start".to_owned()))?
            .and_local_timezone(offset)
            .single()
            .ok_or_else(|| ApiError::Internal("Failed to localize day start".to_owned()))?;
        let day_end = day_start + Duration::days(1);

        if start < day_end && end > day_start {
            keys.push(day.format("%Y-%m-%d").to_string());
        }
    }

    Ok(keys)
}

fn can_view_event(viewer: &ScheduleViewer, event: &ScheduleEventRow) -> bool {
    if event.visibility_mode == "public" {
        return true;
    }
    if event.created_by_user_id == viewer.user.id || viewer.permissions.manage_events {
        return true;
    }
    if !viewer.permissions.view_restricted_events {
        return false;
    }

    let viewer_roles: HashSet<&str> = viewer.discord_role_ids.iter().map(String::as_str).collect();
    event
        .visible_role_ids
        .iter()
        .any(|role_id| viewer_roles.contains(role_id.as_str()))
}

fn can_edit_event(viewer: &ScheduleViewer, event: &ScheduleEventRow) -> bool {
    event.created_by_user_id == viewer.user.id || viewer.permissions.manage_events
}

fn ensure_can_edit_event(viewer: &ScheduleViewer, event: &ScheduleEventRow) -> Result<(), ApiError> {
    ensure_permission(can_edit_event(viewer, event), &viewer.user, "staff")
}

fn event_to_response(viewer: &ScheduleViewer, event: &ScheduleEventRow) -> ScheduleTimelineEvent {
    let visible = can_view_event(viewer, event);
    ScheduleTimelineEvent {
        id: visible.then_some(event.id),
        display_mode: if visible { "full" } else { "masked" },
        title: visible.then(|| event.title.clone()),
        description: visible.then(|| event.description.clone()),
        start_at: event.start_at,
        end_at: event.end_at,
        visibility_mode: event.visibility_mode.clone(),
        auto_notify_enabled: visible.then_some(event.auto_notify_enabled),
        visible_role_ids: if visible { event.visible_role_ids.clone() } else { Vec::new() },
        created_by_viewer: event.created_by_user_id == viewer.user.id,
        editable: can_edit_event(viewer, event),
    }
}

#[derive(Debug)]
struct ValidatedEventPayload {
    title: String,
    description: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    visibility_mode: String,
    auto_notify_enabled: bool,
    visible_role_ids: Vec<String>,
}

async fn validate_event_payload(
    state: &Arc<AppState>,
    viewer: &ScheduleViewer,
    payload: ScheduleEventPayload,
) -> Result<ValidatedEventPayload, ApiError> {
    if !viewer.permissions.manage_events && viewer.user.role == UserRole::Member {
        return Err(ApiError::InsufficientRole {
            required: "staff",
            actual: viewer.user.role.as_str().to_owned(),
        });
    }

    let title = payload.title.trim().to_owned();
    if title.is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "title".to_owned(),
            "Title is required".to_owned(),
        )])));
    }

    let start_at = DateTime::parse_from_rfc3339(&payload.start_at)
        .map_err(|_| ApiError::ValidationError(HashMap::from([(
            "start_at".to_owned(),
            "start_at must be RFC3339".to_owned(),
        )])))?
        .with_timezone(&Utc);
    let end_at = DateTime::parse_from_rfc3339(&payload.end_at)
        .map_err(|_| ApiError::ValidationError(HashMap::from([(
            "end_at".to_owned(),
            "end_at must be RFC3339".to_owned(),
        )])))?
        .with_timezone(&Utc);

    if end_at <= start_at {
        return Err(ApiError::ValidationError(HashMap::from([(
            "end_at".to_owned(),
            "end_at must be after start_at".to_owned(),
        )])));
    }

    let visibility_mode = match payload.visibility_mode.trim() {
        "" | "public" => "public".to_owned(),
        "restricted" => "restricted".to_owned(),
        _ => {
            return Err(ApiError::ValidationError(HashMap::from([(
                "visibility_mode".to_owned(),
                "visibility_mode must be public or restricted".to_owned(),
            )])));
        }
    };

    let visible_role_ids = dedupe_role_ids(payload.visible_role_ids);
    if visibility_mode == "restricted" {
        ensure_permission(
            viewer.permissions.view_restricted_events || viewer.permissions.manage_events,
            &viewer.user,
            "staff",
        )?;
        if visible_role_ids.is_empty() {
            return Err(ApiError::ValidationError(HashMap::from([(
                "visible_role_ids".to_owned(),
                "At least one role is required for restricted events".to_owned(),
            )])));
        }
        validate_visible_roles(state, &visible_role_ids).await?;
    }

    Ok(ValidatedEventPayload {
        title,
        description: payload.description.trim().to_owned(),
        start_at,
        end_at,
        visibility_mode,
        auto_notify_enabled: payload.auto_notify_enabled,
        visible_role_ids: if payload.visibility_mode == "restricted" {
            visible_role_ids
        } else {
            Vec::new()
        },
    })
}

async fn validate_visible_roles(state: &Arc<AppState>, role_ids: &[String]) -> Result<(), ApiError> {
    let rows = load_managed_roles(state).await?;
    let allowed: HashSet<&str> = rows
        .iter()
        .filter(|role| role.can_view_restricted_events)
        .map(|role| role.discord_role_id.as_str())
        .collect();

    for role_id in role_ids {
        if !allowed.contains(role_id.as_str()) {
            return Err(ApiError::ValidationError(HashMap::from([(
                "visible_role_ids".to_owned(),
                format!("Role {role_id} is not allowed for restricted visibility"),
            )])));
        }
    }
    Ok(())
}

async fn replace_visible_roles(
    state: &Arc<AppState>,
    event_id: Uuid,
    role_ids: &[String],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM schedule_event_visible_roles WHERE event_id = $1")
        .bind(event_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    for role_id in role_ids {
        sqlx::query(
            "INSERT INTO schedule_event_visible_roles (event_id, discord_role_id) VALUES ($1, $2)",
        )
        .bind(event_id)
        .bind(role_id)
        .execute(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    }
    Ok(())
}

async fn load_schedule_event(state: &Arc<AppState>, event_id: Uuid) -> Result<ScheduleEventRow, ApiError> {
    sqlx::query_as::<_, ScheduleEventRow>(
        r#"
        SELECT e.id, e.created_by_user_id, e.title, e.description, e.start_at, e.end_at,
               e.visibility_mode, e.auto_notify_enabled,
               COALESCE(array_remove(array_agg(sevr.discord_role_id), NULL), '{}') AS visible_role_ids
        FROM schedule_events e
        LEFT JOIN schedule_event_visible_roles sevr ON sevr.event_id = e.id
        WHERE e.id = $1
        GROUP BY e.id
        "#,
    )
    .bind(event_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?
    .ok_or(ApiError::ValidationError(HashMap::from([(
        "event_id".to_owned(),
        "Schedule event was not found".to_owned(),
    )])))
}

fn validate_role_payload(payload: ScheduleRolePayload) -> Result<ScheduleRolePayload, ApiError> {
    if payload.discord_role_id.trim().is_empty() || payload.display_name.trim().is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "role".to_owned(),
            "discord_role_id and display_name are required".to_owned(),
        )])));
    }

    Ok(ScheduleRolePayload {
        discord_role_id: payload.discord_role_id.trim().to_owned(),
        display_name: payload.display_name.trim().to_owned(),
        description: payload.description.trim().to_owned(),
        can_manage_roles: payload.can_manage_roles,
        can_manage_events: payload.can_manage_events,
        can_manage_templates: payload.can_manage_templates,
        can_manage_notifications: payload.can_manage_notifications,
        can_view_restricted_events: payload.can_view_restricted_events,
    })
}

fn validate_template_payload(payload: ScheduleTemplatePayload) -> Result<ScheduleTemplatePayload, ApiError> {
    if payload.name.trim().is_empty() || payload.title.trim().is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "template".to_owned(),
            "name and title are required".to_owned(),
        )])));
    }

    Ok(ScheduleTemplatePayload {
        name: payload.name.trim().to_owned(),
        title: payload.title.trim().to_owned(),
        description: payload.description.trim().to_owned(),
        is_default: payload.is_default,
    })
}

async fn maybe_clear_default_template(state: &Arc<AppState>, is_default: bool) -> Result<(), ApiError> {
    if is_default {
        sqlx::query("UPDATE schedule_templates SET is_default = FALSE WHERE is_default = TRUE")
            .execute(&state.db_pool)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
    }
    Ok(())
}

fn validate_webhook_url(raw: &str) -> Result<String, ApiError> {
    let webhook_url = raw.trim();
    let parsed = Url::parse(webhook_url).map_err(|_| ApiError::ValidationError(HashMap::from([(
        "webhook_url".to_owned(),
        "Webhook URL must be a valid absolute URL".to_owned(),
    )])))?;
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() != "https" || !matches!(host, "discord.com" | "ptb.discord.com" | "canary.discord.com") {
        return Err(ApiError::ValidationError(HashMap::from([(
            "webhook_url".to_owned(),
            "Webhook URL must point to a Discord webhook".to_owned(),
        )])));
    }
    Ok(webhook_url.to_owned())
}

#[derive(Debug)]
struct ValidatedNotificationRulePayload {
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

fn validate_notification_rule_payload(
    payload: ScheduleNotificationRulePayload,
) -> Result<ValidatedNotificationRulePayload, ApiError> {
    let name = payload.name.trim().to_owned();
    if name.is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "name".to_owned(),
            "Rule name is required".to_owned(),
        )])));
    }
    let body_template = payload.body_template.trim().to_owned();
    if body_template.is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "body_template".to_owned(),
            "Body template is required".to_owned(),
        )])));
    }

    match payload.schedule_type.as_str() {
        "before_event" => {
            if payload.offset_minutes.is_none() {
                return Err(ApiError::ValidationError(HashMap::from([(
                    "offset_minutes".to_owned(),
                    "offset_minutes is required for before_event rules".to_owned(),
                )])));
            }
        }
        "daily_at" => {
            if payload.time_of_day.is_none() {
                return Err(ApiError::ValidationError(HashMap::from([(
                    "time_of_day".to_owned(),
                    "time_of_day is required for daily_at rules".to_owned(),
                )])));
            }
        }
        _ => {
            return Err(ApiError::ValidationError(HashMap::from([(
                "schedule_type".to_owned(),
                "schedule_type must be before_event or daily_at".to_owned(),
            )])));
        }
    }

    Ok(ValidatedNotificationRulePayload {
        name,
        enabled: payload.enabled,
        schedule_type: payload.schedule_type,
        offset_minutes: payload.offset_minutes,
        time_of_day_minutes: payload.time_of_day.as_deref().map(parse_time_of_day).transpose()?,
        window_start_minutes: payload.window_start_minutes,
        window_end_minutes: payload.window_end_minutes,
        body_template,
        list_item_template: payload.list_item_template.unwrap_or_default().trim().to_owned(),
    })
}

fn parse_time_of_day(value: &str) -> Result<i32, ApiError> {
    let mut parts = value.split(':');
    let hours = parts
        .next()
        .ok_or_else(|| ApiError::ValidationError(HashMap::from([(
            "time_of_day".to_owned(),
            "time_of_day must be HH:MM".to_owned(),
        )])))?
        .parse::<i32>()
        .map_err(|_| ApiError::ValidationError(HashMap::from([(
            "time_of_day".to_owned(),
            "time_of_day must be HH:MM".to_owned(),
        )])))?;
    let minutes = parts
        .next()
        .ok_or_else(|| ApiError::ValidationError(HashMap::from([(
            "time_of_day".to_owned(),
            "time_of_day must be HH:MM".to_owned(),
        )])))?
        .parse::<i32>()
        .map_err(|_| ApiError::ValidationError(HashMap::from([(
            "time_of_day".to_owned(),
            "time_of_day must be HH:MM".to_owned(),
        )])))?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return Err(ApiError::ValidationError(HashMap::from([(
            "time_of_day".to_owned(),
            "time_of_day must be between 00:00 and 23:59".to_owned(),
        )])));
    }
    Ok(hours * 60 + minutes)
}

async fn load_notification_state(
    state: &Arc<AppState>,
) -> Result<ScheduleNotificationStateResponse, ApiError> {
    let setting = sqlx::query_as::<_, ScheduleNotificationSettingRow>(
        "SELECT webhook_url FROM schedule_notification_settings WHERE id = TRUE",
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let rules = sqlx::query_as::<_, ScheduleNotificationRuleRow>(
        r#"
        SELECT id, name, enabled, schedule_type, offset_minutes, time_of_day_minutes,
               window_start_minutes, window_end_minutes, body_template, list_item_template,
               created_at, updated_at
        FROM schedule_notification_rules
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(ScheduleNotificationStateResponse {
        webhook_url: setting.map_or_else(String::new, |row| row.webhook_url),
        rules: rules.into_iter().map(notification_rule_to_response).collect(),
        placeholders: ScheduleNotificationPlaceholderCatalog {
            before_event: BEFORE_EVENT_PLACEHOLDERS.to_vec(),
            daily_body: DAILY_BODY_PLACEHOLDERS.to_vec(),
            daily_item: DAILY_ITEM_PLACEHOLDERS.to_vec(),
        },
    })
}

fn role_to_response(row: ScheduleManagedRoleRow) -> ScheduleManagedRoleResponse {
    ScheduleManagedRoleResponse {
        id: row.id,
        discord_role_id: row.discord_role_id,
        display_name: row.display_name,
        description: row.description,
        can_manage_roles: row.can_manage_roles,
        can_manage_events: row.can_manage_events,
        can_manage_templates: row.can_manage_templates,
        can_manage_notifications: row.can_manage_notifications,
        can_view_restricted_events: row.can_view_restricted_events,
    }
}

fn template_to_response(row: ScheduleTemplateRow) -> ScheduleTemplateResponse {
    ScheduleTemplateResponse {
        id: row.id,
        name: row.name,
        title: row.title,
        description: row.description,
        is_default: row.is_default,
    }
}

fn notification_rule_to_response(row: ScheduleNotificationRuleRow) -> ScheduleNotificationRuleResponse {
    ScheduleNotificationRuleResponse {
        id: row.id,
        name: row.name,
        enabled: row.enabled,
        schedule_type: row.schedule_type,
        offset_minutes: row.offset_minutes,
        time_of_day: row.time_of_day_minutes.map(format_time_of_day),
        window_start_minutes: row.window_start_minutes,
        window_end_minutes: row.window_end_minutes,
        body_template: row.body_template,
        list_item_template: (!row.list_item_template.is_empty()).then_some(row.list_item_template),
    }
}

fn format_time_of_day(total_minutes: i32) -> String {
    let hours = total_minutes.div_euclid(60);
    let minutes = total_minutes.rem_euclid(60);
    format!("{hours:02}:{minutes:02}")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/bootstrap", get(get_schedule_bootstrap))
        .route("/events", post(create_schedule_event))
        .route("/events/{event_id}", patch(update_schedule_event).delete(delete_schedule_event))
        .route("/roles", post(create_schedule_role))
        .route("/roles/{role_id}", patch(update_schedule_role).delete(delete_schedule_role))
        .route("/templates", post(create_schedule_template))
        .route(
            "/templates/{template_id}",
            patch(update_schedule_template).delete(delete_schedule_template),
        )
        .route("/notifications/webhook", put(update_schedule_webhook).delete(delete_schedule_webhook))
        .route("/notifications/rules", post(create_schedule_notification_rule))
        .route(
            "/notifications/rules/{rule_id}",
            patch(update_schedule_notification_rule).delete(delete_schedule_notification_rule),
        )
}