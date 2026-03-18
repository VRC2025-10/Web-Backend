use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::adapters::outbound::markdown::renderer::PulldownCmarkRenderer;
use crate::auth::extractor::AuthenticatedUser;
use crate::auth::roles::{Admin, Role, Staff};
use crate::domain::entities::gallery::GalleryImageStatus;
use crate::domain::entities::report::{ReportStatus, ReportTargetType};
use crate::domain::entities::user::{UserRole, UserStatus};
use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;
use crate::domain::value_objects::pagination::{PageRequest, PageResponse};
use crate::errors::api::ApiError;

// ===== User management types =====

#[derive(Deserialize)]
struct UserListQuery {
    #[serde(flatten)]
    page: PageRequest,
    status: Option<UserStatus>,
    role: Option<UserRole>,
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
    #[serde(flatten)]
    page: PageRequest,
    status: Option<ReportStatus>,
}

#[derive(Serialize)]
struct AdminReportView {
    id: Uuid,
    reporter_user_id: Uuid,
    reporter_display_name: String,
    target_type: ReportTargetType,
    target_id: Uuid,
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

#[derive(Deserialize)]
struct CreateClubRequest {
    name: String,
    description_markdown: Option<String>,
    owner_user_id: Uuid,
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
struct UploadGalleryRequest {
    image_url: String,
    caption: Option<String>,
}

#[derive(Serialize)]
struct GalleryUploadResponse {
    id: Uuid,
    club_id: Uuid,
    image_url: String,
    caption: Option<String>,
    status: GalleryImageStatus,
    uploaded_by: UserBrief,
    created_at: DateTime<Utc>,
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

struct ReportRow {
    id: Uuid,
    reporter_user_id: Uuid,
    reporter_display_name: String,
    target_type: ReportTargetType,
    target_id: Uuid,
    reason: String,
    status: ReportStatus,
    created_at: DateTime<Utc>,
}

// ===== Handlers =====

async fn list_users(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Admin>,
    Query(mut query): Query<UserListQuery>,
) -> Result<Json<PageResponse<AdminUserView>>, ApiError> {
    query.page.validate();

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
        query.page.limit(),
        query.page.offset()
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

    Ok(Json(PageResponse::new(items, count, query.page.per_page)))
}

/// Validate role change authorization rules per spec:
/// - Only admin+ can change roles (ERR-ROLE-004)
/// - Cannot modify super_admin unless you are super_admin (ERR-ROLE-003)
/// - Only super_admin can grant admin (ERR-ROLE-001)
/// - Only super_admin can grant super_admin (ERR-ROLE-002)
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
    if body.status == UserStatus::Suspended {
        let _ = sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
            .execute(&state.db_pool)
            .await;
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

async fn list_reports(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser<Staff>,
    Query(mut query): Query<ReportListQuery>,
) -> Result<Json<PageResponse<AdminReportView>>, ApiError> {
    query.page.validate();

    let rows = sqlx::query_as!(
        ReportRow,
        r#"
        SELECT r.id,
               r.reporter_user_id,
               u.discord_display_name as reporter_display_name,
               r.target_type as "target_type: ReportTargetType",
               r.target_id,
               r.reason,
               r.status as "status: ReportStatus",
               r.created_at
        FROM reports r
        JOIN users u ON u.id = r.reporter_user_id
        WHERE ($1::report_status IS NULL OR r.status = $1)
        ORDER BY r.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        query.status as Option<ReportStatus>,
        query.page.limit(),
        query.page.offset()
    )
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

    Ok(Json(PageResponse::new(items, count, query.page.per_page)))
}

async fn resolve_report(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(report_id): Path<Uuid>,
    Json(body): Json<ResolveReportRequest>,
) -> Result<Json<ResolveReportResponse>, ApiError> {
    // Only allow resolving to reviewed or dismissed — not back to pending
    if body.status == ReportStatus::Pending {
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

async fn create_club(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Json(body): Json<CreateClubRequest>,
) -> Result<(StatusCode, Json<ClubResponse>), ApiError> {
    let mut errors: HashMap<String, String> = HashMap::new();

    if body.name.is_empty() || body.name.len() > 100 {
        errors.insert("name".to_owned(), "1〜100文字で入力してください".to_owned());
    }

    if let Some(ref desc) = body.description_markdown {
        if desc.len() > 2000 {
            errors.insert(
                "description_markdown".to_owned(),
                "2000文字以内で入力してください".to_owned(),
            );
        }
    }

    if !errors.is_empty() {
        return Err(ApiError::ValidationError(errors));
    }

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

async fn upload_gallery_image(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser<Staff>,
    Path(club_id): Path<Uuid>,
    Json(body): Json<UploadGalleryRequest>,
) -> Result<(StatusCode, Json<GalleryUploadResponse>), ApiError> {
    let mut errors: HashMap<String, String> = HashMap::new();

    if !body.image_url.starts_with("https://") || body.image_url.len() > 500 {
        errors.insert(
            "image_url".to_owned(),
            "有効なHTTPS URLを入力してください".to_owned(),
        );
    }

    if let Some(ref caption) = body.caption {
        if caption.len() > 200 {
            errors.insert(
                "caption".to_owned(),
                "200文字以内で入力してください".to_owned(),
            );
        }
    }

    if !errors.is_empty() {
        return Err(ApiError::ValidationError(errors));
    }

    // Verify club exists
    let club_exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM clubs WHERE id = $1) as "exists!: bool""#,
        club_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !club_exists {
        return Err(ApiError::ClubNotFound);
    }

    let image = sqlx::query!(
        r#"
        INSERT INTO gallery_images (club_id, uploaded_by_user_id, image_url, caption, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING id, created_at
        "#,
        club_id,
        auth.user.id,
        body.image_url,
        body.caption,
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(GalleryUploadResponse {
            id: image.id,
            club_id,
            image_url: body.image_url,
            caption: body.caption,
            status: GalleryImageStatus::Pending,
            uploaded_by: UserBrief {
                user_id: auth.user.discord_id.clone(),
                discord_display_name: auth.user.discord_display_name.clone(),
            },
            created_at: image.created_at,
        }),
    ))
}

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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/role", patch(change_user_role))
        .route("/users/{id}/status", patch(change_user_status))
        .route("/reports", get(list_reports))
        .route("/reports/{id}", patch(resolve_report))
        .route("/clubs", post(create_club))
        .route("/clubs/{id}/gallery", post(upload_gallery_image))
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
