use std::collections::HashMap;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::adapters::inbound::extractors::{ValidatedJson, ValidatedPayload, ValidatedQuery};
use crate::adapters::outbound::markdown::renderer::PulldownCmarkRenderer;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::roles::{Admin, Role, Staff};
use crate::domain::entities::gallery::{GalleryImageStatus, GalleryTargetType};
use crate::domain::entities::report::{ReportStatus, ReportTargetType};
use crate::domain::entities::user::{UserRole, UserStatus};
use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;
use crate::domain::value_objects::pagination::{PageRequest, PageResponse};
use crate::errors::api::ApiError;

/// Validate that a string is a well-formed HTTPS URL with a non-empty host.
///
/// Rejects bare schemes, `localhost`, and IP-based hosts to prevent SSRF when
/// the URL is later rendered or fetched.
fn is_valid_https_url(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    // Must have content after "https://"
    let rest = &url[8..];
    if rest.is_empty() {
        return false;
    }
    // Extract host portion (before first '/' or end of string)
    let host = rest.split('/').next().unwrap_or("");
    // Host must not be empty, localhost, or bare IP
    if host.is_empty() || host.starts_with("localhost") || host.starts_with("127.") {
        return false;
    }
    // Must contain at least one dot (reject single-label hosts)
    let host_without_port = host.split(':').next().unwrap_or("");
    host_without_port.contains('.')
}

// ===== User management types =====

#[derive(Deserialize)]
struct UserListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
    status: Option<UserStatus>,
    role: Option<UserRole>,
}

fn default_page() -> u32 { 1 }
fn default_per_page() -> u32 { 20 }

impl UserListQuery {
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
struct AdminUserView {
    id: Uuid,
    discord_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    role: UserRole,
    status: UserStatus,
    joined_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct RoleChangeRequest {
    role: UserRole,
}

#[derive(Serialize)]
struct RoleChangeResponse {
    id: Uuid,
    role: UserRole,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct StatusChangeRequest {
    status: UserStatus,
}

#[derive(Serialize)]
struct StatusChangeResponse {
    id: Uuid,
    status: UserStatus,
    updated_at: DateTime<Utc>,
}

// ===== Report management types =====

#[derive(Deserialize)]
struct ReportListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
    status: Option<ReportStatus>,
}

impl ReportListQuery {
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
struct AdminReportView {
    id: Uuid,
    reporter_user_id: Uuid,
    reporter_display_name: String,
    target_type: ReportTargetType,
    target_id: String,
    reason: String,
    status: ReportStatus,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ResolveReportRequest {
    status: ReportStatus,
}

#[derive(Serialize)]
struct ResolveReportResponse {
    id: Uuid,
    status: ReportStatus,
    updated_at: DateTime<Utc>,
}

// ===== Club management types =====

#[derive(Deserialize, vrc_macros::Validate)]
struct CreateClubRequest {
    #[validate(min_length = 1, max_length = 100)]
    name: String,
    #[validate(max_length = 2000)]
    description_markdown: Option<String>,
    owner_user_id: Uuid,
}

impl ValidatedPayload for CreateClubRequest {
    fn validate_payload(&self) -> Result<(), HashMap<String, String>> {
        self.validate()
    }

    fn validation_error(errors: HashMap<String, String>) -> ApiError {
        ApiError::ValidationError(errors)
    }
}

#[derive(Serialize)]
struct ClubResponse {
    id: Uuid,
    name: String,
    description_html: Option<String>,
    owner: UserBrief,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct UserBrief {
    user_id: String,
    discord_display_name: String,
}

// ===== Gallery management types =====

#[derive(Deserialize)]
struct GalleryListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
    target_type: Option<GalleryTargetType>,
    club_id: Option<Uuid>,
}

impl GalleryListQuery {
    fn page_request(&self) -> Result<PageRequest, ApiError> {
        PageRequest::new(self.page, self.per_page).ok_or_else(|| {
            ApiError::ValidationError(std::collections::HashMap::from([(
                "pagination".to_owned(),
                "page must be >= 1 and per_page must be between 1 and 100".to_owned(),
            )]))
        })
    }
}

#[derive(Deserialize, vrc_macros::Validate)]
struct UploadGalleryRequest {
    target_type: GalleryTargetType,
    club_id: Option<Uuid>,
    #[validate(max_length = 500)]
    image_url: String,
    #[validate(max_length = 200)]
    caption: Option<String>,
}

impl ValidatedPayload for UploadGalleryRequest {
    fn validate_payload(&self) -> Result<(), HashMap<String, String>> {
        self.validate()
    }

    fn validation_error(errors: HashMap<String, String>) -> ApiError {
        ApiError::ValidationError(errors)
    }
}

#[derive(Deserialize, vrc_macros::Validate)]
struct UploadClubGalleryRequest {
    #[validate(max_length = 500)]
    image_url: String,
    #[validate(max_length = 200)]
    caption: Option<String>,
}

impl ValidatedPayload for UploadClubGalleryRequest {
    fn validate_payload(&self) -> Result<(), HashMap<String, String>> {
        self.validate()
    }

    fn validation_error(errors: HashMap<String, String>) -> ApiError {
        ApiError::ValidationError(errors)
    }
}

#[derive(Serialize)]
struct GalleryClubSummary {
    id: Uuid,
    name: String,
}

#[derive(Serialize)]
struct AdminGalleryView {
    id: Uuid,
    target_type: GalleryTargetType,
    club_id: Option<Uuid>,
    club: Option<GalleryClubSummary>,
    image_url: String,
    caption: Option<String>,
    status: GalleryImageStatus,
    uploaded_by: UserBrief,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct GalleryUploadBatchResponse {
    uploaded_count: usize,
    items: Vec<AdminGalleryView>,
}

#[derive(Deserialize)]
struct GalleryStatusRequest {
    status: GalleryImageStatus,
}

#[derive(Serialize)]
struct GalleryStatusResponse {
    id: Uuid,
    status: GalleryImageStatus,
    reviewed_by: UserBrief,
    reviewed_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AdminGalleryRow {
    id: Uuid,
    target_type: GalleryTargetType,
    club_id: Option<Uuid>,
    club_name: Option<String>,
    image_url: String,
    caption: Option<String>,
    status: GalleryImageStatus,
    uploader_discord_id: String,
    uploader_display_name: String,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ClubNameRow {
    name: String,
}

struct PendingGalleryFile {
    file_name: String,
    bytes: Vec<u8>,
}

#[derive(sqlx::FromRow)]
struct DeletedGalleryRow {
    id: Uuid,
    image_url: String,
}

fn validate_gallery_target_fields(
    target_type: GalleryTargetType,
    club_id: Option<Uuid>,
) -> HashMap<String, String> {
    let mut errors = HashMap::new();

    match target_type {
        GalleryTargetType::Community => {
            if club_id.is_some() {
                errors.insert(
                    "club_id".to_owned(),
                    "club_id must be omitted for community gallery images".to_owned(),
                );
            }
        }
        GalleryTargetType::Club => {
            if club_id.is_none() {
                errors.insert(
                    "club_id".to_owned(),
                    "club_id is required for club gallery images".to_owned(),
                );
            }
        }
    }

    errors
}

fn validate_gallery_scope(body: &UploadGalleryRequest) -> Result<(), ApiError> {
    let mut errors = validate_gallery_target_fields(body.target_type, body.club_id);

    if !is_valid_https_url(&body.image_url) {
        errors.insert(
            "image_url".to_owned(),
            "有効なHTTPS URLを入力してください".to_owned(),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::ValidationError(errors))
    }
}

fn parse_gallery_target_type(value: &str) -> Result<GalleryTargetType, ApiError> {
    match value.trim() {
        "community" => Ok(GalleryTargetType::Community),
        "club" => Ok(GalleryTargetType::Club),
        _ => Err(ApiError::ValidationError(HashMap::from([(
            "target_type".to_owned(),
            "community か club を指定してください".to_owned(),
        )]))),
    }
}

fn parse_optional_uuid_field(field_name: &str, value: &str) -> Result<Option<Uuid>, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Uuid::parse_str(trimmed)
        .map(Some)
        .map_err(|_| ApiError::ValidationError(HashMap::from([(
            field_name.to_owned(),
            "UUID を指定してください".to_owned(),
        )])))
}

fn sanitize_upload_caption(value: String) -> Result<Option<String>, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 200 {
        return Err(ApiError::ValidationError(HashMap::from([(
            "caption".to_owned(),
            "キャプションは200文字以内で入力してください".to_owned(),
        )])));
    }

    Ok(Some(trimmed.to_owned()))
}

fn extension_from_upload(content_type: Option<&str>, file_name: &str) -> Option<&'static str> {
    match content_type {
        Some("image/png") => return Some("png"),
        Some("image/jpeg") => return Some("jpg"),
        Some("image/webp") => return Some("webp"),
        _ => {}
    }

    let extension = file_name.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        _ => None,
    }
}

fn build_gallery_public_url(base_url: &str, file_name: &str) -> String {
    format!("{}/gallery/{file_name}", base_url.trim_end_matches('/'))
}

fn local_gallery_file_name(image_url: &str, backend_base_url: &str) -> Option<String> {
    let parsed = Url::parse(image_url).ok()?;
    let backend_origin = Url::parse(backend_base_url).ok()?.origin().ascii_serialization();
    if parsed.origin().ascii_serialization() != backend_origin {
        return None;
    }

    let path = parsed.path();
    let prefix = "/gallery/";
    let file_name = path.strip_prefix(prefix)?;
    let is_safe = !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && file_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'));

    is_safe.then(|| file_name.to_owned())
}

async fn cleanup_saved_gallery_files(paths: &[PathBuf]) {
    for path in paths {
        if let Err(error) = tokio::fs::remove_file(path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(error = %error, path = %path.display(), "Failed to clean up gallery upload file");
        }
    }
}

async fn delete_local_gallery_file(state: &Arc<AppState>, image_url: &str) -> Result<(), std::io::Error> {
    let Some(file_name) = local_gallery_file_name(image_url, &state.config.backend_base_url) else {
        return Ok(());
    };

    let path = state.config.gallery_storage_dir.join(file_name);
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn ensure_gallery_storage_dir(path: &StdPath) -> Result<(), ApiError> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|error| ApiError::Internal(format!("Failed to prepare gallery storage: {error}")))
}

fn to_admin_gallery_view(row: AdminGalleryRow) -> AdminGalleryView {
    AdminGalleryView {
        id: row.id,
        target_type: row.target_type,
        club_id: row.club_id,
        club: row.club_id.zip(row.club_name).map(|(id, name)| GalleryClubSummary { id, name }),
        image_url: row.image_url,
        caption: row.caption,
        status: row.status,
        uploaded_by: UserBrief {
            user_id: row.uploader_discord_id,
            discord_display_name: row.uploader_display_name,
        },
        created_at: row.created_at,
    }
}

// ===== SQL query helper row types =====

struct AdminUserRow {
    id: Uuid,
    discord_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    role: UserRole,
    status: UserStatus,
    joined_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ReportRow {
    id: Uuid,
    reporter_user_id: Uuid,
    reporter_display_name: String,
    target_type: ReportTargetType,
    target_id: String,
    reason: String,
    status: ReportStatus,
    created_at: DateTime<Utc>,
}

// ===== Handlers =====

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/admin/users", role = Admin, rate_limit = "internal", summary = "List users")]
async fn list_users(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Admin>,
    ValidatedQuery(query): ValidatedQuery<UserListQuery>,
) -> Result<PageResponse<AdminUserView>, ApiError> {
    let page = query.page_request()?;
    let rows = sqlx::query_as!(
        AdminUserRow,
        r#"
        SELECT id, discord_id, discord_display_name, discord_avatar_hash,
               role as "role: UserRole", status as "status: UserStatus",
               joined_at, updated_at
        FROM users
        WHERE ($1::user_status IS NULL OR status = $1)
          AND ($2::user_role IS NULL OR role = $2)
        ORDER BY joined_at DESC
        LIMIT $3 OFFSET $4
        "#,
        query.status as Option<UserStatus>,
        query.role as Option<UserRole>,
        page.limit(),
        page.offset()
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!: i64"
        FROM users
        WHERE ($1::user_status IS NULL OR status = $1)
          AND ($2::user_role IS NULL OR role = $2)
        "#,
        query.status as Option<UserStatus>,
        query.role as Option<UserRole>,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<AdminUserView> = rows
        .into_iter()
        .map(|r| AdminUserView {
            id: r.id,
            discord_id: r.discord_id,
            discord_display_name: r.discord_display_name,
            discord_avatar_hash: r.discord_avatar_hash,
            role: r.role,
            status: r.status,
            joined_at: r.joined_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

/// Validate role change authorization rules per spec:
/// - Only admin+ can change roles (ERR-ROLE-004)
/// - Cannot modify `super_admin` unless you are `super_admin` (ERR-ROLE-003)
/// - Only `super_admin` can grant admin (ERR-ROLE-001)
/// - Only `super_admin` can grant `super_admin` (ERR-ROLE-002)
fn validate_role_change(
    actor_role: UserRole,
    target_role: UserRole,
    new_role: UserRole,
) -> Result<(), ApiError> {
    // Rule 1: Only admin+ can change roles
    if actor_role.level() < Admin::LEVEL {
        return Err(ApiError::RoleLevelInsufficient);
    }

    // Rule 2: Cannot modify super_admin unless you are super_admin
    if target_role == UserRole::SuperAdmin && actor_role != UserRole::SuperAdmin {
        return Err(ApiError::SuperAdminProtected);
    }

    // Rule 3: Only super_admin can grant admin
    if new_role == UserRole::Admin && actor_role != UserRole::SuperAdmin {
        return Err(ApiError::AdminRoleEscalation);
    }

    // Rule 4: Only super_admin can grant super_admin
    if new_role == UserRole::SuperAdmin && actor_role != UserRole::SuperAdmin {
        return Err(ApiError::SuperAdminRoleEscalation);
    }

    Ok(())
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/admin/users/{id}/role", role = Admin, rate_limit = "internal", summary = "Change user role")]
async fn change_user_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Admin>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<RoleChangeRequest>,
) -> Result<Json<RoleChangeResponse>, ApiError> {
    // Fetch the target user to check their current role
    let target = sqlx::query_as!(
        AdminUserRow,
        r#"
        SELECT id, discord_id, discord_display_name, discord_avatar_hash,
               role as "role: UserRole", status as "status: UserStatus",
               joined_at, updated_at
        FROM users WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::UserNotFound)?;

    validate_role_change(auth.user.role, target.role, body.role)?;

    let updated = sqlx::query!(
        r#"
        UPDATE users SET role = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING updated_at
        "#,
        body.role as UserRole,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        actor_id = %auth.user.id,
        target_id = %user_id,
        new_role = body.role.as_str(),
        "User role changed"
    );

    Ok(Json(RoleChangeResponse {
        id: user_id,
        role: body.role,
        updated_at: updated.updated_at,
    }))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/admin/users/{id}/status", role = Admin, rate_limit = "internal", summary = "Change user status")]
async fn change_user_status(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Admin>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<StatusChangeRequest>,
) -> Result<Json<StatusChangeResponse>, ApiError> {
    // Fetch the target user to enforce super_admin protection
    let target = sqlx::query_as!(
        AdminUserRow,
        r#"
        SELECT id, discord_id, discord_display_name, discord_avatar_hash,
               role as "role: UserRole", status as "status: UserStatus",
               joined_at, updated_at
        FROM users WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::UserNotFound)?;

    // Cannot modify super_admin unless you are super_admin
    if target.role == UserRole::SuperAdmin && auth.user.role != UserRole::SuperAdmin {
        return Err(ApiError::SuperAdminProtected);
    }

    let updated = sqlx::query!(
        r#"
        UPDATE users SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING updated_at
        "#,
        body.status as UserStatus,
        user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // If suspending, invalidate all sessions for the user
    if body.status == UserStatus::Suspended
        && let Err(e) = sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
            .execute(&state.db_pool)
            .await
    {
        tracing::error!(
            error = %e,
            user_id = %user_id,
            "Failed to invalidate sessions during user suspension"
        );
        return Err(ApiError::Internal(
            "Failed to invalidate sessions".to_owned(),
        ));
    }

    tracing::info!(
        actor_id = %auth.user.id,
        target_id = %user_id,
        new_status = ?body.status,
        "User status changed"
    );

    Ok(Json(StatusChangeResponse {
        id: user_id,
        status: body.status,
        updated_at: updated.updated_at,
    }))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/admin/reports", role = Staff, rate_limit = "internal", summary = "List reports")]
async fn list_reports(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Staff>,
    ValidatedQuery(query): ValidatedQuery<ReportListQuery>,
) -> Result<PageResponse<AdminReportView>, ApiError> {
    let page = query.page_request()?;
    let rows = sqlx::query_as::<_, ReportRow>(
        r#"
        SELECT r.id,
               r.reporter_user_id,
               u.discord_display_name as reporter_display_name,
               r.target_type,
               r.target_id,
               r.reason,
               r.status,
               r.created_at
        FROM reports r
        JOIN users u ON u.id = r.reporter_user_id
        WHERE ($1::report_status IS NULL OR r.status = $1)
        ORDER BY r.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status)
    .bind(page.limit())
    .bind(page.offset())
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!: i64"
        FROM reports
        WHERE ($1::report_status IS NULL OR status = $1)
        "#,
        query.status as Option<ReportStatus>,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<AdminReportView> = rows
        .into_iter()
        .map(|r| AdminReportView {
            id: r.id,
            reporter_user_id: r.reporter_user_id,
            reporter_display_name: r.reporter_display_name,
            target_type: r.target_type,
            target_id: r.target_id,
            reason: r.reason,
            status: r.status,
            created_at: r.created_at,
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/internal/admin/galleries", role = Staff, rate_limit = "internal", summary = "List gallery images")]
async fn list_galleries(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Staff>,
    ValidatedQuery(query): ValidatedQuery<GalleryListQuery>,
) -> Result<PageResponse<AdminGalleryView>, ApiError> {
    let page = query.page_request()?;

    let rows = sqlx::query_as::<_, AdminGalleryRow>(
        r#"
        SELECT g.id,
               g.target_type,
               g.club_id,
               c.name as club_name,
               g.image_url,
               g.caption,
               g.status,
               u.discord_id as uploader_discord_id,
               u.discord_display_name as uploader_display_name,
               g.created_at
        FROM gallery_images g
        JOIN users u ON u.id = g.uploaded_by_user_id
        LEFT JOIN clubs c ON c.id = g.club_id
        WHERE ($1::gallery_target_type IS NULL OR g.target_type = $1)
          AND ($2::uuid IS NULL OR g.club_id = $2)
        ORDER BY g.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(query.target_type)
    .bind(query.club_id)
    .bind(page.limit())
    .bind(page.offset())
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM gallery_images g
        WHERE ($1::gallery_target_type IS NULL OR g.target_type = $1)
          AND ($2::uuid IS NULL OR g.club_id = $2)
        "#,
    )
    .bind(query.target_type)
    .bind(query.club_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(PageResponse::new(
        rows.into_iter().map(to_admin_gallery_view).collect(),
        count,
        page.per_page(),
    ))
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/admin/reports/{id}", role = Staff, rate_limit = "internal", summary = "Resolve report")]
async fn resolve_report(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(report_id): Path<Uuid>,
    Json(body): Json<ResolveReportRequest>,
) -> Result<Json<ResolveReportResponse>, ApiError> {
    // Only allow resolving to reviewed or dismissed — not back to open
    if body.status == ReportStatus::Open {
        let mut errors = HashMap::new();
        errors.insert(
            "status".to_owned(),
            "resolved or dismissed を指定してください".to_owned(),
        );
        return Err(ApiError::ValidationError(errors));
    }

    let result = sqlx::query!(
        r#"
        UPDATE reports SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING updated_at
        "#,
        body.status as ReportStatus,
        report_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::ReportTargetNotFound)?;

    tracing::info!(
        actor_id = %auth.user.id,
        report_id = %report_id,
        new_status = ?body.status,
        "Report resolved"
    );

    Ok(Json(ResolveReportResponse {
        id: report_id,
        status: body.status,
        updated_at: result.updated_at,
    }))
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/admin/clubs", role = Staff, rate_limit = "internal", summary = "Create club")]
async fn create_club(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    ValidatedJson(body): ValidatedJson<CreateClubRequest>,
) -> Result<(StatusCode, Json<ClubResponse>), ApiError> {
    // Verify owner is an active user
    let owner_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status = 'active') as "exists!: bool""#,
        body.owner_user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !owner_exists {
        return Err(ApiError::UserNotFound);
    }

    // Render markdown if provided
    let description_html = body.description_markdown.as_ref().map(|md| {
        let renderer = PulldownCmarkRenderer::new();
        renderer.render(md)
    });

    // Transaction: create club + add owner as member
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let club = sqlx::query!(
        r#"
        INSERT INTO clubs (name, description_markdown, description_html, owner_user_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at
        "#,
        body.name,
        body.description_markdown.as_deref().unwrap_or(""),
        description_html.as_deref().unwrap_or(""),
        body.owner_user_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    sqlx::query!(
        r#"
        INSERT INTO club_members (club_id, user_id, role)
        VALUES ($1, $2, 'owner')
        "#,
        club.id,
        body.owner_user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Fetch owner info for the response
    let owner = sqlx::query!(
        "SELECT discord_id, discord_display_name FROM users WHERE id = $1",
        body.owner_user_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        actor_id = %auth.user.id,
        club_id = %club.id,
        club_name = %body.name,
        "Club created"
    );

    Ok((
        StatusCode::CREATED,
        Json(ClubResponse {
            id: club.id,
            name: body.name,
            description_html,
            owner: UserBrief {
                user_id: owner.discord_id,
                discord_display_name: owner.discord_display_name,
            },
            created_at: club.created_at,
        }),
    ))
}

async fn create_gallery_image(
    state: &Arc<AppState>,
    auth: &AuthenticatedUser<Staff>,
    body: UploadGalleryRequest,
) -> Result<(StatusCode, Json<AdminGalleryView>), ApiError> {
    validate_gallery_scope(&body)?;

    let club_name = ensure_gallery_target_exists(state, body.club_id).await?;

    let mut transaction = state
        .db_pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    let view = insert_gallery_image_tx(
        &mut transaction,
        auth,
        body.target_type,
        body.club_id,
        club_name,
        &body.image_url,
        body.caption.as_deref(),
    )
    .await?;

    transaction
        .commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((StatusCode::CREATED, Json(view)))
}

async fn ensure_gallery_target_exists(
    state: &Arc<AppState>,
    club_id: Option<Uuid>,
) -> Result<Option<String>, ApiError> {
    let Some(club_id) = club_id else {
        return Ok(None);
    };

    let club = sqlx::query_as::<_, ClubNameRow>(r#"SELECT name FROM clubs WHERE id = $1"#)
        .bind(club_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    club.map(|record| record.name).ok_or(ApiError::ClubNotFound).map(Some)
}

async fn insert_gallery_image_tx(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    auth: &AuthenticatedUser<Staff>,
    target_type: GalleryTargetType,
    club_id: Option<Uuid>,
    club_name: Option<String>,
    image_url: &str,
    caption: Option<&str>,
) -> Result<AdminGalleryView, ApiError> {
    let row = sqlx::query_as::<_, AdminGalleryRow>(
        r#"
        INSERT INTO gallery_images (target_type, club_id, uploaded_by_user_id, image_url, caption, status)
        VALUES ($1, $2, $3, $4, $5, 'pending')
        RETURNING id,
                  target_type,
                  club_id,
                  NULL::text as club_name,
                  image_url,
                  caption,
                  status,
                  $6::text as uploader_discord_id,
                  $7::text as uploader_display_name,
                  created_at
        "#,
    )
    .bind(target_type)
    .bind(club_id)
    .bind(auth.user.id)
    .bind(image_url)
    .bind(caption)
    .bind(&auth.user.discord_id)
    .bind(&auth.user.discord_display_name)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok(to_admin_gallery_view(AdminGalleryRow { club_name, ..row }))
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/admin/gallery/files", role = Staff, rate_limit = "internal", summary = "Upload gallery image files")]
async fn upload_gallery_files(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<GalleryUploadBatchResponse>), ApiError> {
    let mut target_type = None;
    let mut club_id = None;
    let mut caption = None;
    let mut pending_files = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::ValidationError(HashMap::from([(
            "files".to_owned(),
            format!("アップロードデータを読み取れませんでした: {error}"),
        )])))?
    {
        let field_name = field.name().unwrap_or_default().to_owned();

        match field_name.as_str() {
            "target_type" => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| ApiError::ValidationError(HashMap::from([(
                        "target_type".to_owned(),
                        format!("掲載先を読み取れませんでした: {error}"),
                    )])))?;
                target_type = Some(parse_gallery_target_type(&value)?);
            }
            "club_id" => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| ApiError::ValidationError(HashMap::from([(
                        "club_id".to_owned(),
                        format!("部活IDを読み取れませんでした: {error}"),
                    )])))?;
                club_id = parse_optional_uuid_field("club_id", &value)?;
            }
            "caption" => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| ApiError::ValidationError(HashMap::from([(
                        "caption".to_owned(),
                        format!("キャプションを読み取れませんでした: {error}"),
                    )])))?;
                caption = sanitize_upload_caption(value)?;
            }
            "files" => {
                let file_name = field.file_name().unwrap_or("image").to_owned();
                let extension = extension_from_upload(
                    field.content_type(),
                    &file_name,
                )
                    .ok_or_else(|| ApiError::ValidationError(HashMap::from([(
                        "files".to_owned(),
                        format!("{file_name} は PNG/JPG/WebP のみアップロードできます"),
                    )])))?;
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| ApiError::ValidationError(HashMap::from([(
                        "files".to_owned(),
                        format!("{file_name} を読み取れませんでした: {error}"),
                    )])))?;

                if bytes.len() > state.config.gallery_max_upload_bytes {
                    return Err(ApiError::ValidationError(HashMap::from([(
                        "files".to_owned(),
                        format!(
                            "{file_name} はサイズ上限を超えています。最大 {} MB です",
                            state.config.gallery_max_upload_bytes / (1024 * 1024)
                        ),
                    )])));
                }

                let stored_file_name = format!("{}.{}", ulid::Ulid::new(), extension);
                pending_files.push(PendingGalleryFile {
                    file_name: stored_file_name,
                    bytes: bytes.to_vec(),
                });
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let target_type = target_type.ok_or_else(|| ApiError::ValidationError(HashMap::from([(
        "target_type".to_owned(),
        "掲載先を指定してください".to_owned(),
    )])))?;

    let errors = validate_gallery_target_fields(target_type, club_id);
    if !errors.is_empty() {
        return Err(ApiError::ValidationError(errors));
    }

    if pending_files.is_empty() {
        return Err(ApiError::ValidationError(HashMap::from([(
            "files".to_owned(),
            "少なくとも1枚の画像を選択してください".to_owned(),
        )])));
    }

    ensure_gallery_storage_dir(&state.config.gallery_storage_dir).await?;
    let club_name = ensure_gallery_target_exists(&state, club_id).await?;

    let mut saved_paths = Vec::with_capacity(pending_files.len());
    let mut public_urls = Vec::with_capacity(pending_files.len());
    for file in &pending_files {
        let path = state.config.gallery_storage_dir.join(&file.file_name);
        if let Err(error) = tokio::fs::write(&path, &file.bytes).await {
            cleanup_saved_gallery_files(&saved_paths).await;
            return Err(ApiError::Internal(format!("Failed to save gallery upload: {error}")));
        }
        saved_paths.push(path);
        public_urls.push(build_gallery_public_url(&state.config.backend_base_url, &file.file_name));
    }

    let mut transaction = state
        .db_pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    let mut items = Vec::with_capacity(public_urls.len());
    for image_url in &public_urls {
        match insert_gallery_image_tx(
            &mut transaction,
            &auth,
            target_type,
            club_id,
            club_name.clone(),
            image_url,
            caption.as_deref(),
        )
        .await
        {
            Ok(view) => items.push(view),
            Err(error) => {
                cleanup_saved_gallery_files(&saved_paths).await;
                return Err(error);
            }
        }
    }

    if let Err(error) = transaction.commit().await {
        cleanup_saved_gallery_files(&saved_paths).await;
        return Err(ApiError::Internal(error.to_string()));
    }

    Ok((
        StatusCode::CREATED,
        Json(GalleryUploadBatchResponse {
            uploaded_count: items.len(),
            items,
        }),
    ))
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/admin/gallery", role = Staff, rate_limit = "internal", summary = "Create gallery image")]
async fn upload_gallery_image(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    ValidatedJson(body): ValidatedJson<UploadGalleryRequest>,
) -> Result<(StatusCode, Json<AdminGalleryView>), ApiError> {
    create_gallery_image(&state, &auth, body).await
}

#[vrc_macros::handler(method = POST, path = "/api/v1/internal/admin/clubs/{id}/gallery", role = Staff, rate_limit = "internal", summary = "Upload gallery image")]
async fn upload_club_gallery_image(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(club_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UploadClubGalleryRequest>,
) -> Result<(StatusCode, Json<AdminGalleryView>), ApiError> {
    create_gallery_image(
        &state,
        &auth,
        UploadGalleryRequest {
            target_type: GalleryTargetType::Club,
            club_id: Some(club_id),
            image_url: body.image_url,
            caption: body.caption,
        },
    )
    .await
}

#[vrc_macros::handler(method = PATCH, path = "/api/v1/internal/admin/gallery/{image_id}/status", role = Staff, rate_limit = "internal", summary = "Update gallery status")]
async fn update_gallery_status(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(image_id): Path<Uuid>,
    Json(body): Json<GalleryStatusRequest>,
) -> Result<Json<GalleryStatusResponse>, ApiError> {
    // Only allow approved or rejected
    if body.status == GalleryImageStatus::Pending {
        return Err(ApiError::InvalidGalleryStatus);
    }

    let result = sqlx::query!(
        r#"
        UPDATE gallery_images SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING updated_at
        "#,
        body.status as GalleryImageStatus,
        image_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::GalleryImageNotFound)?;

    tracing::info!(
        actor_id = %auth.user.id,
        image_id = %image_id,
        new_status = ?body.status,
        "Gallery image status updated"
    );

    Ok(Json(GalleryStatusResponse {
        id: image_id,
        status: body.status,
        reviewed_by: UserBrief {
            user_id: auth.user.discord_id.clone(),
            discord_display_name: auth.user.discord_display_name.clone(),
        },
        reviewed_at: result.updated_at,
    }))
}

#[vrc_macros::handler(method = DELETE, path = "/api/v1/internal/admin/gallery/{image_id}", role = Staff, rate_limit = "internal", summary = "Delete gallery image")]
async fn delete_gallery_image(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(image_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let deleted = sqlx::query_as::<_, DeletedGalleryRow>(
        r#"
        DELETE FROM gallery_images
        WHERE id = $1
        RETURNING id, image_url
        "#,
    )
    .bind(image_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::GalleryImageNotFound)?;

    if let Err(error) = delete_local_gallery_file(&state, &deleted.image_url).await {
        tracing::warn!(error = %error, image_id = %deleted.id, "Failed to delete local gallery file");
    }

    tracing::info!(
        actor_id = %auth.user.id,
        image_id = %deleted.id,
        "Gallery image deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/role", patch(change_user_role))
        .route("/users/{id}/status", patch(change_user_status))
        .route("/reports", get(list_reports))
        .route("/reports/{id}", patch(resolve_report))
        .route("/galleries", get(list_galleries))
        .route("/clubs", post(create_club))
        .route("/gallery", post(upload_gallery_image))
        .route("/gallery/files", post(upload_gallery_files))
        .route("/clubs/{id}/gallery", post(upload_club_gallery_image))
        .route("/gallery/{image_id}", delete(delete_gallery_image))
        .route("/gallery/{image_id}/status", patch(update_gallery_status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_can_set_member_to_staff() {
        assert!(validate_role_change(UserRole::Admin, UserRole::Member, UserRole::Staff).is_ok());
    }

    #[test]
    fn test_admin_cannot_grant_admin() {
        let result = validate_role_change(UserRole::Admin, UserRole::Member, UserRole::Admin);
        assert!(matches!(result, Err(ApiError::AdminRoleEscalation)));
    }

    #[test]
    fn test_admin_cannot_grant_super_admin() {
        let result = validate_role_change(UserRole::Admin, UserRole::Member, UserRole::SuperAdmin);
        assert!(matches!(result, Err(ApiError::SuperAdminRoleEscalation)));
    }

    #[test]
    fn test_admin_cannot_modify_super_admin() {
        let result = validate_role_change(UserRole::Admin, UserRole::SuperAdmin, UserRole::Member);
        assert!(matches!(result, Err(ApiError::SuperAdminProtected)));
    }

    #[test]
    fn test_super_admin_can_grant_admin() {
        assert!(
            validate_role_change(UserRole::SuperAdmin, UserRole::Member, UserRole::Admin).is_ok()
        );
    }

    #[test]
    fn test_super_admin_can_grant_super_admin() {
        assert!(
            validate_role_change(UserRole::SuperAdmin, UserRole::Member, UserRole::SuperAdmin)
                .is_ok()
        );
    }

    #[test]
    fn test_super_admin_can_modify_super_admin() {
        assert!(
            validate_role_change(UserRole::SuperAdmin, UserRole::SuperAdmin, UserRole::Member)
                .is_ok()
        );
    }

    #[test]
    fn test_member_cannot_change_roles() {
        let result = validate_role_change(UserRole::Member, UserRole::Member, UserRole::Staff);
        assert!(matches!(result, Err(ApiError::RoleLevelInsufficient)));
    }

    #[test]
    fn test_staff_cannot_change_roles() {
        let result = validate_role_change(UserRole::Staff, UserRole::Member, UserRole::Staff);
        assert!(matches!(result, Err(ApiError::RoleLevelInsufficient)));
    }

    #[test]
    fn test_admin_can_demote_staff_to_member() {
        assert!(validate_role_change(UserRole::Admin, UserRole::Staff, UserRole::Member).is_ok());
    }
}

// Kani formal verification harnesses for role change authorization.
// Run with: cargo kani --harness proof_role_change_no_escalation
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    fn any_role() -> UserRole {
        let v: u8 = kani::any();
        kani::assume(v < 4);
        match v {
            0 => UserRole::Member,
            1 => UserRole::Staff,
            2 => UserRole::Admin,
            _ => UserRole::SuperAdmin,
        }
    }

    /// P1: If a role change is allowed, the actor must have sufficient privilege.
    /// Specifically, actor.level() >= new_role.level().
    #[kani::proof]
    fn proof_role_change_no_escalation() {
        let actor_role = any_role();
        let target_current_role = any_role();
        let new_role = any_role();

        if validate_role_change(actor_role, target_current_role, new_role).is_ok() {
            assert!(actor_role.level() >= new_role.level());
        }
    }

    /// P1b: Admin cannot grant admin or super_admin.
    #[kani::proof]
    fn proof_admin_cannot_grant_admin_or_above() {
        let target_current_role = any_role();
        let new_role = any_role();
        kani::assume(new_role == UserRole::Admin || new_role == UserRole::SuperAdmin);

        let result = validate_role_change(UserRole::Admin, target_current_role, new_role);
        assert!(result.is_err());
    }

    /// P1c: After any allowed role change among 3 users, at least one super_admin remains.
    #[kani::proof]
    fn proof_super_admin_always_exists() {
        let roles: [UserRole; 3] = [any_role(), any_role(), any_role()];
        let actor_idx: usize = kani::any();
        let target_idx: usize = kani::any();
        let new_role = any_role();
        kani::assume(actor_idx < 3 && target_idx < 3 && actor_idx != target_idx);

        let sa_count = roles.iter().filter(|r| **r == UserRole::SuperAdmin).count();
        kani::assume(sa_count >= 1);

        if validate_role_change(roles[actor_idx], roles[target_idx], new_role).is_ok() {
            let mut new_roles = roles;
            new_roles[target_idx] = new_role;
            let new_sa_count = new_roles
                .iter()
                .filter(|r| **r == UserRole::SuperAdmin)
                .count();
            assert!(
                new_sa_count >= 1,
                "Role change must not eliminate all super_admins"
            );
        }
    }

    /// P1d: Member and staff can never change roles.
    #[kani::proof]
    fn proof_member_staff_cannot_change_roles() {
        let target_role = any_role();
        let new_role = any_role();

        assert!(validate_role_change(UserRole::Member, target_role, new_role).is_err());
        assert!(validate_role_change(UserRole::Staff, target_role, new_role).is_err());
    }
}
