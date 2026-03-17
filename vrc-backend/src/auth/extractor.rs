use std::marker::PhantomData;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AppState;
use crate::auth::roles::Role;
use crate::domain::entities::user::{User, UserRole, UserStatus};
use crate::errors::api::ApiError;

/// Axum extractor that validates session and enforces minimum role at compile time.
///
/// Usage: `AuthenticatedUser<Member>` for any logged-in user,
///        `AuthenticatedUser<Admin>` for admin+ only.
pub struct AuthenticatedUser<R: Role> {
    pub user: User,
    pub session_id: Uuid,
    _phantom: PhantomData<R>,
}

impl<R: Role> AuthenticatedUser<R> {
    pub fn user_id(&self) -> Uuid {
        self.user.id
    }

    pub fn role(&self) -> UserRole {
        self.user.role
    }
}

impl<R: Role> FromRequestParts<Arc<AppState>> for AuthenticatedUser<R> {
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let state = state.clone();
        async move {
            use base64::Engine;
            use base64::engine::general_purpose::URL_SAFE_NO_PAD;

            let jar = CookieJar::from_headers(&parts.headers);

            let raw_token = jar
                .get("session_id")
                .ok_or(ApiError::SessionInvalid)?
                .value()
                .to_owned();

            // Decode base64url token
            let token_bytes = URL_SAFE_NO_PAD
                .decode(&raw_token)
                .map_err(|_| ApiError::SessionInvalid)?;

            // SHA-256 hash for lookup
            let mut hasher = Sha256::new();
            hasher.update(&token_bytes);
            let token_hash = hasher.finalize().to_vec();

            // Lookup session + user in a single query
            let row = sqlx::query_as!(
                SessionUserRow,
                r#"
                SELECT s.id as session_id, s.user_id, s.expires_at,
                       u.discord_id, u.discord_username,
                       u.discord_display_name, u.discord_avatar_hash,
                       u.avatar_url,
                       u.role as "role: UserRole", u.status as "status: UserStatus",
                       u.joined_at, u.created_at, u.updated_at
                FROM sessions s
                JOIN users u ON u.id = s.user_id
                WHERE s.token_hash = $1 AND s.expires_at > NOW()
                "#,
                &token_hash[..]
            )
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error during session validation");
                ApiError::Internal("Database error".to_owned())
            })?
            .ok_or(ApiError::SessionInvalid)?;

            // Check user status
            if row.status == UserStatus::Suspended {
                return Err(ApiError::AccountSuspended);
            }

            // Runtime role level check
            if row.role.level() < R::LEVEL {
                return Err(ApiError::InsufficientRole {
                    required: R::NAME,
                    actual: row.role.as_str().to_owned(),
                });
            }

            let user = User {
                id: row.user_id,
                discord_id: row.discord_id,
                discord_username: row.discord_username,
                discord_display_name: row.discord_display_name,
                discord_avatar_hash: row.discord_avatar_hash,
                avatar_url: row.avatar_url,
                role: row.role,
                status: row.status,
                joined_at: row.joined_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };

            Ok(AuthenticatedUser {
                user,
                session_id: row.session_id,
                _phantom: PhantomData,
            })
        }
    }
}

/// Internal row type for session + user join query.
#[derive(Debug)]
struct SessionUserRow {
    session_id: Uuid,
    user_id: Uuid,
    #[allow(dead_code)]
    expires_at: chrono::DateTime<chrono::Utc>,
    discord_id: String,
    discord_username: String,
    discord_display_name: String,
    discord_avatar_hash: Option<String>,
    avatar_url: Option<String>,
    role: UserRole,
    status: UserStatus,
    joined_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}
