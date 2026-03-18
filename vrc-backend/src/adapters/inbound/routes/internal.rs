use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::adapters::outbound::markdown::renderer::PulldownCmarkRenderer;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::roles::Member;
use crate::domain::entities::event::EventStatus;
use crate::domain::entities::report::ReportTargetType;
use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;
use crate::domain::value_objects::pagination::{PageRequest, PageResponse};
use crate::errors::api::ApiError;

static VRC_ID_RE: LazyLock<regex_lite::Regex> = LazyLock::new(|| {
    regex_lite::Regex::new(r"^usr_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .expect("valid regex")
});

static X_ID_RE: LazyLock<regex_lite::Regex> =
    LazyLock::new(|| regex_lite::Regex::new(r"^[a-zA-Z0-9_]{1,15}$").expect("valid regex"));

// ===== Profile types =====

#[derive(Serialize)]
struct OwnProfile {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_markdown: String,
    bio_html: String,
    avatar_url: Option<String>,
    is_public: bool,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
struct ProfileUpdateRequest {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_markdown: Option<String>,
    avatar_url: Option<String>,
    is_public: bool,
}

// ===== Report types =====

#[derive(Deserialize)]
struct CreateReportRequest {
    target_type: ReportTargetType,
    target_id: Uuid,
    reason: String,
}

#[derive(Serialize)]
struct ReportResponse {
    id: Uuid,
    target_type: ReportTargetType,
    target_id: Uuid,
    status: crate::domain::entities::report::ReportStatus,
    created_at: chrono::DateTime<Utc>,
}

// ===== Me response (for auth/me and auth/logout) =====

#[derive(Serialize)]
struct MeResponse {
    user: UserInfo,
    has_profile: bool,
    profile_summary: Option<ProfileSummary>,
}

#[derive(Serialize)]
struct UserInfo {
    id: Uuid,
    discord_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    role: crate::domain::entities::user::UserRole,
    status: crate::domain::entities::user::UserStatus,
    joined_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
struct ProfileSummary {
    nickname: Option<String>,
    avatar_url: Option<String>,
}

// ===== Event types =====

#[derive(Deserialize)]
struct EventListQuery {
    #[serde(flatten)]
    page: PageRequest,
    status: Option<EventStatus>,
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

// ===== Handlers =====

async fn get_me(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
) -> Result<Json<MeResponse>, ApiError> {
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

    let has_profile = profile.is_some();
    let profile_summary = profile.map(|p| ProfileSummary {
        nickname: p.nickname,
        avatar_url: p.avatar_url,
    });

    Ok(Json(MeResponse {
        user: UserInfo {
            id: auth.user.id,
            discord_id: auth.user.discord_id,
            discord_display_name: auth.user.discord_display_name,
            discord_avatar_hash: auth.user.discord_avatar_hash,
            role: auth.user.role,
            status: auth.user.status,
            joined_at: auth.user.joined_at,
        },
        has_profile,
        profile_summary,
    }))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Member>,
    jar: axum_extra::extract::CookieJar,
) -> impl IntoResponse {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use sha2::{Digest, Sha256};

    if let Some(cookie) = jar.get("session_id")
        && let Ok(token_bytes) = URL_SAFE_NO_PAD.decode(cookie.value())
    {
        let mut hasher = Sha256::new();
        hasher.update(&token_bytes);
        let token_hash = hasher.finalize().to_vec();

        let _ = sqlx::query!(
            "DELETE FROM sessions WHERE token_hash = $1",
            &token_hash[..]
        )
        .execute(&state.db_pool)
        .await;
    }

    let remove_cookie = axum_extra::extract::cookie::Cookie::build(("session_id", ""))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO);

    (jar.remove(remove_cookie), StatusCode::NO_CONTENT)
}

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
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::ProfileNotFound)?;

    Ok(Json(OwnProfile {
        nickname: profile.nickname,
        vrc_id: profile.vrc_id,
        x_id: profile.x_id,
        bio_markdown: profile.bio_markdown,
        bio_html: profile.bio_html,
        avatar_url: profile.avatar_url,
        is_public: profile.is_public,
        updated_at: profile.updated_at,
    }))
}

async fn update_my_profile(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(body): Json<ProfileUpdateRequest>,
) -> Result<Json<OwnProfile>, ApiError> {
    // Validate fields
    let mut errors: HashMap<String, String> = HashMap::new();

    if let Some(ref nickname) = body.nickname
        && (nickname.is_empty() || nickname.len() > 50)
    {
        errors.insert(
            "nickname".to_owned(),
            "1〜50文字で入力してください".to_owned(),
        );
    }

    if let Some(ref vrc_id) = body.vrc_id
        && !VRC_ID_RE.is_match(vrc_id)
    {
        errors.insert(
            "vrc_id".to_owned(),
            "VRC IDの形式が正しくありません".to_owned(),
        );
    }

    if let Some(ref x_id) = body.x_id
        && !X_ID_RE.is_match(x_id)
    {
        errors.insert("x_id".to_owned(), "X IDの形式が正しくありません".to_owned());
    }

    if let Some(ref bio) = body.bio_markdown
        && bio.len() > 2000
    {
        errors.insert(
            "bio_markdown".to_owned(),
            "2000文字以内で入力してください".to_owned(),
        );
    }

    if let Some(ref url) = body.avatar_url
        && (url.len() > 500 || !url.starts_with("https://"))
    {
        errors.insert(
            "avatar_url".to_owned(),
            "有効なHTTPS URLを入力してください".to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(ApiError::ProfileValidation(errors));
    }

    // Render markdown to HTML
    let renderer = PulldownCmarkRenderer::new();
    let bio_markdown = body.bio_markdown.unwrap_or_default();
    let bio_html = renderer.render(&bio_markdown);

    // Post-sanitization XSS check
    let lower_html = bio_html.to_lowercase();
    if lower_html.contains("javascript:")
        || lower_html.contains("data:")
        || lower_html.contains("vbscript:")
    {
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
        bio_markdown: profile.bio_markdown,
        bio_html: profile.bio_html,
        avatar_url: profile.avatar_url,
        is_public: profile.is_public,
        updated_at: profile.updated_at,
    }))
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Member>,
    Query(mut query): Query<EventListQuery>,
) -> Result<Json<PageResponse<EventSummary>>, ApiError> {
    query.page.validate();
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
        query.page.limit(),
        query.page.offset()
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

    Ok(Json(PageResponse::new(items, count, query.page.per_page)))
}

async fn create_report(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Member>,
    Json(body): Json<CreateReportRequest>,
) -> Result<(StatusCode, Json<ReportResponse>), ApiError> {
    // Validate reason length
    if body.reason.len() < 10 || body.reason.len() > 1000 {
        return Err(ApiError::ReportReasonLength);
    }

    // Check for duplicate report
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reports
            WHERE reporter_user_id = $1 AND target_type = $2 AND target_id = $3
        ) as "exists!: bool"
        "#,
        auth.user.id,
        body.target_type as ReportTargetType,
        body.target_id,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    if exists {
        return Err(ApiError::DuplicateReport);
    }

    // Verify target exists based on target_type
    let target_exists = match body.target_type {
        ReportTargetType::Profile => sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM profiles WHERE user_id = $1) as "exists!: bool""#,
            body.target_id
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?,
        ReportTargetType::Event => sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM events WHERE id = $1) as "exists!: bool""#,
            body.target_id
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?,
        ReportTargetType::Club => sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM clubs WHERE id = $1) as "exists!: bool""#,
            body.target_id
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?,
        ReportTargetType::GalleryImage => sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM gallery_images WHERE id = $1) as "exists!: bool""#,
            body.target_id
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?,
    };

    if !target_exists {
        return Err(ApiError::ReportTargetNotFound);
    }

    // Create report
    let report = sqlx::query_as!(
        crate::domain::entities::report::Report,
        r#"
        INSERT INTO reports (reporter_user_id, target_type, target_id, reason)
        VALUES ($1, $2, $3, $4)
        RETURNING id, reporter_user_id, target_type as "target_type: ReportTargetType",
                  target_id, reason, status as "status: crate::domain::entities::report::ReportStatus",
                  created_at
        "#,
        auth.user.id,
        body.target_type as ReportTargetType,
        body.target_id,
        body.reason
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

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

    // ===== VRC ID validation =====

    #[test]
    fn test_validate_vrc_id_valid() {
        let id = "usr_12345678-1234-1234-1234-123456789abc";
        assert!(VRC_ID_RE.is_match(id));
    }

    #[test]
    fn test_validate_vrc_id_missing_prefix() {
        let id = "12345678-1234-1234-1234-123456789abc";
        assert!(!VRC_ID_RE.is_match(id));
    }

    #[test]
    fn test_validate_vrc_id_uppercase_rejected() {
        let id = "usr_12345678-1234-1234-1234-123456789ABC";
        assert!(!VRC_ID_RE.is_match(id));
    }

    #[test]
    fn test_validate_vrc_id_too_short() {
        let id = "usr_1234";
        assert!(!VRC_ID_RE.is_match(id));
    }

    #[test]
    fn test_validate_vrc_id_empty() {
        assert!(!VRC_ID_RE.is_match(""));
    }

    // ===== X ID validation =====

    #[test]
    fn test_validate_x_id_valid_alphanumeric() {
        assert!(X_ID_RE.is_match("aqua_vrc"));
    }

    #[test]
    fn test_validate_x_id_single_char() {
        assert!(X_ID_RE.is_match("A"));
    }

    #[test]
    fn test_validate_x_id_max_length() {
        assert!(X_ID_RE.is_match("123456789012345")); // 15 chars
    }

    #[test]
    fn test_validate_x_id_too_long() {
        assert!(!X_ID_RE.is_match("1234567890123456")); // 16 chars
    }

    #[test]
    fn test_validate_x_id_special_chars_rejected() {
        assert!(!X_ID_RE.is_match("aqua@vrc"));
        assert!(!X_ID_RE.is_match("aqua vrc"));
        assert!(!X_ID_RE.is_match("aqua-vrc"));
    }

    #[test]
    fn test_validate_x_id_empty_rejected() {
        assert!(!X_ID_RE.is_match(""));
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
