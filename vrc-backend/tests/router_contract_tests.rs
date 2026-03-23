/// Router-level contract tests.
/// Spec refs:
/// - auth-api.md
/// - internal-api.md
/// - application-security.md
/// - error-handling.md
/// Coverage:
/// - OAuth login redirect and signed state payload
/// - Internal API CSRF/cache/security header behavior without a database round trip
/// - Metrics endpoint auth and content-type contract
use std::sync::Arc;
use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use secrecy::SecretString;
use tower::ServiceExt;
use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::config::AppConfig;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn test_config() -> AppConfig {
    AppConfig {
        bind_address: "127.0.0.1:0".to_owned(),
        database_url: SecretString::from("postgres://test:test@localhost/test".to_owned()),
        database_max_connections: 5,
        discord_client_id: "discord-client-id".to_owned(),
        discord_client_secret: SecretString::from(
            "0123456789abcdef0123456789abcdef".to_owned(),
        ),
        discord_guild_id: "guild-id".to_owned(),
        backend_base_url: "https://backend.example".to_owned(),
        frontend_origin: "https://frontend.example".to_owned(),
        frontend_origin_header: "https://frontend.example".parse().expect("valid header"),
        gallery_storage_dir: std::env::temp_dir().join("vrc-gallery-test-router"),
        gallery_max_upload_bytes: 10 * 1024 * 1024,
        session_secret: SecretString::from("abcdefghijklmnopqrstuvwxyz012345".to_owned()),
        system_api_token: SecretString::from(
            "0123456789abcdefghijklmnopqrstuvwxyz".to_owned(),
        ),
        session_max_age_secs: 604_800,
        session_cleanup_interval_secs: 3600,
        event_archival_interval_secs: 3600,
        super_admin_discord_id: None,
        discord_webhook_url: None,
        cookie_secure: false,
        trust_x_forwarded_for: false,
    }
}

fn build_app() -> TestResult<axum::Router> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost/test")?;
    let state = Arc::new(AppState {
        db_pool: pool,
        http_client: reqwest::Client::new(),
        config: test_config(),
        start_time: Instant::now(),
        webhook: None,
    });

    Ok(routes::build_router(state)?)
}

fn decode_state_payload(location: &str) -> TestResult<serde_json::Value> {
    let location = reqwest::Url::parse(location)?;
    let state = location
        .query_pairs()
        .find_map(|(key, value)| (key == "state").then(|| value.into_owned()))
        .ok_or("missing state query parameter")?;
    let payload_b64 = state.split('.').next().ok_or("missing state payload")?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64)?;

    Ok(serde_json::from_slice(&payload)?)
}

async fn parse_json(response: axum::http::Response<Body>) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?)
}

#[tokio::test]
async fn test_login_redirect_sets_oauth_cookie_and_signed_state() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/auth/discord/login?redirect_to=/dashboard")
                .body(Body::empty())?,
        )
        .await?;

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing location header")?;
    assert!(location.starts_with("https://discord.com/oauth2/authorize"));

    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing set-cookie header")?;
    assert!(cookie.contains("oauth_state="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Max-Age=600"));

    let payload = decode_state_payload(location)?;
    assert_eq!(payload["redirect_to"], "/dashboard");
    assert!(payload["nonce"].as_str().is_some_and(|nonce| nonce.len() == 64));
    assert!(payload["expires_at"].as_i64().is_some());
    Ok(())
}

#[tokio::test]
async fn test_login_redirect_sanitizes_invalid_redirect_target() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/auth/discord/login?redirect_to=https://evil.example")
                .body(Body::empty())?,
        )
        .await?;

    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing location header")?;
    let payload = decode_state_payload(location)?;

    assert_eq!(payload["redirect_to"], "/");
    Ok(())
}

#[tokio::test]
async fn test_internal_get_without_session_has_private_cache_and_security_headers() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/internal/auth/me")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("private, no-store")
    );
    assert_eq!(
        response
            .headers()
            .get("x-content-type-options")
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.len() == 26),
        true
    );

    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-AUTH-003");
    Ok(())
}

#[tokio::test]
async fn test_internal_post_without_origin_fails_csrf_before_auth_lookup() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/auth/logout")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-CSRF-001");
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint_requires_bearer_token() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/metrics")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint_accepts_valid_bearer_token() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/metrics")
                .header("authorization", "Bearer 0123456789abcdefghijklmnopqrstuvwxyz")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; version=0.0.4; charset=utf-8")
    );
    Ok(())
}