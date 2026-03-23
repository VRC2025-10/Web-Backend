use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use crate::adapters::inbound::extractors::ValidatedQuery;
use crate::domain::entities::event::EventStatus;
use crate::domain::entities::gallery::GalleryTargetType;
use crate::domain::value_objects::pagination::{PageRequest, PageResponse};
use crate::errors::api::ApiError;

// ===== Shared types =====

#[derive(Serialize)]
struct UserBrief {
    user_id: String,
    discord_display_name: String,
}

// ===== Members =====

#[derive(Serialize)]
struct PublicProfileSummary {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_html: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
struct PublicMemberSummary {
    user_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    joined_at: DateTime<Utc>,
    profile: Option<PublicProfileSummary>,
}

#[derive(Serialize)]
struct PublicProfileDetail {
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_html: Option<String>,
    avatar_url: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ClubMembership {
    id: Uuid,
    name: String,
    role: String,
}

#[derive(Serialize)]
struct PublicMemberDetail {
    user_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    joined_at: DateTime<Utc>,
    profile: Option<PublicProfileDetail>,
    clubs: Vec<ClubMembership>,
}

// ===== Events =====

#[derive(serde::Deserialize)]
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
                "query".to_owned(),
                "クエリパラメータが不正です".to_owned(),
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
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    location: Option<String>,
    tags: Vec<String>,
    created_at: DateTime<Utc>,
}

// ===== Clubs =====

#[derive(Serialize)]
struct ClubSummary {
    id: Uuid,
    name: String,
    description_html: Option<String>,
    cover_image_url: Option<String>,
    owner: UserBrief,
    member_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ClubMemberInfo {
    user_id: String,
    discord_display_name: String,
    role: String,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ClubDetail {
    id: Uuid,
    name: String,
    description_html: Option<String>,
    cover_image_url: Option<String>,
    owner: UserBrief,
    members: Vec<ClubMemberInfo>,
    created_at: DateTime<Utc>,
}

// ===== Gallery =====

#[derive(Serialize)]
struct GalleryImagePublic {
    id: Uuid,
    image_url: String,
    caption: Option<String>,
    uploaded_by: UserBrief,
    created_at: DateTime<Utc>,
}

// ===== Row types for SQL queries =====

struct MemberRow {
    discord_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    joined_at: DateTime<Utc>,
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_html: Option<String>,
    avatar_url: Option<String>,
}

struct MemberDetailRow {
    discord_id: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    joined_at: DateTime<Utc>,
    nickname: Option<String>,
    vrc_id: Option<String>,
    x_id: Option<String>,
    bio_html: Option<String>,
    avatar_url: Option<String>,
    profile_updated_at: Option<DateTime<Utc>>,
}

struct ClubListRow {
    id: Uuid,
    name: String,
    description_html: Option<String>,
    cover_image_url: Option<String>,
    owner_discord_id: String,
    owner_display_name: String,
    member_count: i64,
    created_at: DateTime<Utc>,
}

struct ClubDetailRow {
    id: Uuid,
    name: String,
    description_html: Option<String>,
    cover_image_url: Option<String>,
    owner_discord_id: String,
    owner_display_name: String,
    created_at: DateTime<Utc>,
}

struct ClubMemberRow {
    discord_id: String,
    discord_display_name: String,
    role: String,
    joined_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct GalleryRow {
    id: Uuid,
    image_url: String,
    caption: Option<String>,
    uploader_discord_id: String,
    uploader_display_name: String,
    created_at: DateTime<Utc>,
}

// ===== Handlers =====

#[vrc_macros::handler(method = GET, path = "/api/v1/public/members", summary = "List public members")]
async fn list_members(
    State(state): State<Arc<AppState>>,
    ValidatedQuery(page): ValidatedQuery<PageRequest>,
) -> Result<PageResponse<PublicMemberSummary>, ApiError> {
    let rows_future = sqlx::query_as!(
        MemberRow,
        r#"
        SELECT u.discord_id, u.discord_display_name, u.discord_avatar_hash,
               u.joined_at,
               p.nickname, p.vrc_id, p.x_id, p.bio_html, p.avatar_url
        FROM users u
        JOIN profiles p ON p.user_id = u.id AND p.is_public = true
        WHERE u.status = 'active'
        ORDER BY u.joined_at DESC
        LIMIT $1 OFFSET $2
        "#,
        page.limit(),
        page.offset()
    )
    .fetch_all(&state.db_pool);

    let count_future = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) as "count!: i64"
        FROM users u
        JOIN profiles p ON p.user_id = u.id AND p.is_public = true
        WHERE u.status = 'active'
        "#,
    )
    .fetch_one(&state.db_pool);

    let (rows, count) = tokio::try_join!(rows_future, count_future)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<PublicMemberSummary> = rows
        .into_iter()
        .map(|r| PublicMemberSummary {
            user_id: r.discord_id,
            discord_display_name: r.discord_display_name,
            discord_avatar_hash: r.discord_avatar_hash,
            joined_at: r.joined_at,
            profile: Some(PublicProfileSummary {
                nickname: r.nickname,
                vrc_id: r.vrc_id,
                x_id: r.x_id,
                bio_html: r.bio_html,
                avatar_url: r.avatar_url,
            }),
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/public/members/{user_id}", summary = "Get public member")]
async fn get_member(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<PublicMemberDetail>, ApiError> {
    let row = sqlx::query_as!(
        MemberDetailRow,
        r#"
        SELECT u.discord_id, u.discord_display_name, u.discord_avatar_hash,
               u.joined_at,
               p.nickname, p.vrc_id, p.x_id, p.bio_html, p.avatar_url,
               p.updated_at as profile_updated_at
        FROM users u
        JOIN profiles p ON p.user_id = u.id AND p.is_public = true
        WHERE u.discord_id = $1 AND u.status = 'active'
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::ProfileNotFound)?;

    // Fetch clubs for this user
    let clubs = sqlx::query!(
        r#"
        SELECT c.id, c.name, cm.role
        FROM club_members cm
        JOIN clubs c ON c.id = cm.club_id
        JOIN users u ON u.id = cm.user_id
        WHERE u.discord_id = $1
        ORDER BY c.name
        "#,
        user_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let profile = row
        .profile_updated_at
        .map(|updated_at| PublicProfileDetail {
            nickname: row.nickname,
            vrc_id: row.vrc_id,
            x_id: row.x_id,
            bio_html: row.bio_html,
            avatar_url: row.avatar_url,
            updated_at,
        });

    Ok(Json(PublicMemberDetail {
        user_id: row.discord_id,
        discord_display_name: row.discord_display_name,
        discord_avatar_hash: row.discord_avatar_hash,
        joined_at: row.joined_at,
        profile,
        clubs: clubs
            .into_iter()
            .map(|c| ClubMembership {
                id: c.id,
                name: c.name,
                role: c.role,
            })
            .collect(),
    }))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/public/events", summary = "List public events")]
async fn list_events(
    State(state): State<Arc<AppState>>,
    ValidatedQuery(query): ValidatedQuery<EventListQuery>,
) -> Result<PageResponse<EventSummary>, ApiError> {
    let now = Utc::now();
    let page = query.page_request()?;

    // Public events only show published (or optionally filtered)
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

#[vrc_macros::handler(method = GET, path = "/api/v1/public/events/{event_id}", summary = "Get public event")]
async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventSummary>, ApiError> {
    let now = Utc::now();

    let event = sqlx::query_as!(
        crate::domain::entities::event::Event,
        r#"
        SELECT id, external_source_id, title, description_markdown, description_html,
               host_user_id, host_name, event_status as "event_status: EventStatus",
               start_time, end_time, location, created_at, updated_at
        FROM events
        WHERE id = $1
        "#,
        event_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::EventNotFound)?;

    let tags: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT t.name
        FROM event_tags t
        JOIN event_tag_mappings m ON m.tag_id = t.id
        WHERE m.event_id = $1
        "#,
        event_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let display_status = event.display_status(now);

    Ok(Json(EventSummary {
        id: event.id,
        title: event.title,
        description_html: event.description_html,
        status: event.event_status,
        display_status,
        start_time: event.start_time,
        end_time: event.end_time,
        location: event.location,
        tags,
        created_at: event.created_at,
    }))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/public/clubs", summary = "List public clubs")]
async fn list_clubs(
    State(state): State<Arc<AppState>>,
    ValidatedQuery(page): ValidatedQuery<PageRequest>,
) -> Result<PageResponse<ClubSummary>, ApiError> {
    let rows = sqlx::query_as!(
        ClubListRow,
        r#"
         SELECT c.id, c.name, c.description_html, c.cover_image_url,
               u.discord_id as owner_discord_id,
               u.discord_display_name as owner_display_name,
               COUNT(cm.user_id) as "member_count!: i64",
               c.created_at
        FROM clubs c
        JOIN users u ON u.id = c.owner_user_id
        LEFT JOIN club_members cm ON cm.club_id = c.id
         GROUP BY c.id, c.name, c.description_html, c.cover_image_url, u.discord_id, u.discord_display_name, c.created_at
        ORDER BY c.name
        LIMIT $1 OFFSET $2
        "#,
        page.limit(),
        page.offset()
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!: i64" FROM clubs"#,)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<ClubSummary> = rows
        .into_iter()
        .map(|r| ClubSummary {
            id: r.id,
            name: r.name,
            description_html: r.description_html,
            cover_image_url: r.cover_image_url,
            owner: UserBrief {
                user_id: r.owner_discord_id,
                discord_display_name: r.owner_display_name,
            },
            member_count: r.member_count,
            created_at: r.created_at,
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/public/clubs/{id}", summary = "Get public club")]
async fn get_club(
    State(state): State<Arc<AppState>>,
    Path(club_id): Path<Uuid>,
) -> Result<Json<ClubDetail>, ApiError> {
    let row = sqlx::query_as!(
        ClubDetailRow,
        r#"
        SELECT c.id, c.name, c.description_html, c.cover_image_url,
               u.discord_id as owner_discord_id,
               u.discord_display_name as owner_display_name,
               c.created_at
        FROM clubs c
        JOIN users u ON u.id = c.owner_user_id
        WHERE c.id = $1
        "#,
        club_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or(ApiError::ClubNotFound)?;

    let members = sqlx::query_as!(
        ClubMemberRow,
        r#"
        SELECT u.discord_id, u.discord_display_name, cm.role, cm.joined_at
        FROM club_members cm
        JOIN users u ON u.id = cm.user_id
        WHERE cm.club_id = $1
        ORDER BY cm.joined_at
        "#,
        club_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ClubDetail {
        id: row.id,
        name: row.name,
        description_html: row.description_html,
        cover_image_url: row.cover_image_url,
        owner: UserBrief {
            user_id: row.owner_discord_id,
            discord_display_name: row.owner_display_name,
        },
        members: members
            .into_iter()
            .map(|m| ClubMemberInfo {
                user_id: m.discord_id,
                discord_display_name: m.discord_display_name,
                role: m.role,
                joined_at: m.joined_at,
            })
            .collect(),
        created_at: row.created_at,
    }))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/public/clubs/{id}/gallery", summary = "List public gallery images")]
async fn list_gallery(
    State(state): State<Arc<AppState>>,
    Path(club_id): Path<Uuid>,
    ValidatedQuery(page): ValidatedQuery<PageRequest>,
) -> Result<PageResponse<GalleryImagePublic>, ApiError> {
    // Verify club exists
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM clubs WHERE id = $1) as "exists!: bool""#,
        club_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !exists {
        return Err(ApiError::ClubNotFound);
    }

    let rows = sqlx::query_as::<_, GalleryRow>(
        r#"
        SELECT g.id, g.image_url, g.caption,
               u.discord_id as uploader_discord_id,
               u.discord_display_name as uploader_display_name,
               g.created_at
        FROM gallery_images g
        JOIN users u ON u.id = g.uploaded_by_user_id
        WHERE g.club_id = $1 AND g.status = 'approved' AND g.target_type = $2
        ORDER BY g.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(club_id)
    .bind(GalleryTargetType::Club)
    .bind(page.limit())
    .bind(page.offset())
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM gallery_images
        WHERE club_id = $1 AND status = 'approved' AND target_type = $2
        "#,
    )
    .bind(club_id)
    .bind(GalleryTargetType::Club)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<GalleryImagePublic> = rows
        .into_iter()
        .map(|r| GalleryImagePublic {
            id: r.id,
            image_url: r.image_url,
            caption: r.caption,
            uploaded_by: UserBrief {
                user_id: r.uploader_discord_id,
                discord_display_name: r.uploader_display_name,
            },
            created_at: r.created_at,
        })
        .collect();

    Ok(PageResponse::new(items, count, page.per_page()))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/members", get(list_members))
        .route("/members/{user_id}", get(get_member))
        .route("/events", get(list_events))
        .route("/events/{event_id}", get(get_event))
        .route("/clubs", get(list_clubs))
        .route("/clubs/{id}", get(get_club))
        .route("/clubs/{id}/gallery", get(list_gallery))
}
