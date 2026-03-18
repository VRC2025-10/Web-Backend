/// Integration tests for the VRC Backend API.
///
/// These tests require a running PostgreSQL instance.
/// Set `DATABASE_URL` env var or use the default test database:
///   postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend
///
/// Run with: `cargo test --test api_tests -- --test-threads=1`
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::config::AppConfig;

// ===== Test helpers =====

/// Create a fresh pool for each test. Each test run uses unique discord IDs
/// so no TRUNCATE is needed between tests.
async fn setup_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend".into());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test DB");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

fn test_config() -> AppConfig {
    AppConfig {
        bind_address: "127.0.0.1:0".to_owned(),
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend".into()
        }),
        database_max_connections: 5,
        discord_client_id: "test_client_id".to_owned(),
        discord_client_secret: "test_client_secret".to_owned(),
        discord_guild_id: "test_guild_id".to_owned(),
        backend_base_url: "http://localhost:3000".to_owned(),
        frontend_origin: "http://localhost:5173".to_owned(),
        session_secret: "test_secret_key_at_least_32_bytes_long".to_owned(),
        system_api_token: "test_system_token_at_least_32_chars_long".to_owned(),
        session_max_age_secs: 604_800,
        session_cleanup_interval_secs: 3600,
        super_admin_discord_id: None,
        discord_webhook_url: None,
        cookie_secure: false,
    }
}

fn build_app(pool: PgPool) -> axum::Router {
    // Install metrics recorder if not already installed.
    // Use build_recorder() to avoid starting a background HTTP listener in tests.
    static METRICS_INIT: std::sync::Once = std::sync::Once::new();
    METRICS_INIT.call_once(|| {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        let _ = metrics::set_global_recorder(recorder);
        let _ = vrc_backend::METRICS_HANDLE.set(handle);
    });

    let state = Arc::new(AppState {
        db_pool: pool,
        http_client: reqwest::Client::new(),
        config: test_config(),
        start_time: Instant::now(),
        webhook: None,
    });
    routes::build_router(state)
}

/// Generate a unique discord ID for test isolation.
fn unique_discord_id() -> String {
    Uuid::new_v4().to_string().replace('-', "")[..18].to_owned()
}

async fn create_test_user(pool: &PgPool, discord_id: &str, role: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (discord_id, discord_username, discord_display_name, role, status)
         VALUES ($1, $2, $3, $4::user_role, 'active')
         RETURNING id",
    )
    .bind(discord_id)
    .bind("TestUser")
    .bind(format!("TestUser_{discord_id}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
}

async fn create_test_session(pool: &PgPool, user_id: Uuid) -> String {
    let mut raw_token = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut raw_token);
    let cookie_value = URL_SAFE_NO_PAD.encode(raw_token);

    let mut hasher = Sha256::new();
    hasher.update(raw_token);
    let token_hash = hasher.finalize().to_vec();

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')",
    )
    .bind(user_id)
    .bind(&token_hash[..])
    .execute(pool)
    .await
    .expect("Failed to create test session");

    cookie_value
}

async fn parse_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");
    serde_json::from_slice(&body).expect("Failed to parse JSON")
}

// ===== Tests =====

#[tokio::test]
async fn test_health_returns_ok() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    let response = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["database"], "connected");
}

#[tokio::test]
async fn test_auth_me_without_session_returns_401() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = parse_json(response).await;
    assert_eq!(body["error"], "ERR-AUTH-003");
}

#[tokio::test]
async fn test_auth_me_returns_user_info() {
    let pool = setup_pool().await;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await;
    let session = create_test_session(&pool, user_id).await;
    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["user"]["discord_id"], did);
    assert_eq!(body["user"]["role"], "member");
}

#[tokio::test]
async fn test_suspended_user_returns_403() {
    let pool = setup_pool().await;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await;
    let session = create_test_session(&pool, user_id).await;

    // Suspend the user
    sqlx::query("UPDATE users SET status = 'suspended' WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = parse_json(response).await;
    assert_eq!(body["error"], "ERR-AUTH-004");
}

#[tokio::test]
async fn test_update_profile_success() {
    let pool = setup_pool().await;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await;
    let session = create_test_session(&pool, user_id).await;
    let app = build_app(pool);

    let body = serde_json::json!({
        "nickname": "テストユーザー",
        "vrc_id": "usr_12345678-1234-1234-1234-123456789abc",
        "bio_markdown": "# Hello\nWorld",
        "is_public": true
    });

    let response = app
        .oneshot(
            Request::put("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let resp = parse_json(response).await;
    assert_eq!(status, StatusCode::OK, "Profile update failed: {resp}");
    assert_eq!(resp["nickname"], "テストユーザー");
    assert!(
        resp["bio_html"]
            .as_str()
            .unwrap()
            .contains("<h1>Hello</h1>")
    );
    assert_eq!(resp["is_public"], true);
}

#[tokio::test]
async fn test_update_profile_invalid_vrc_id() {
    let pool = setup_pool().await;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await;
    let session = create_test_session(&pool, user_id).await;
    let app = build_app(pool);

    let body = serde_json::json!({
        "vrc_id": "invalid_id_format",
        "is_public": true
    });

    let response = app
        .oneshot(
            Request::put("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let resp = parse_json(response).await;
    assert_eq!(resp["error"], "ERR-PROF-001");
    assert!(resp["details"]["vrc_id"].is_string());
}

#[tokio::test]
async fn test_csrf_blocks_post_without_origin() {
    let pool = setup_pool().await;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await;
    let session = create_test_session(&pool, user_id).await;
    let app = build_app(pool);

    let body = serde_json::json!({
        "target_type": "profile",
        "target_id": "00000000-0000-0000-0000-000000000001",
        "reason": "This is a test report with enough characters"
    });

    let response = app
        .oneshot(
            Request::post("/api/v1/internal/reports")
                .header("Cookie", format!("session_id={session}"))
                // No Origin header — should be rejected by CSRF
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let resp = parse_json(response).await;
    assert_eq!(resp["error"], "ERR-CSRF-001");
}

#[tokio::test]
async fn test_public_events_no_auth_required() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    let response = app
        .oneshot(
            Request::get("/api/v1/public/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body["items"].is_array());
    assert_eq!(body["total_count"], 0);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    // Metrics endpoint now requires Bearer token authentication
    let response = app
        .oneshot(
            Request::get("/metrics")
                .header(
                    "authorization",
                    "Bearer test_system_token_at_least_32_chars_long",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint_unauthorized() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    // Metrics endpoint without token should return 401
    let response = app
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_security_headers_present() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    let response = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        headers.get("referrer-policy").unwrap(),
        "strict-origin-when-cross-origin"
    );
    assert!(headers.get("strict-transport-security").is_some());
    assert!(headers.get("content-security-policy").is_some());
    assert!(headers.get("permissions-policy").is_some());
}

#[tokio::test]
async fn test_system_endpoint_requires_bearer_token() {
    let pool = setup_pool().await;
    let app = build_app(pool);

    let body = serde_json::json!({
        "discord_id": "leaving_user_123"
    });

    let response = app
        .oneshot(
            Request::post("/api/v1/system/sync/users/leave")
                // No Authorization header
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
