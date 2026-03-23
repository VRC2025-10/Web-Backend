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
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::AppState;
use crate::adapters::outbound::discord::client::ReqwestDiscordClient;
use crate::domain::ports::services::discord_client::DiscordClient;
use crate::errors::api::ApiError;

use secrecy::ExposeSecret;

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
fn sign_state(payload: &[u8], secret: &str) -> Result<Vec<u8>, ApiError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|error| {
        tracing::error!(error = %error, "Failed to initialize OAuth state signer");
        ApiError::Internal("Failed to initialize OAuth state signer".to_owned())
    })?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Verify HMAC-SHA256 signature.
fn verify_state(payload: &[u8], signature: &[u8], secret: &str) -> Result<bool, ApiError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|error| {
        tracing::error!(error = %error, "Failed to initialize OAuth state verifier");
        ApiError::Internal("Failed to initialize OAuth state verifier".to_owned())
    })?;
    mac.update(payload);
    Ok(mac.verify_slice(signature).is_ok())
}

fn expired_oauth_state_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build(("oauth_state", ""))
        .http_only(true)
        .secure(secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
}

fn map_login_error_code(reason: &str) -> &'static str {
    match reason {
        "invalid_state" | "expired" | "csrf" => "csrf_failed",
        "not_member" => "not_guild_member",
        "discord_error" => "discord_error",
        "suspended" => "suspended",
        _ => "auth_failed",
    }
}

fn build_login_error_redirect_url(frontend_origin: &str, reason: &str) -> String {
    let error_code = map_login_error_code(reason);
    format!("{frontend_origin}/login?error={error_code}")
}

#[vrc_macros::handler(method = GET, path = "/api/v1/auth/discord/login", rate_limit = "auth", summary = "Start Discord OAuth login")]
async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiError> {
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

    let payload_json = serde_json::to_vec(&payload).map_err(|error| {
        tracing::error!(error = %error, "Failed to serialize OAuth state payload");
        ApiError::Internal("Failed to serialize OAuth state payload".to_owned())
    })?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_json);
    let signature = sign_state(payload_json.as_slice(), state.config.session_secret.expose_secret())?;
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

    Ok((jar.add(oauth_cookie), Redirect::temporary(&discord_url)))
}

#[vrc_macros::handler(method = GET, path = "/api/v1/auth/discord/callback", rate_limit = "auth", summary = "Handle Discord OAuth callback")]
#[allow(clippy::too_many_lines)] // OAuth callback flow is inherently long
async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<Response, ApiError> {
    let frontend = &state.config.frontend_origin;
    let cookie_secure = state.config.cookie_secure;
    let error_redirect = |reason: &str| -> Response {
        let url = build_login_error_redirect_url(frontend, reason);
        (
            jar.clone()
                .remove(expired_oauth_state_cookie(cookie_secure)),
            Redirect::temporary(&url),
        )
            .into_response()
    };

    // 1. Verify state token
    let parts: Vec<&str> = query.state.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Ok(error_redirect("invalid_state"));
    }

    let Ok(payload_bytes) = URL_SAFE_NO_PAD.decode(parts[0]) else {
        return Ok(error_redirect("invalid_state"));
    };
    let Ok(sig_bytes) = URL_SAFE_NO_PAD.decode(parts[1]) else {
        return Ok(error_redirect("invalid_state"));
    };

    if !verify_state(&payload_bytes, &sig_bytes, state.config.session_secret.expose_secret())? {
        return Ok(error_redirect("invalid_state"));
    }

    let payload: OAuthStatePayload = match serde_json::from_slice(&payload_bytes) {
        Ok(payload) => payload,
        Err(_) => return Ok(error_redirect("invalid_state")),
    };

    // Check expiry
    if Utc::now().timestamp() > payload.expires_at {
        return Ok(error_redirect("expired"));
    }

    // Compare nonce with cookie
    let cookie_nonce = jar
        .get("oauth_state")
        .map(|cookie| cookie.value().to_owned())
        .unwrap_or_default();

    if cookie_nonce.is_empty() {
        return Ok(error_redirect("csrf"));
    }

    if !bool::from(cookie_nonce.as_bytes().ct_eq(payload.nonce.as_bytes())) {
        return Ok(error_redirect("csrf"));
    }

    // 2. Exchange code for tokens
    let discord = ReqwestDiscordClient::new(
        state.http_client.clone(),
        state.config.discord_client_id.clone(),
        state.config.discord_client_secret.expose_secret().to_owned(),
    );

    let callback_url = format!(
        "{}/api/v1/auth/discord/callback",
        state.config.backend_base_url
    );

    let token_response = discord.exchange_code(&query.code, &callback_url).await;
    let token_response = match token_response {
        Ok(token_response) => token_response,
        Err(error) => {
            tracing::error!(error = %error, "Discord token exchange failed");
            return Ok(error_redirect("discord_error"));
        }
    };

    // 3. Fetch user
    let discord_user = discord.get_user(&token_response.access_token).await;
    let discord_user = match discord_user {
        Ok(discord_user) => discord_user,
        Err(error) => {
            tracing::error!(error = %error, "Discord get user failed");
            return Ok(error_redirect("discord_error"));
        }
    };

    // 4. Verify guild membership
    let guilds = discord.get_user_guilds(&token_response.access_token).await;
    let guilds = match guilds {
        Ok(guilds) => guilds,
        Err(error) => {
            tracing::error!(error = %error, "Discord get guilds failed");
            return Ok(error_redirect("discord_error"));
        }
    };

    let is_member = guilds.iter().any(|g| g.id == state.config.discord_guild_id);

    if !is_member {
        return Ok(error_redirect("not_member"));
    }

    // 5. Upsert user
    let avatar_url = discord_user.avatar_url();
    let is_super_admin = state.config.is_super_admin_discord_id(&discord_user.id);
    let user = sqlx::query_as!(
        crate::domain::entities::user::User,
        r#"
        INSERT INTO users (
            discord_id,
            discord_username,
            discord_display_name,
            discord_avatar_hash,
            avatar_url,
            role
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            CASE
                WHEN $6 THEN 'super_admin'::user_role
                ELSE 'member'::user_role
            END
        )
        ON CONFLICT (discord_id) DO UPDATE SET
            discord_username = EXCLUDED.discord_username,
            discord_display_name = EXCLUDED.discord_display_name,
            discord_avatar_hash = EXCLUDED.discord_avatar_hash,
            avatar_url = EXCLUDED.avatar_url,
            role = CASE
                WHEN $6 THEN 'super_admin'::user_role
                ELSE users.role
            END,
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
        is_super_admin,
    )
    .fetch_one(&state.db_pool)
    .await;
    let user = match user {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "User upsert failed");
            return Ok(error_redirect("discord_error"));
        }
    };

    // 6. Check suspension
    if user.status == crate::domain::entities::user::UserStatus::Suspended {
        return Ok(error_redirect("suspended"));
    }

    // 7. Create session
    let mut raw_token = [0u8; 32];
    rand::rng().fill_bytes(&mut raw_token);
    let token_b64 = URL_SAFE_NO_PAD.encode(raw_token);

    let token_hash = crate::auth::crypto::sha256_hash(&raw_token);

    // session_max_age_secs is at most ~604800 (7 days), well within f64 precision
    #[allow(clippy::cast_precision_loss)]
    let max_age_f64 = state.config.session_max_age_secs as f64;

    let session_creation = sqlx::query!(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + make_interval(secs => $3::double precision))
        "#,
        user.id,
        &token_hash[..],
        max_age_f64
    )
    .execute(&state.db_pool)
    .await;
    if let Err(error) = session_creation {
        tracing::error!(error = %error, "Session creation failed");
        return Ok(error_redirect("discord_error"));
    }

    // 8. Clear oauth_state cookie and set session cookie
    let mut session_cookie = Cookie::build(("session_id", token_b64))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(state.config.session_max_age_secs));

    if let Some(cookie_domain) = state.config.cookie_domain.clone() {
        session_cookie = session_cookie.domain(cookie_domain);
    }

    let session_cookie = session_cookie.build();

    let redirect_to = validate_redirect(&payload.redirect_to);
    let redirect_url = format!("{frontend}{redirect_to}");

    Ok((
        jar.add(session_cookie)
            .remove(expired_oauth_state_cookie(state.config.cookie_secure)),
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

    // Spec refs: auth-api.md "State Token" and authentication-design.md.
    // Coverage: redirect validation, state signing/verification, and cookie expiry contract.

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

    #[test]
    fn test_validate_redirect_accepts_relative_query_string() {
        assert_eq!(validate_redirect("/events?page=2"), "/events?page=2");
    }

    #[test]
    fn test_sign_and_verify_state_round_trip() {
        let payload = br#"{"nonce":"abc","redirect_to":"/dashboard","expires_at":42}"#;
        let signature = sign_state(payload, "abcdefghijklmnopqrstuvwxyz012345")
            .expect("signing must succeed");

        let verified = verify_state(payload, &signature, "abcdefghijklmnopqrstuvwxyz012345")
            .expect("verification must succeed");

        assert!(verified);
    }

    #[test]
    fn test_verify_state_rejects_tampered_payload() {
        let payload = br#"{"nonce":"abc","redirect_to":"/dashboard","expires_at":42}"#;
        let signature = sign_state(payload, "abcdefghijklmnopqrstuvwxyz012345")
            .expect("signing must succeed");

        let verified = verify_state(
            br#"{"nonce":"abc","redirect_to":"/admin","expires_at":42}"#,
            &signature,
            "abcdefghijklmnopqrstuvwxyz012345",
        )
        .expect("verification must succeed");

        assert!(!verified);
    }

    #[test]
    fn test_verify_state_rejects_wrong_secret() {
        let payload = br#"{"nonce":"abc","redirect_to":"/dashboard","expires_at":42}"#;
        let signature = sign_state(payload, "abcdefghijklmnopqrstuvwxyz012345")
            .expect("signing must succeed");

        let verified = verify_state(payload, &signature, "012345abcdefghijklmnopqrstuvwxyz")
            .expect("verification must succeed");

        assert!(!verified);
    }

    #[test]
    fn test_expired_oauth_state_cookie_has_expected_security_attributes() {
        let cookie = expired_oauth_state_cookie(true);

        assert_eq!(cookie.name(), "oauth_state");
        assert_eq!(cookie.value(), "");
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(
            cookie.same_site(),
            Some(axum_extra::extract::cookie::SameSite::Lax)
        );
        assert_eq!(cookie.max_age(), Some(time::Duration::ZERO));
    }

    #[test]
    fn test_map_login_error_code_normalizes_callback_failures() {
        assert_eq!(map_login_error_code("invalid_state"), "csrf_failed");
        assert_eq!(map_login_error_code("expired"), "csrf_failed");
        assert_eq!(map_login_error_code("csrf"), "csrf_failed");
        assert_eq!(map_login_error_code("not_member"), "not_guild_member");
        assert_eq!(map_login_error_code("discord_error"), "discord_error");
        assert_eq!(map_login_error_code("suspended"), "suspended");
        assert_eq!(map_login_error_code("unexpected"), "auth_failed");
    }

    #[test]
    fn test_build_login_error_redirect_url_targets_existing_login_route() {
        assert_eq!(
            build_login_error_redirect_url("https://frontend.example", "not_member"),
            "https://frontend.example/login?error=not_guild_member"
        );
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

// Kani formal verification harness for redirect validation.
// Run with: cargo kani --harness proof_redirect_always_relative
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// P3: Redirect validation always produces a safe relative path.
    /// For any byte sequence that is valid UTF-8, the output starts with '/',
    /// contains no doubled slashes, and no backslashes.
    #[kani::proof]
    #[kani::unwind(34)] // Bound string length + 2 for termination
    fn proof_redirect_always_relative() {
        let len: usize = kani::any();
        kani::assume(len <= 32);

        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(kani::any::<u8>());
        }

        if let Ok(s) = std::str::from_utf8(&input) {
            let result = validate_redirect(s);
            assert!(result.starts_with('/'));
            assert!(!result.contains("//"));
            assert!(!result.contains('\\'));
            if result.len() >= 2 {
                assert!(result.as_bytes()[1] != b'/');
            }
        }
    }
}
