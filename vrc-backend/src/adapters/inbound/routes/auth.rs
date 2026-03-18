use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AppState;
use crate::adapters::outbound::discord::client::ReqwestDiscordClient;
use crate::domain::ports::services::discord_client::DiscordClient;

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub struct LoginQuery {
    redirect_to: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

#[derive(Serialize, Deserialize)]
struct OAuthStatePayload {
    nonce: String,
    redirect_to: String,
    expires_at: i64,
}

/// Validate redirect path to prevent open redirects.
fn validate_redirect(path: &str) -> &str {
    if path.starts_with('/')
        && !path.contains("//")
        && !path.contains('\\')
        && path.chars().all(|c| !c.is_control())
    {
        path
    } else {
        "/"
    }
}

/// Sign a payload with HMAC-SHA256.
fn sign_state(payload: &[u8], secret: &str) -> Vec<u8> {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload);
    mac.finalize().into_bytes().to_vec()
}

/// Verify HMAC-SHA256 signature.
fn verify_state(payload: &[u8], signature: &[u8], secret: &str) -> bool {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload);
    mac.verify_slice(signature).is_ok()
}

async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
    jar: CookieJar,
) -> impl IntoResponse {
    let redirect_to = query
        .redirect_to
        .as_deref()
        .map_or("/", |r| validate_redirect(r));

    // Generate cryptographically random nonce
    let mut nonce_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = hex::encode(nonce_bytes);

    let payload = OAuthStatePayload {
        nonce: nonce.clone(),
        redirect_to: redirect_to.to_owned(),
        expires_at: (Utc::now() + chrono::Duration::minutes(10)).timestamp(),
    };

    let payload_json = serde_json::to_vec(&payload).expect("serialization cannot fail");
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_json);
    let signature = sign_state(payload_json.as_slice(), &state.config.session_secret);
    let sig_b64 = URL_SAFE_NO_PAD.encode(&signature);
    let state_token = format!("{payload_b64}.{sig_b64}");

    // Set nonce in HttpOnly cookie for CSRF verification
    let oauth_cookie = Cookie::build(("oauth_state", nonce))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(600));

    let callback_url = format!(
        "{}/api/v1/auth/discord/callback",
        state.config.backend_base_url
    );

    let discord_url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify+guilds&state={}",
        state.config.discord_client_id,
        urlencoding::encode(&callback_url),
        urlencoding::encode(&state_token)
    );

    (jar.add(oauth_cookie), Redirect::temporary(&discord_url))
}

#[allow(clippy::too_many_lines)] // OAuth callback flow is inherently long
async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let frontend = &state.config.frontend_origin;
    let error_redirect = |reason: &str| -> Response {
        let url = format!("{frontend}/auth/error?reason={reason}");
        Redirect::temporary(&url).into_response()
    };

    // 1. Verify state token
    let parts: Vec<&str> = query.state.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(error_redirect("invalid_state"));
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| error_redirect("invalid_state"))?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| error_redirect("invalid_state"))?;

    if !verify_state(&payload_bytes, &sig_bytes, &state.config.session_secret) {
        return Err(error_redirect("invalid_state"));
    }

    let payload: OAuthStatePayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| error_redirect("invalid_state"))?;

    // Check expiry
    if Utc::now().timestamp() > payload.expires_at {
        return Err(error_redirect("expired"));
    }

    // Compare nonce with cookie
    let cookie_nonce = jar
        .get("oauth_state")
        .ok_or_else(|| error_redirect("csrf"))?
        .value()
        .to_owned();

    if cookie_nonce != payload.nonce {
        return Err(error_redirect("csrf"));
    }

    // 2. Exchange code for tokens
    let discord = ReqwestDiscordClient::new(
        state.http_client.clone(),
        state.config.discord_client_id.clone(),
        state.config.discord_client_secret.clone(),
    );

    let callback_url = format!(
        "{}/api/v1/auth/discord/callback",
        state.config.backend_base_url
    );

    let token_response = discord
        .exchange_code(&query.code, &callback_url)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Discord token exchange failed");
            error_redirect("discord_error")
        })?;

    // 3. Fetch user
    let discord_user = discord
        .get_user(&token_response.access_token)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Discord get user failed");
            error_redirect("discord_error")
        })?;

    // 4. Verify guild membership
    let guilds = discord
        .get_user_guilds(&token_response.access_token)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Discord get guilds failed");
            error_redirect("discord_error")
        })?;

    let is_member = guilds.iter().any(|g| g.id == state.config.discord_guild_id);

    if !is_member {
        return Err(error_redirect("not_member"));
    }

    // 5. Upsert user
    let avatar_url = discord_user.avatar_url();
    let user = sqlx::query_as!(
        crate::domain::entities::user::User,
        r#"
        INSERT INTO users (discord_id, discord_username, discord_display_name, discord_avatar_hash, avatar_url)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (discord_id) DO UPDATE SET
            discord_username = EXCLUDED.discord_username,
            discord_display_name = EXCLUDED.discord_display_name,
            discord_avatar_hash = EXCLUDED.discord_avatar_hash,
            avatar_url = EXCLUDED.avatar_url,
            updated_at = NOW()
        RETURNING id, discord_id, discord_username, discord_display_name,
                  discord_avatar_hash, avatar_url,
                  role as "role: crate::domain::entities::user::UserRole",
                  status as "status: crate::domain::entities::user::UserStatus",
                  joined_at, created_at, updated_at
        "#,
        discord_user.id,
        discord_user.username,
        discord_user.display_name(),
        discord_user.avatar.as_deref(),
        avatar_url.as_deref(),
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "User upsert failed");
        error_redirect("discord_error")
    })?;

    // 6. Check suspension
    if user.status == crate::domain::entities::user::UserStatus::Suspended {
        return Err(error_redirect("suspended"));
    }

    // 7. Create session
    let mut raw_token = [0u8; 32];
    rand::rng().fill_bytes(&mut raw_token);
    let token_b64 = URL_SAFE_NO_PAD.encode(raw_token);

    let mut hasher = Sha256::new();
    hasher.update(raw_token);
    let token_hash = hasher.finalize().to_vec();

    // session_max_age_secs is at most ~604800 (7 days), well within f64 precision
    #[allow(clippy::cast_precision_loss)]
    let max_age_f64 = state.config.session_max_age_secs as f64;

    sqlx::query!(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + make_interval(secs => $3::double precision))
        "#,
        user.id,
        &token_hash[..],
        max_age_f64
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Session creation failed");
        error_redirect("discord_error")
    })?;

    // 8. Clear oauth_state cookie and set session cookie
    let remove_oauth = Cookie::build(("oauth_state", ""))
        .http_only(true)
        .path("/")
        .max_age(time::Duration::ZERO);

    let session_cookie = Cookie::build(("session_id", token_b64))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(state.config.session_max_age_secs));

    let redirect_to = validate_redirect(&payload.redirect_to);
    let redirect_url = format!("{frontend}{redirect_to}");

    Ok((
        jar.add(session_cookie).remove(remove_oauth),
        Redirect::temporary(&redirect_url),
    )
        .into_response())
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_normal_path() {
        assert_eq!(validate_redirect("/dashboard"), "/dashboard");
    }

    #[test]
    fn test_validate_redirect_root() {
        assert_eq!(validate_redirect("/"), "/");
    }

    #[test]
    fn test_validate_redirect_nested_path() {
        assert_eq!(validate_redirect("/profile/edit"), "/profile/edit");
    }

    #[test]
    fn test_validate_redirect_rejects_protocol_relative() {
        assert_eq!(validate_redirect("//evil.com"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_absolute_url() {
        assert_eq!(validate_redirect("https://evil.com"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_backslash() {
        assert_eq!(validate_redirect("/foo\\bar"), "/");
    }

    #[test]
    fn test_validate_redirect_rejects_control_chars() {
        assert_eq!(validate_redirect("/foo\nbar"), "/");
    }

    #[test]
    fn test_validate_redirect_empty_string() {
        assert_eq!(validate_redirect(""), "/");
    }

    #[test]
    fn test_validate_redirect_just_text() {
        assert_eq!(validate_redirect("notapath"), "/");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// P6: Redirect validation always returns a safe relative path.
        #[test]
        fn redirect_never_returns_absolute_url(input in "\\PC{0,200}") {
            let result = validate_redirect(&input);
            // Must start with '/' (relative) or be the default "/"
            prop_assert!(result.starts_with('/'));
            // Must not contain protocol-relative patterns
            prop_assert!(!result.contains("//"));
            prop_assert!(!result.contains('\\'));
        }
    }
}
