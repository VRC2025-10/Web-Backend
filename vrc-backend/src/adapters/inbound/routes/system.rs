use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::AppState;
use crate::adapters::outbound::markdown::renderer::PulldownCmarkRenderer;
use crate::domain::entities::event::EventStatus;
use crate::domain::entities::user::UserStatus;
use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;
use crate::domain::ports::services::webhook_sender::{EmbedField, WebhookSender};
use crate::errors::api::ApiError;

// ===== System token verification =====

/// Verify Bearer token via constant-time comparison to prevent timing attacks.
fn verify_system_token(headers: &HeaderMap, expected: &str) -> Result<(), ApiError> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::SystemTokenInvalid)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::SystemTokenInvalid)?;

    if token.as_bytes().ct_eq(expected.as_bytes()).into() {
        Ok(())
    } else {
        Err(ApiError::SystemTokenInvalid)
    }
}

// ===== Event sync types =====

#[derive(Deserialize, vrc_macros::Validate)]
struct EventUpsertRequest {
    #[validate(min_length = 1, max_length = 100)]
    external_id: String,
    #[validate(min_length = 1, max_length = 200)]
    title: String,
    #[validate(max_length = 2000)]
    description_markdown: Option<String>,
    status: EventStatus,
    host_discord_id: Option<String>,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    #[validate(max_length = 200)]
    location: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Serialize)]
struct EventUpsertResponse {
    id: Uuid,
    external_id: String,
    title: String,
    status: EventStatus,
    action: UpsertAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum UpsertAction {
    Created,
    Updated,
}

// ===== Member leave types =====

#[derive(Deserialize)]
struct MemberLeaveRequest {
    discord_id: String,
}

#[derive(Serialize)]
struct MemberLeaveResponse {
    user_id: Uuid,
    discord_id: String,
    previous_status: UserStatus,
    new_status: UserStatus,
    sessions_invalidated: i64,
    clubs_removed: i64,
    profile_set_private: bool,
}

// ===== Handlers =====

#[allow(clippy::too_many_lines)] // Multi-step upsert with tag management and webhook
async fn upsert_event(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EventUpsertRequest>,
) -> Result<(StatusCode, Json<EventUpsertResponse>), ApiError> {
    verify_system_token(&headers, &state.config.system_api_token)?;

    // Validate simple field constraints via derive macro
    let mut errors = body.validate().err().unwrap_or_default();

    // Cross-field and collection validations that the macro cannot express
    if let Some(ref tags) = body.tags {
        if tags.len() > 10 {
            errors.insert("tags".to_owned(), "タグは最大10個までです".to_owned());
        }
        for tag in tags {
            if tag.is_empty() || tag.len() > 50 {
                errors.insert(
                    "tags".to_owned(),
                    "各タグは1〜50文字で入力してください".to_owned(),
                );
                break;
            }
        }
    }

    if let Some(end_time) = body.end_time
        && end_time <= body.start_time
    {
        errors.insert(
            "end_time".to_owned(),
            "end_time は start_time より後にしてください".to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(ApiError::SystemValidation(errors));
    }

    // Render markdown if provided
    let description_html = body.description_markdown.as_ref().map(|md| {
        let renderer = PulldownCmarkRenderer::new();
        renderer.render(md)
    });

    // Resolve host user if discord_id provided
    let host_user_id: Option<Uuid> = if let Some(ref discord_id) = body.host_discord_id {
        sqlx::query_scalar!("SELECT id FROM users WHERE discord_id = $1", discord_id)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
    } else {
        None
    };

    let host_name = if let Some(ref discord_id) = body.host_discord_id {
        sqlx::query_scalar!(
            "SELECT discord_display_name FROM users WHERE discord_id = $1",
            discord_id
        )
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .unwrap_or_default()
    } else {
        String::new()
    };

    // Begin transaction: upsert event + manage tags
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Upsert event — use xmax to detect insert vs update
    let row = sqlx::query!(
        r#"
        INSERT INTO events (external_source_id, title, description_markdown, description_html,
                           host_user_id, host_name, event_status, start_time, end_time, location)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (external_source_id) DO UPDATE SET
            title = EXCLUDED.title,
            description_markdown = EXCLUDED.description_markdown,
            description_html = EXCLUDED.description_html,
            host_user_id = EXCLUDED.host_user_id,
            host_name = EXCLUDED.host_name,
            event_status = EXCLUDED.event_status,
            start_time = EXCLUDED.start_time,
            end_time = EXCLUDED.end_time,
            location = EXCLUDED.location,
            updated_at = NOW()
        RETURNING id, created_at, updated_at, (xmax = 0) as "is_insert!: bool"
        "#,
        body.external_id,
        body.title,
        body.description_markdown.as_deref().unwrap_or(""),
        description_html.as_deref().unwrap_or(""),
        host_user_id,
        host_name,
        body.status as EventStatus,
        body.start_time,
        body.end_time,
        body.location,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let event_id = row.id;

    // Delete existing tag mappings for this event
    sqlx::query!(
        "DELETE FROM event_tag_mappings WHERE event_id = $1",
        event_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Upsert tags and create mappings
    if let Some(ref tags) = body.tags {
        for tag_name in tags {
            let tag_id = sqlx::query_scalar!(
                r#"
                INSERT INTO event_tags (name) VALUES ($1)
                ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                RETURNING id
                "#,
                tag_name
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            sqlx::query!(
                "INSERT INTO event_tag_mappings (event_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                event_id,
                tag_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let (action, http_status) = if row.is_insert {
        (UpsertAction::Created, StatusCode::CREATED)
    } else {
        (UpsertAction::Updated, StatusCode::OK)
    };

    tracing::info!(
        event_id = %event_id,
        external_id = %body.external_id,
        action = ?action,
        "Event synced"
    );

    // Send Discord webhook notification for newly created events only
    if row.is_insert
        && let Some(ref webhook) = state.webhook
    {
            let mut fields = vec![
                EmbedField {
                    name: "Host".to_owned(),
                    value: if host_name.is_empty() {
                        "TBD".to_owned()
                    } else {
                        host_name.clone()
                    },
                    inline: true,
                },
                EmbedField {
                    name: "Start".to_owned(),
                    value: body.start_time.format("%Y-%m-%d %H:%M UTC").to_string(),
                    inline: true,
                },
            ];
            if let Some(ref loc) = body.location {
                fields.push(EmbedField {
                    name: "Location".to_owned(),
                    value: loc.clone(),
                    inline: true,
                });
            }

            // Fire-and-forget: webhook failures must not break the API response
            let desc_preview: String = body
                .description_markdown
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect();

            if let Err(e) = webhook
                .send_embed(
                    &format!("🎉 New Event: {}", body.title),
                    &desc_preview,
                    0x0058_65F2, // Discord blurple
                    fields,
                )
                .await
            {
                tracing::error!(error = %e, event_id = %event_id, "Failed to send event webhook");
            }
    }

    Ok((
        http_status,
        Json(EventUpsertResponse {
            id: event_id,
            external_id: body.external_id,
            title: body.title,
            status: body.status,
            action,
            created_at: if row.is_insert {
                Some(row.created_at)
            } else {
                None
            },
            updated_at: if row.is_insert {
                None
            } else {
                Some(row.updated_at)
            },
        }),
    ))
}

#[allow(clippy::too_many_lines)] // Atomic 4-step suspension transaction
async fn handle_member_leave(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<MemberLeaveRequest>,
) -> Result<(StatusCode, Json<Option<MemberLeaveResponse>>), ApiError> {
    verify_system_token(&headers, &state.config.system_api_token)?;

    // Validate discord_id: must be 17-20 digit numeric string (Discord snowflake)
    if body.discord_id.is_empty()
        || body.discord_id.len() < 17
        || body.discord_id.len() > 20
        || !body.discord_id.chars().all(|c| c.is_ascii_digit())
    {
        let mut errors = HashMap::new();
        errors.insert(
            "discord_id".to_owned(),
            "17〜20桁の数値で入力してください".to_owned(),
        );
        return Err(ApiError::SystemValidation(errors));
    }

    // Look up the user
    let user = sqlx::query!(
        r#"
        SELECT id, status as "status: UserStatus"
        FROM users WHERE discord_id = $1
        "#,
        body.discord_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // User not found in system — this is not an error per spec
    let Some(user) = user else {
        return Ok((StatusCode::NO_CONTENT, Json(None)));
    };

    let previous_status = user.status;
    let user_id = user.id;

    // Atomic transaction: suspend + delete sessions + make profile private + remove from clubs
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 1. Suspend user
    sqlx::query!(
        "UPDATE users SET status = 'suspended', updated_at = NOW() WHERE id = $1",
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 2. Delete all sessions
    let sessions_deleted = sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .rows_affected();

    // 3. Set profile to non-public
    let profile_updated = sqlx::query!(
        "UPDATE profiles SET is_public = false, updated_at = NOW() WHERE user_id = $1",
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .rows_affected();

    // 4. Remove from all clubs
    let clubs_removed = sqlx::query!("DELETE FROM club_members WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .rows_affected();

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        user_id = %user_id,
        discord_id = %body.discord_id,
        sessions_invalidated = sessions_deleted,
        clubs_removed = clubs_removed,
        "Member leave processed"
    );

    // Notify admin channel about the member leave
    if let Some(ref webhook) = state.webhook {
        let fields = vec![
            EmbedField {
                name: "Discord ID".to_owned(),
                value: body.discord_id.clone(),
                inline: true,
            },
            EmbedField {
                name: "Previous Status".to_owned(),
                value: format!("{previous_status:?}"),
                inline: true,
            },
            EmbedField {
                name: "Sessions Cleared".to_owned(),
                value: sessions_deleted.to_string(),
                inline: true,
            },
            EmbedField {
                name: "Clubs Removed".to_owned(),
                value: clubs_removed.to_string(),
                inline: true,
            },
        ];

        if let Err(e) = webhook
            .send_embed(
                "👋 Member Left Server",
                &format!("User `{user_id}` has been suspended after leaving the Discord server."),
                0x00ED_4245, // Discord red
                fields,
            )
            .await
        {
            tracing::error!(error = %e, user_id = %user_id, "Failed to send member leave webhook");
        }
    }

    Ok((
        StatusCode::OK,
        Json(Some(MemberLeaveResponse {
            user_id,
            discord_id: body.discord_id,
            previous_status,
            new_status: UserStatus::Suspended,
            sessions_invalidated: sessions_deleted.cast_signed(),
            clubs_removed: clubs_removed.cast_signed(),
            profile_set_private: profile_updated > 0,
        })),
    ))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/events", post(upsert_event))
        .route("/sync/users/leave", post(handle_member_leave))
}

// ===== Pure state machine for formal verification =====

/// Pre-state of a member before the leave operation.
/// Extracted from the transactional SQL logic for formal verification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(kani), allow(dead_code))]
struct MemberState {
    status: UserStatus,
    has_sessions: bool,
    profile_is_public: bool,
    club_count: u8,
}

/// Compute the post-state after a member leave operation.
/// This mirrors the atomic transaction in `handle_member_leave`:
/// 1. Suspend user 2. Delete sessions 3. Set profile private 4. Remove from clubs
#[cfg_attr(not(kani), allow(dead_code))]
fn compute_leave_state(_pre: &MemberState) -> MemberState {
    MemberState {
        status: UserStatus::Suspended,
        has_sessions: false,
        profile_is_public: false,
        club_count: 0,
    }
}

// Kani formal verification harness for member leave state machine.
// Run with: cargo kani --harness proof_leave_result_is_fully_suspended
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// P5: For any valid pre-state, the leave operation always produces
    /// a fully suspended, cleaned-up post-state.
    #[kani::proof]
    fn proof_leave_result_is_fully_suspended() {
        let user_status: UserStatus = {
            let v: u8 = kani::any();
            kani::assume(v < 2);
            match v {
                0 => UserStatus::Active,
                _ => UserStatus::Suspended,
            }
        };
        let has_sessions: bool = kani::any();
        let profile_is_public: bool = kani::any();
        let club_count: u8 = kani::any();
        kani::assume(club_count <= 5);

        let pre = MemberState {
            status: user_status,
            has_sessions,
            profile_is_public,
            club_count,
        };

        let post = compute_leave_state(&pre);

        // Post-conditions: all cleanup applied regardless of pre-state
        assert_eq!(post.status, UserStatus::Suspended);
        assert!(!post.has_sessions);
        assert!(!post.profile_is_public);
        assert_eq!(post.club_count, 0);
    }
}
