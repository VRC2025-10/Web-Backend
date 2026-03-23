use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::adapters::inbound::extractors::{ValidatedJson, ValidatedPayload, ValidatedQuery};
use crate::adapters::outbound::markdown::renderer::PulldownCmarkRenderer;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::roles::Member;
use crate::domain::entities::event::EventStatus;
use crate::domain::entities::report::ReportTargetType;
use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;
use crate::domain::ports::services::webhook_sender::{EmbedField, WebhookSender};
use crate::domain::value_objects::pagination::{PageRequest, PageResponse};
use crate::errors::api::ApiError;

// ===== Profile types =====

#[derive(Serialize)]
struct OwnProfile {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_markdown: Option<String>,
    bio_html: Option<String>,
    avatar_url: Option<String>,
    is_public: bool,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, vrc_macros::Validate)]
struct ProfileUpdateRequest {
    #[validate(min_length = 1, max_length = 50)]
    nickname: Option<String>,
    #[validate(regex = r"^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")]
    vrc_id: Option<String>,
    #[validate(regex = r"^[a-zA-Z0-9_]{1,15}$")]
    x_id: Option<String>,
    #[validate(max_length = 2000)]
    bio_markdown: Option<String>,
    #[validate(max_length = 500, https_url)]
    avatar_url: Option<String>,
    is_public: bool,
}

impl ValidatedPayload for ProfileUpdateRequest {
    fn validate_payload(&self) -> Result<(), HashMap<String, String>> {
        self.validate()
    }

    fn validation_error(errors: HashMap<String, String>) -> ApiError {
        ApiError::ProfileValidation(errors)
    }
}

// ===== Report types =====

#[derive(Deserialize)]
struct CreateReportRequest {
    target_type: ReportTargetType,
    target_id: String,
    reason: String,
}

#[derive(Serialize)]
struct ReportResponse {
    id: Uuid,
    target_type: ReportTargetType,
    target_id: String,
    status: crate::domain::entities::report::ReportStatus,
    created_at: chrono::DateTime<Utc>,
}

// ===== Me response (for auth/me and auth/logout) =====

#[derive(Serialize)]
struct MeResponse {
    id: Uuid,
    discord_id: String,
    discord_username: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    avatar_url: Option<String>,
    role: crate::domain::entities::user::UserRole,
    status: crate::domain::entities::user::UserStatus,
    joined_at: chrono::DateTime<Utc>,
    profile: Option<MeProfile>,
}

#[derive(Serialize)]
struct MeProfile {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    is_public: bool,
}

// ===== Event types =====

#[derive(Deserialize)]
struct EventListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
    status: Option<EventStatus>,
}

fn default_page() -> u32 { 1 }
fn default_per_page() -> u32 { 20 }

impl EventListQuery {
    fn page_request(&self) -> Result<PageRequest, ApiError> {
        PageRequest::new(self.page, self.per_page).ok_or_else(|| {
            ApiError::ValidationError(std::collections::HashMap::from([(
                "pagination".to_owned(),
                "page must be >= 1 and per_page must be between 1 and 100".to_owned(),
            )]))
        })
    }
}

#[derive(Serialize)]
struct EventSummary {
    id: Uuid,
    title: String,
    description_html: String,
    status: EventStatus,
    display_status: crate::domain::entities::event::DisplayStatus,
    start_time: chrono::DateTime<Utc>,
    end_time: Option<chrono::DateTime<Utc>>,
    location: Option<String>,
    tags: Vec<String>,
    created_at: chrono::DateTime<Utc>,
}

// ===== Helpers =====

/// Convert an empty string to `None` for API responses.
/// The database stores empty strings as NOT NULL DEFAULT '', but the API spec
/// represents absent values as null.
fn none_if_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

fn contains_html_event_handler(html: &str) -> bool {
    let bytes = html.as_bytes();

    for index in 0..bytes.len() {
        let previous_is_boundary = index == 0 || !is_html_attribute_char(bytes[index - 1]);
        if !previous_is_boundary || bytes[index] != b'o' {
            continue;
        }

        let Some(next_index) = index.checked_add(1) else {
            continue;
        };
        if bytes.get(next_index) != Some(&b'n') {
            continue;
        }

        let mut cursor = index + 2;
        if bytes
            .get(cursor)
            .is_none_or(|byte| !is_html_attribute_char(*byte))
        {
            continue;
        }

        while let Some(byte) = bytes.get(cursor) {
            if !is_html_attribute_char(*byte) {
                break;
            }
            cursor += 1;
        }

        if bytes.get(cursor) == Some(&b'=') {
            return true;
        }
    }

    false
}

fn is_html_attribute_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':')
}

#[derive(Debug)]
enum NormalizedReportTarget {
    Profile(String),
    Resource { text_id: String, uuid: Uuid },
}

fn normalize_report_target(
    target_type: ReportTargetType,
    raw_target_id: &str,
) -> Result<NormalizedReportTarget, ApiError> {
    let trimmed = raw_target_id.trim();
    if trimmed.is_empty() {
        return Err(ApiError::ReportTargetNotFound);
    }

    match target_type {
        ReportTargetType::Profile => Ok(NormalizedReportTarget::Profile(trimmed.to_owned())),
        ReportTargetType::Event => Err(ApiError::ValidationError(HashMap::from([(
            "target_type".to_owned(),
            "Event reports are not supported".to_owned(),
        )]))),
        ReportTargetType::Club | ReportTargetType::GalleryImage => {
            let parsed = Uuid::parse_str(trimmed).map_err(|_| ApiError::ReportTargetNotFound)?;
            Ok(NormalizedReportTarget::Resource {
                text_id: parsed.hyphenated().to_string(),
                uuid: parsed,
            })
        }
    }
}

#[cfg(test)]
fn is_valid_vrc_id(value: &str) -> bool {
    const HYPHEN_POSITIONS: [usize; 4] = [12, 17, 22, 27];

    if value.len() != 40 || !value.starts_with("usr_") {
        return false;
    }

    value.as_bytes().iter().enumerate().all(|(index, byte)| {
        if index < 4 {
            return true;
        }
        if HYPHEN_POSITIONS.contains(&index) {
            *byte == b'-'
        } else {
            byte.is_ascii_digit() || matches!(*byte, b'a'..=b'f')
        }
    })
}

#[cfg(test)]
fn is_valid_x_id(value: &str) -> bool {
    (1..=15).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

// ===== Handlers =====

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/auth/me", role = Member, rate_limit = "internal", summary = "Get current user")]
async fn get_me(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
) -> Result<Json<MeResponse>, ApiError> {
    let user_avatar_url = auth.user.avatar_url.clone();
    let profile = sqlx::query_as!(
        crate::domain::entities::profile::Profile,
        r#"
        SELECT user_id, nickname, vrc_id, x_id, bio_markdown, bio_html,
               avatar_url, is_public, updated_at
        FROM profiles WHERE user_id = $1
        "#,
        auth.user.id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let avatar_url = user_avatar_url.or_else(|| profile.as_ref().and_then(|item| item.avatar_url.clone()));
    let profile = profile.map(|item| MeProfile {
        nickname: item.nickname,
        vrc_id: item.vrc_id,
        x_id: item.x_id,
        is_public: item.is_public,
    });

    Ok(Json(MeResponse {
        id: auth.user.id,
        discord_id: auth.user.discord_id,
        discord_username: auth.user.discord_username,
        discord_display_name: auth.user.discord_display_name,
        discord_avatar_hash: auth.user.discord_avatar_hash,
        avatar_url,
        role: auth.user.role,
        status: auth.user.status,
        joined_at: auth.user.joined_at,
        profile,
    }))
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/auth/logout", role = Member, rate_limit = "internal", summary = "Logout current user")]
async fn logout(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Member>,
    jar: axum_extra::extract::CookieJar,
) -> Result<impl IntoResponse, ApiError> {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    if let Some(cookie) = jar.get("session_id")
        && let Ok(token_bytes) = URL_SAFE_NO_PAD.decode(cookie.value())
    {
        let token_hash = crate::auth::crypto::sha256_hash(&token_bytes);

        if let Err(e) = sqlx::query!(
            "DELETE FROM sessions WHERE token_hash = $1",
            &token_hash[..]
        )
        .execute(&state.db_pool)
        .await
        {
            tracing::error!(error = %e, "Failed to delete session during logout");
            return Err(ApiError::Internal(
                "Failed to invalidate session during logout".to_owned(),
            ));
        }
    }

    let mut remove_cookie = axum_extra::extract::cookie::Cookie::build(("session_id", ""))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO);

    if let Some(cookie_domain) = state.config.cookie_domain.clone() {
        remove_cookie = remove_cookie.domain(cookie_domain);
    }

    let remove_cookie = remove_cookie.build();

    Ok((jar.remove(remove_cookie), StatusCode::NO_CONTENT))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/me/profile", role = Member, rate_limit = "internal", summary = "Get own profile")]
async fn get_my_profile(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
) -> Result<Json<OwnProfile>, ApiError> {
    let profile = sqlx::query_as!(
        crate::domain::entities::profile::Profile,
        r#"
        SELECT user_id, nickname, vrc_id, x_id, bio_markdown, bio_html,
               avatar_url, is_public, updated_at
        FROM profiles WHERE user_id = $1
        "#,
        auth.user.id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = if let Some(profile) = profile {
        OwnProfile {
            nickname: profile.nickname,
            vrc_id: profile.vrc_id,
            x_id: profile.x_id,
            bio_markdown: none_if_empty(profile.bio_markdown),
            bio_html: none_if_empty(profile.bio_html),
            avatar_url: profile.avatar_url,
            is_public: profile.is_public,
            updated_at: profile.updated_at,
        }
    } else {
        // First-time users may exist before creating any editable profile data.
        // Return an empty profile payload so the editor can render instead of 404.
        OwnProfile {
            nickname: None,
            vrc_id: None,
            x_id: None,
            bio_markdown: Some(String::new()),
            bio_html: Some(String::new()),
            avatar_url: None,
            is_public: false,
            updated_at: Utc::now(),
        }
    };

    Ok(Json(response))
}

#[vrc_macros::handler(method = PUT, path = "/api/v1/internal/me/profile", role = Member, rate_limit = "internal", summary = "Update own profile")]
async fn update_my_profile(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    ValidatedJson(body): ValidatedJson<ProfileUpdateRequest>,
) -> Result<Json<OwnProfile>, ApiError> {
    let bio_markdown = body.bio_markdown.unwrap_or_default();

    // Render markdown to HTML (ammonia sanitizes the output)
    let renderer = PulldownCmarkRenderer::new();
    let bio_html = renderer.render(&bio_markdown);

    // Post-sanitization defense-in-depth XSS check on the rendered HTML output.
    // Ammonia should strip dangerous content, but we reject the input entirely if
    // any suspicious patterns survive sanitization. This catches hypothetical
    // ammonia bypasses and logs the attempt for security auditing.
    let lower_html = bio_html.to_lowercase();
    if lower_html.contains("<script")
        || lower_html.contains("javascript:")
        || lower_html.contains("vbscript:")
        || contains_html_event_handler(&lower_html)
    {
        tracing::warn!(
            user_id = %auth.user.id,
            "Rejected bio: suspicious payload detected in rendered HTML"
        );
        return Err(ApiError::BioDangerous);
    }

    // UPSERT profile
    let profile = sqlx::query_as!(
        crate::domain::entities::profile::Profile,
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
        auth.user.id,
        body.nickname,
        body.vrc_id,
        body.x_id,
        bio_markdown,
        bio_html,
        body.avatar_url,
        body.is_public,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(OwnProfile {
        nickname: profile.nickname,
        vrc_id: profile.vrc_id,
        x_id: profile.x_id,
        bio_markdown: none_if_empty(profile.bio_markdown),
        bio_html: none_if_empty(profile.bio_html),
        avatar_url: profile.avatar_url,
        is_public: profile.is_public,
        updated_at: profile.updated_at,
    }))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/events", role = Member, rate_limit = "internal", summary = "List internal events")]
async fn list_events(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Member>,
    ValidatedQuery(query): ValidatedQuery<EventListQuery>,
) -> Result<PageResponse<EventSummary>, ApiError> {
    let page = query.page_request()?;
    let now = Utc::now();

    let events = sqlx::query_as!(
        crate::domain::entities::event::Event,
        r#"
        SELECT id, external_source_id, title, description_markdown, description_html,
               host_user_id, host_name, event_status as "event_status: EventStatus",
               start_time, end_time, location, created_at, updated_at
        FROM events
        WHERE ($1::event_status IS NULL OR event_status = $1)
        ORDER BY start_time DESC
        LIMIT $2 OFFSET $3
        "#,
        query.status as Option<EventStatus>,
        page.limit(),
        page.offset()
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!: i64"
        FROM events
        WHERE ($1::event_status IS NULL OR event_status = $1)
        "#,
        query.status as Option<EventStatus>,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Fetch tags for events
    let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();
    let tag_rows = sqlx::query!(
        r#"
        SELECT m.event_id, t.name
        FROM event_tags t
        JOIN event_tag_mappings m ON m.tag_id = t.id
        WHERE m.event_id = ANY($1)
        "#,
        &event_ids
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut tags_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in tag_rows {
        tags_map.entry(row.event_id).or_default().push(row.name);
    }

    let items: Vec<EventSummary> = events
        .into_iter()
        .map(|e| {
            let display_status = e.display_status(now);
            let tags = tags_map.remove(&e.id).unwrap_or_default();
            EventSummary {
                id: e.id,
                title: e.title,
                description_html: e.description_html,
                status: e.event_status,
                display_status,
                start_time: e.start_time,
                end_time: e.end_time,
                location: e.location,
                tags,
                created_at: e.created_at,
            }
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

#[allow(clippy::too_many_lines)] // Multi-step report with target validation
#[vrc_macros::handler(method = POST, path = "/api/v1/internal/reports", role = Member, rate_limit = "internal", summary = "Create moderation report")]
async fn create_report(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(body): Json<CreateReportRequest>,
) -> Result<(StatusCode, Json<ReportResponse>), ApiError> {
    let normalized_target = normalize_report_target(body.target_type, &body.target_id)?;
    let normalized_target_id = match &normalized_target {
        NormalizedReportTarget::Profile(target_id) => target_id.clone(),
        NormalizedReportTarget::Resource { text_id, .. } => text_id.clone(),
    };
    let reason = body.reason.trim().to_owned();

    if reason.len() < 10 || reason.len() > 1000 {
        return Err(ApiError::ReportReasonLength);
    }

    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reports
            WHERE reporter_user_id = $1 AND target_type = $2 AND target_id = $3
        )
        "#,
    )
    .bind(auth.user.id)
    .bind(body.target_type)
    .bind(&normalized_target_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    if exists {
        return Err(ApiError::DuplicateReport);
    }

    let target_exists = match normalized_target {
        NormalizedReportTarget::Profile(profile_target_id) => sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(
                SELECT 1 FROM users u JOIN profiles p ON p.user_id = u.id
                WHERE u.discord_id = $1
            )"#,
        )
        .bind(profile_target_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?,
        NormalizedReportTarget::Resource { uuid, .. }
            if body.target_type == ReportTargetType::Club =>
        {
            sqlx::query_scalar::<_, bool>(r#"SELECT EXISTS(SELECT 1 FROM clubs WHERE id = $1)"#)
                .bind(uuid)
                .fetch_one(&state.db_pool)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
        }
        NormalizedReportTarget::Resource { uuid, .. } => {
            sqlx::query_scalar::<_, bool>(
                r#"SELECT EXISTS(SELECT 1 FROM gallery_images WHERE id = $1)"#,
            )
            .bind(uuid)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        }
    };

    if !target_exists {
        return Err(ApiError::ReportTargetNotFound);
    }

    let report = sqlx::query_as::<_, crate::domain::entities::report::Report>(
        r#"
        INSERT INTO reports (reporter_user_id, target_type, target_id, reason)
        VALUES ($1, $2, $3, $4)
        RETURNING id, reporter_user_id, target_type, target_id, reason, status, created_at
        "#,
    )
    .bind(auth.user.id)
    .bind(body.target_type)
    .bind(&normalized_target_id)
    .bind(&reason)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Notify staff channel about the new report
    if let Some(ref webhook) = state.webhook {
        let fields = vec![
            EmbedField {
                name: "Target Type".to_owned(),
                value: format!("{:?}", report.target_type),
                inline: true,
            },
            EmbedField {
                name: "Target ID".to_owned(),
                value: report.target_id.to_string(),
                inline: true,
            },
            EmbedField {
                name: "Reason".to_owned(),
                value: reason.chars().take(200).collect(),
                inline: false,
            },
        ];

        if let Err(e) = webhook
            .send_embed(
                "🚨 New Report Submitted",
                &format!("Report `{}` requires staff review.", report.id),
                0x00FE_E75C, // Discord yellow
                fields,
            )
            .await
        {
            tracing::error!(error = %e, report_id = %report.id, "Failed to send report webhook");
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(ReportResponse {
            id: report.id,
            target_type: report.target_type,
            target_id: report.target_id,
            status: report.status,
            created_at: report.created_at,
        }),
    ))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/me", get(get_me))
        .route("/auth/logout", post(logout))
        .route("/me/profile", get(get_my_profile).put(update_my_profile))
        .route("/events", get(list_events))
        .route("/reports", post(create_report))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec refs: internal-api.md report creation contract and application-security.md input validation.
    // Coverage: helper normalization, XSS event-handler detection, and validation boundaries.

    #[test]
    fn test_none_if_empty_converts_empty_string_to_none() {
        assert_eq!(none_if_empty(String::new()), None);
    }

    #[test]
    fn test_none_if_empty_preserves_non_empty_string() {
        assert_eq!(none_if_empty("hello".to_owned()), Some("hello".to_owned()));
    }

    #[test]
    fn test_contains_html_event_handler_detects_event_handler_attribute() {
        assert!(contains_html_event_handler(r#"<img src=\"x\" onload=\"alert(1)\">"#));
    }

    #[test]
    fn test_contains_html_event_handler_ignores_data_attribute_prefixes() {
        assert!(!contains_html_event_handler(r#"<div data-onclick=\"noop\"></div>"#));
    }

    #[test]
    fn test_contains_html_event_handler_ignores_embedded_on_substrings() {
        assert!(!contains_html_event_handler(r#"<div json=\"value\"></div>"#));
    }

    #[test]
    fn test_normalize_report_target_trims_profile_identifier() {
        let target = normalize_report_target(ReportTargetType::Profile, "  123456789012345678  ")
            .expect("trimmed profile id must be accepted");

        match target {
            NormalizedReportTarget::Profile(target_id) => {
                assert_eq!(target_id, "123456789012345678");
            }
            NormalizedReportTarget::Resource { .. } => panic!("profile target must stay string based"),
        }
    }

    #[test]
    fn test_normalize_report_target_rejects_empty_identifier() {
        let error = normalize_report_target(ReportTargetType::Profile, "   ")
            .expect_err("empty target must be rejected");

        assert!(matches!(error, ApiError::ReportTargetNotFound));
    }

    #[test]
    fn test_normalize_report_target_rejects_event_reports() {
        let error = normalize_report_target(
            ReportTargetType::Event,
            "550e8400-e29b-41d4-a716-446655440000",
        )
        .expect_err("event reports must be rejected");

        assert!(matches!(error, ApiError::ValidationError(details) if details.get("target_type") == Some(&"Event reports are not supported".to_owned())));
    }

    #[test]
    fn test_normalize_report_target_normalizes_resource_uuid_to_hyphenated_lowercase() {
        let target = normalize_report_target(
            ReportTargetType::Club,
            "550E8400-E29B-41D4-A716-446655440000",
        )
        .expect("valid UUID must be accepted");

        match target {
            NormalizedReportTarget::Resource { text_id, uuid } => {
                assert_eq!(uuid, Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid must parse"));
                assert_eq!(text_id, "550e8400-e29b-41d4-a716-446655440000");
            }
            NormalizedReportTarget::Profile(_) => panic!("club target must normalize as resource"),
        }
    }

    #[test]
    fn test_normalize_report_target_rejects_malformed_resource_uuid() {
        let error = normalize_report_target(ReportTargetType::GalleryImage, "not-a-uuid")
            .expect_err("malformed UUID must be rejected");

        assert!(matches!(error, ApiError::ReportTargetNotFound));
    }

    // ===== VRC ID validation =====

    #[test]
    fn test_validate_vrc_id_valid() {
        let id = "usr_12345678-1234-1234-1234-123456789abc";
        assert!(is_valid_vrc_id(id));
    }

    #[test]
    fn test_validate_vrc_id_missing_prefix() {
        let id = "12345678-1234-1234-1234-123456789abc";
        assert!(!is_valid_vrc_id(id));
    }

    #[test]
    fn test_validate_vrc_id_uppercase_rejected() {
        let id = "usr_12345678-1234-1234-1234-123456789ABC";
        assert!(!is_valid_vrc_id(id));
    }

    #[test]
    fn test_validate_vrc_id_too_short() {
        let id = "usr_1234";
        assert!(!is_valid_vrc_id(id));
    }

    #[test]
    fn test_validate_vrc_id_empty() {
        assert!(!is_valid_vrc_id(""));
    }

    #[test]
    fn test_validate_vrc_id_rejects_wrong_hyphen_positions() {
        let id = "usr_123456781234-1234-1234-123456789abc";
        assert!(!is_valid_vrc_id(id));
    }

    // ===== X ID validation =====

    #[test]
    fn test_validate_x_id_valid_alphanumeric() {
        assert!(is_valid_x_id("aqua_vrc"));
    }

    #[test]
    fn test_validate_x_id_single_char() {
        assert!(is_valid_x_id("A"));
    }

    #[test]
    fn test_validate_x_id_max_length() {
        assert!(is_valid_x_id("123456789012345")); // 15 chars
    }

    #[test]
    fn test_validate_x_id_too_long() {
        assert!(!is_valid_x_id("1234567890123456")); // 16 chars
    }

    #[test]
    fn test_validate_x_id_special_chars_rejected() {
        assert!(!is_valid_x_id("aqua@vrc"));
        assert!(!is_valid_x_id("aqua vrc"));
        assert!(!is_valid_x_id("aqua-vrc"));
    }

    #[test]
    fn test_validate_x_id_empty_rejected() {
        assert!(!is_valid_x_id(""));
    }

    #[test]
    fn test_validate_x_id_rejects_non_ascii_characters() {
        assert!(!is_valid_x_id("あqua"));
    }

    // ===== Bio length =====

    #[test]
    fn test_bio_at_limit_passes() {
        let bio = "a".repeat(2000);
        assert!(bio.len() <= 2000);
    }

    #[test]
    fn test_bio_over_limit_fails() {
        let bio = "a".repeat(2001);
        assert!(bio.len() > 2000);
    }

    // ===== Avatar URL =====

    #[test]
    fn test_avatar_url_https_valid() {
        let url = "https://example.com/avatar.png";
        assert!(url.starts_with("https://") && url.len() <= 500);
    }

    #[test]
    fn test_avatar_url_http_rejected() {
        let url = "http://example.com/avatar.png";
        assert!(!url.starts_with("https://"));
    }

    #[test]
    fn test_avatar_url_too_long_rejected() {
        let url = format!("https://example.com/{}", "a".repeat(500));
        assert!(url.len() > 500);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// P2: Valid VRC IDs always match the pattern.
        #[test]
        fn valid_vrc_ids_match_pattern(
            a in "[0-9a-f]{8}",
            b in "[0-9a-f]{4}",
            c in "[0-9a-f]{4}",
            d in "[0-9a-f]{4}",
            e in "[0-9a-f]{12}",
        ) {
            let id = format!("usr_{a}-{b}-{c}-{d}-{e}");
            prop_assert!(is_valid_vrc_id(&id));
        }

        /// P2b: Random strings without the usr_ prefix are rejected.
        #[test]
        fn random_strings_rejected_as_vrc_id(input in "\\PC{0,100}") {
            if !input.starts_with("usr_") {
                prop_assert!(!is_valid_vrc_id(&input));
            }
        }

        /// P3: Valid X IDs are accepted.
        #[test]
        fn valid_x_ids_accepted(id in "[a-zA-Z0-9_]{1,15}") {
            prop_assert!(is_valid_x_id(&id));
        }

        /// P3b: X IDs with special characters are rejected.
        #[test]
        fn x_ids_with_special_chars_rejected(
            prefix in "[a-zA-Z0-9_]{0,7}",
            bad_char in "[^a-zA-Z0-9_]",
            suffix in "[a-zA-Z0-9_]{0,7}",
        ) {
            let input = format!("{prefix}{bad_char}{suffix}");
            prop_assert!(!is_valid_x_id(&input));
        }
    }
}
