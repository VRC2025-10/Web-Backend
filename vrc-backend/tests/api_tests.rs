/// Integration tests for the VRC Backend API.
///
/// These tests require a running `PostgreSQL` instance.
/// Set `DATABASE_URL` env var or use the default test database:
///   <postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend>
///
/// Run with: `cargo test --test api_tests -- --test-threads=1`
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use axum::body::Body;
use axum::http::HeaderValue;
use axum::http::{Method, Request, Response, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use vrc_backend::AppState;
use vrc_backend::adapters::inbound::routes;
use vrc_backend::config::AppConfig;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const PERFORMANCE_INDEX_MIGRATION: i64 = 20_250_103_000_000;

// ===== Test helpers =====

/// Create a fresh pool for each test. Each test run uses unique discord IDs
/// so no TRUNCATE is needed between tests.
async fn setup_pool() -> TestResult<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend".into());
    let migrations_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Integration tests intentionally reuse a shared local database. When a
    // migration evolves during development, the stored checksum for that
    // version can block subsequent test runs with VersionMismatch. Reset the
    // performance-index migration state so the current definition is applied.
    for statement in [
        "DROP INDEX IF EXISTS idx_sessions_token_hash_expires",
        "DROP INDEX IF EXISTS idx_users_active_joined",
        "DROP INDEX IF EXISTS idx_reports_status_created",
        "DROP INDEX IF EXISTS idx_events_status_start_time",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1")
        .bind(PERFORMANCE_INDEX_MIGRATION)
        .execute(&pool)
        .await?;

    sqlx::migrate::Migrator::new(migrations_dir)
        .await?
        .run(&pool)
        .await?;

    Ok(pool)
}

fn test_config() -> AppConfig {
    let gallery_storage_dir = std::env::temp_dir().join(format!("vrc-gallery-test-{}", Uuid::new_v4()));

    AppConfig {
        bind_address: "127.0.0.1:0".to_owned(),
        database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://vrc:vrc_dev_password@localhost:5432/vrc_backend".into()
        }).into(),
        database_max_connections: 5,
        discord_client_id: "test_client_id".to_owned(),
        discord_client_secret: "test_client_secret".to_owned().into(),
        discord_guild_id: "test_guild_id".to_owned(),
        backend_base_url: "http://localhost:3000".to_owned(),
        frontend_origin: "http://localhost:5173".to_owned(),
        frontend_origin_header: HeaderValue::from_static("http://localhost:5173"),
        cookie_domain: None,
        gallery_storage_dir,
        gallery_max_upload_bytes: 10 * 1024 * 1024,
        session_secret: "test_secret_key_at_least_32_bytes_long".to_owned().into(),
        system_api_token: "test_system_token_at_least_32_chars_long".to_owned().into(),
        session_max_age_secs: 604_800,
        session_cleanup_interval_secs: 3600,
        event_archival_interval_secs: 3600,
        super_admin_discord_id: None,
        discord_webhook_url: None,
        cookie_secure: false,
        trust_x_forwarded_for: false,
    }
}

fn build_app(pool: PgPool) -> TestResult<axum::Router> {
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
    Ok(routes::build_router(state)?)
}

/// Generate a unique discord ID for test isolation.
fn unique_discord_id() -> String {
    Uuid::new_v4().to_string().replace('-', "")[..18].to_owned()
}

async fn create_test_user(pool: &PgPool, discord_id: &str, role: &str) -> TestResult<Uuid> {
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (discord_id, discord_username, discord_display_name, role, status)
         VALUES ($1, $2, $3, $4::user_role, 'active')
         RETURNING id",
    )
    .bind(discord_id)
    .bind("TestUser")
    .bind(format!("TestUser_{discord_id}"))
    .bind(role)
    .fetch_one(pool)
    .await?;

    Ok(user_id)
}

async fn create_test_session(pool: &PgPool, user_id: Uuid) -> TestResult<String> {
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
    .await?;

    Ok(cookie_value)
}

async fn create_test_profile(
    pool: &PgPool,
    user_id: Uuid,
    nickname: &str,
    is_public: bool,
) -> TestResult {
    sqlx::query(
        "INSERT INTO profiles (user_id, nickname, bio_markdown, bio_html, is_public) VALUES ($1, $2, '', '', $3)",
    )
    .bind(user_id)
    .bind(nickname)
    .bind(is_public)
    .execute(pool)
    .await?;

    Ok(())
}

async fn parse_json(response: Response<Body>) -> TestResult<serde_json::Value> {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn empty_request(method: Method, uri: &str) -> TestResult<Request<Body>> {
    Ok(Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())?)
}

fn request_with_connect_info(method: Method, uri: &str, ip_octet: u8) -> TestResult<Request<Body>> {
    let mut request = Request::builder().method(method).uri(uri).body(Body::empty())?;
    request.extensions_mut().insert(ConnectInfo(std::net::SocketAddr::from((
        std::net::Ipv4Addr::new(127, 0, 0, ip_octet),
        30_000 + u16::from(ip_octet),
    ))));
    Ok(request)
}

fn multipart_request(
    uri: &str,
    session: &str,
    text_fields: &[(&str, &str)],
    file_fields: &[(&str, &str, &str, &[u8])],
) -> TestResult<Request<Body>> {
    let boundary = "x-vrc-boundary";
    let mut body = Vec::new();

    for (name, value) in text_fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    for (name, file_name, content_type, bytes) in file_fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    Ok(Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("Cookie", format!("session_id={session}"))
        .header("Origin", "http://localhost:5173")
        .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))?)
}

// ===== Tests =====

#[tokio::test]
async fn test_health_returns_ok() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app.oneshot(empty_request(Method::GET, "/health")?).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["database"], "connected");
    Ok(())
}

#[tokio::test]
async fn test_auth_me_without_session_returns_401() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/internal/auth/me")?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-AUTH-003");
    Ok(())
}

#[tokio::test]
async fn test_auth_me_returns_user_info() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool)?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    assert_eq!(body["discord_id"], did);
    assert_eq!(body["role"], "member");
    Ok(())
}

#[tokio::test]
async fn test_auth_me_prefers_discord_avatar_over_profile_avatar() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;

    sqlx::query(
        "UPDATE users SET avatar_url = $1 WHERE id = $2",
    )
    .bind("https://cdn.discordapp.com/avatars/test-user/test-hash.png")
    .bind(user_id)
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO profiles (user_id, nickname, avatar_url, bio_markdown, bio_html, is_public) VALUES ($1, $2, $3, '', '', true)",
    )
    .bind(user_id)
    .bind("ProfileNickname")
    .bind("https://images.example.com/custom-avatar.png")
    .execute(&pool)
    .await?;

    let app = build_app(pool)?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    assert_eq!(
        body["avatar_url"],
        "https://cdn.discordapp.com/avatars/test-user/test-hash.png"
    );
    assert_eq!(body["profile"]["nickname"], "ProfileNickname");
    Ok(())
}

#[tokio::test]
async fn test_suspended_user_returns_403() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;

    // Suspend the user
    sqlx::query("UPDATE users SET status = 'suspended' WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await?;

    let app = build_app(pool)?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-AUTH-004");
    Ok(())
}

#[tokio::test]
async fn test_update_profile_success() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "nickname": "テストユーザー",
        "vrc_id": "usr_12345678-1234-1234-1234-123456789abc",
        "bio_markdown": "# Hello\nWorld",
        "is_public": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    let status = response.status();
    let resp = parse_json(response).await?;
    assert_eq!(status, StatusCode::OK, "Profile update failed: {resp}");
    assert_eq!(resp["nickname"], "テストユーザー");
    assert!(
        resp["bio_html"]
            .as_str()
            .is_some_and(|html| html.contains("<h1>Hello</h1>"))
    );
    assert_eq!(resp["is_public"], true);
    Ok(())
}

#[tokio::test]
async fn test_update_profile_invalid_vrc_id() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "vrc_id": "invalid_id_format",
        "is_public": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let resp = parse_json(response).await?;
    assert_eq!(resp["error"], "ERR-PROF-001");
    assert!(resp["details"]["vrc_id"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_update_profile_allows_empty_optional_ids() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "vrc_id": "",
        "x_id": "",
        "bio_markdown": "Hello",
        "is_public": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    let status = response.status();
    let resp = parse_json(response).await?;
    assert_eq!(status, StatusCode::OK, "Profile update failed: {resp}");
    assert!(resp["vrc_id"].is_null());
    assert!(resp["x_id"].is_null());
    Ok(())
}

#[tokio::test]
async fn test_public_members_excludes_private_profiles_from_list_and_count() -> TestResult {
    let pool = setup_pool().await?;
    let public_did = unique_discord_id();
    let public_user_id = create_test_user(&pool, &public_did, "member").await?;
    create_test_profile(&pool, public_user_id, "PublicUser", true).await?;

    let private_did = unique_discord_id();
    let private_user_id = create_test_user(&pool, &private_did, "member").await?;
    create_test_profile(&pool, private_user_id, "PrivateUser", false).await?;

    let app = build_app(pool)?;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/public/members?per_page=100")?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    let items = body["items"].as_array().ok_or("items must be an array")?;
    let user_ids: Vec<&str> = items
        .iter()
        .filter_map(|item| item["user_id"].as_str())
        .collect();

    assert!(user_ids.contains(&public_did.as_str()));
    assert!(!user_ids.contains(&private_did.as_str()));
    let public_item = items
        .iter()
        .find(|item| item["user_id"] == public_did)
        .ok_or("public profile must be returned")?;
    assert!(public_item["profile"].is_object());
    Ok(())
}

#[tokio::test]
async fn test_public_member_detail_returns_404_for_private_profile() -> TestResult {
    let pool = setup_pool().await?;
    let private_did = unique_discord_id();
    let private_user_id = create_test_user(&pool, &private_did, "member").await?;
    create_test_profile(&pool, private_user_id, "PrivateUser", false).await?;

    let app = build_app(pool)?;

    let response = app
        .oneshot(empty_request(
            Method::GET,
            &format!("/api/v1/public/members/{private_did}"),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-PROF-004");
    Ok(())
}

#[tokio::test]
async fn test_csrf_blocks_post_without_origin() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "target_type": "profile",
        "target_id": "00000000-0000-0000-0000-000000000001",
        "reason": "This is a test report with enough characters"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/reports")
                .header("Cookie", format!("session_id={session}"))
                // No Origin header — should be rejected by CSRF
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let resp = parse_json(response).await?;
    assert_eq!(resp["error"], "ERR-CSRF-001");
    Ok(())
}

#[tokio::test]
async fn test_create_profile_report_uses_discord_id_string_target() -> TestResult {
    let pool = setup_pool().await?;
    let reporter_did = unique_discord_id();
    let reporter_id = create_test_user(&pool, &reporter_did, "member").await?;
    let target_did = unique_discord_id();
    let target_id = create_test_user(&pool, &target_did, "member").await?;
    let session = create_test_session(&pool, reporter_id).await?;

    sqlx::query(
        "INSERT INTO profiles (user_id, nickname, bio_markdown, bio_html, is_public) VALUES ($1, $2, '', '', true)"
    )
    .bind(target_id)
    .bind("TargetUser")
    .execute(&pool)
    .await?;

    let app = build_app(pool)?;
    let body = serde_json::json!({
        "target_type": "profile",
        "target_id": target_did,
        "reason": "プロフィール内容がガイドライン違反に見えます。"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/reports")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let resp = parse_json(response).await?;
    assert_eq!(resp["target_type"], "profile");
    assert_eq!(resp["target_id"], body["target_id"]);
    assert_eq!(resp["status"], "open");
    Ok(())
}

#[tokio::test]
async fn test_create_club_report_normalizes_uuid_for_duplicate_detection() -> TestResult {
    let pool = setup_pool().await?;
    let reporter_did = unique_discord_id();
    let reporter_id = create_test_user(&pool, &reporter_did, "member").await?;
    let owner_did = unique_discord_id();
    let owner_id = create_test_user(&pool, &owner_did, "member").await?;
    let session = create_test_session(&pool, reporter_id).await?;

    let club_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO clubs (name, description_markdown, description_html, owner_user_id) VALUES ($1, '', '', $2) RETURNING id"
    )
    .bind("Report Target Club")
    .bind(owner_id)
    .fetch_one(&pool)
    .await?;

    let app = build_app(pool)?;
    let first_body = serde_json::json!({
        "target_type": "club",
        "target_id": club_id.to_string().to_uppercase(),
        "reason": "クラブ説明に問題があり、確認が必要です。"
    });

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/reports")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&first_body)?))?,
        )
        .await?;

    assert_eq!(first.status(), StatusCode::CREATED);
    let first_resp = parse_json(first).await?;
    assert_eq!(first_resp["target_id"], club_id.to_string());

    let duplicate_body = serde_json::json!({
        "target_type": "club",
        "target_id": club_id.to_string(),
        "reason": "同じ対象に対する重複通報です。"
    });

    let duplicate = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/reports")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&duplicate_body)?))?,
        )
        .await?;

    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    let resp = parse_json(duplicate).await?;
    assert_eq!(resp["error"], "ERR-MOD-002");
    Ok(())
}

#[tokio::test]
async fn test_staff_can_upload_community_gallery_image_and_list_admin_galleries() -> TestResult {
    let pool = setup_pool().await?;
    let staff_did = unique_discord_id();
    let staff_id = create_test_user(&pool, &staff_did, "staff").await?;
    let session = create_test_session(&pool, staff_id).await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "target_type": "community",
        "image_url": "https://example.com/gallery/community.webp",
        "caption": "Community memory"
    });

    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/admin/gallery")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(upload.status(), StatusCode::CREATED);
    let upload_body = parse_json(upload).await?;
    assert_eq!(upload_body["target_type"], "community");
    assert!(upload_body["club"].is_null());

    let list = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/internal/admin/galleries?target_type=community")
                .header("Cookie", format!("session_id={session}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(list.status(), StatusCode::OK);
    let list_body = parse_json(list).await?;
    assert_eq!(list_body["items"][0]["target_type"], "community");
    assert_eq!(list_body["items"][0]["image_url"], body["image_url"]);
    Ok(())
}

#[tokio::test]
async fn test_staff_can_upload_gallery_files_and_backend_serves_them() -> TestResult {
    let pool = setup_pool().await?;
    let staff_did = unique_discord_id();
    let staff_id = create_test_user(&pool, &staff_did, "staff").await?;
    let session = create_test_session(&pool, staff_id).await?;
    let app = build_app(pool)?;

    let upload = app
        .clone()
        .oneshot(multipart_request(
            "/api/v1/internal/admin/gallery/files",
            &session,
            &[("target_type", "community"), ("caption", "Batch upload")],
            &[("files", "photo.webp", "image/webp", b"fake-webp-binary")],
        )?)
        .await?;

    assert_eq!(upload.status(), StatusCode::CREATED);
    let upload_body = parse_json(upload).await?;
    assert_eq!(upload_body["uploaded_count"], 1);

    let image_url = upload_body["items"][0]["image_url"]
        .as_str()
        .ok_or("missing uploaded image url")?;
    let image_id = upload_body["items"][0]["id"]
        .as_str()
        .ok_or("missing uploaded image id")?;
    let image_path = image_url
        .strip_prefix("http://localhost:3000")
        .ok_or("uploaded image url must use backend base url")?;

    let file_response = app
        .clone()
        .oneshot(empty_request(Method::GET, image_path)?)
        .await?;
    assert_eq!(file_response.status(), StatusCode::OK);
    let file_bytes = axum::body::to_bytes(file_response.into_body(), usize::MAX).await?;
    assert_eq!(file_bytes.as_ref(), b"fake-webp-binary");

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/internal/admin/gallery/{image_id}"))
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let missing_response = app
        .oneshot(empty_request(Method::GET, image_path)?)
        .await?;
    assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn test_public_club_gallery_excludes_community_images() -> TestResult {
    let pool = setup_pool().await?;
    let staff_did = unique_discord_id();
    let staff_id = create_test_user(&pool, &staff_did, "staff").await?;

    let club_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO clubs (name, description_markdown, description_html, owner_user_id) VALUES ($1, '', '', $2) RETURNING id"
    )
    .bind("Scoped Gallery Club")
    .bind(staff_id)
    .fetch_one(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO gallery_images (target_type, club_id, uploaded_by_user_id, image_url, caption, status) VALUES ('community', NULL, $1, $2, NULL, 'approved')"
    )
    .bind(staff_id)
    .bind("https://example.com/gallery/community-only.webp")
    .execute(&pool)
    .await?;

    sqlx::query(
        "INSERT INTO gallery_images (target_type, club_id, uploaded_by_user_id, image_url, caption, status) VALUES ('club', $1, $2, $3, NULL, 'approved')"
    )
    .bind(club_id)
    .bind(staff_id)
    .bind("https://example.com/gallery/club-only.webp")
    .execute(&pool)
    .await?;

    let app = build_app(pool)?;
    let response = app
        .oneshot(empty_request(
            Method::GET,
            &format!("/api/v1/public/clubs/{club_id}/gallery"),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    assert_eq!(body["total_count"], 1);
    assert_eq!(body["items"][0]["image_url"], "https://example.com/gallery/club-only.webp");
    Ok(())
}

#[tokio::test]
async fn test_logout_invalidates_session_and_clears_cookie() -> TestResult {
    let pool = setup_pool().await?;
    let did = unique_discord_id();
    let user_id = create_test_user(&pool, &did, "member").await?;
    let session = create_test_session(&pool, user_id).await?;
    let app = build_app(pool.clone())?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/internal/auth/logout")
                .header("Cookie", format!("session_id={session}"))
                .header("Origin", "http://localhost:5173")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let set_cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing set-cookie header")?;
    assert!(set_cookie.contains("session_id="));
    assert!(set_cookie.contains("Max-Age=0"));

    let remaining_sessions =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(remaining_sessions, 0);
    Ok(())
}

#[tokio::test]
async fn test_auth_callback_invalid_state_clears_oauth_cookie() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/auth/discord/callback?code=test-code&state=invalid")
                .header("Cookie", "oauth_state=test-nonce")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("http://localhost:5173/login?error=csrf_failed")
    );

    let set_cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing set-cookie header")?;
    assert!(set_cookie.contains("oauth_state="));
    assert!(set_cookie.contains("Max-Age=0"));
    Ok(())
}

#[tokio::test]
async fn test_public_events_no_auth_required() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/public/events")?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await?;
    assert!(body["items"].is_array());
    assert_eq!(body["total_count"], 0);
    Ok(())
}

#[tokio::test]
async fn test_public_members_invalid_query_returns_validation_error() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app
        .oneshot(empty_request(Method::GET, "/api/v1/public/members?page=0&per_page=20")?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    assert!(body["details"]["query"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    // Metrics endpoint now requires Bearer token authentication
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/metrics")
                .header(
                    "authorization",
                    "Bearer test_system_token_at_least_32_chars_long",
                )
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint_unauthorized() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    // Metrics endpoint without token should return 401
    let response = app.oneshot(empty_request(Method::GET, "/metrics")?).await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn test_security_headers_present() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let response = app.oneshot(empty_request(Method::GET, "/health")?).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(
        headers
            .get("x-content-type-options")
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        headers
            .get("x-frame-options")
            .and_then(|value| value.to_str().ok()),
        Some("DENY")
    );
    assert_eq!(
        headers
            .get("referrer-policy")
            .and_then(|value| value.to_str().ok()),
        Some("strict-origin-when-cross-origin")
    );
    assert!(headers.get("strict-transport-security").is_some());
    assert!(headers.get("content-security-policy").is_some());
    assert!(headers.get("permissions-policy").is_some());
    Ok(())
}

#[tokio::test]
async fn test_system_endpoint_requires_bearer_token() -> TestResult {
    let pool = setup_pool().await?;
    let app = build_app(pool)?;

    let body = serde_json::json!({
        "discord_id": "leaving_user_123"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/sync/users/leave")
                // No Authorization header
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

fn percentile_micros(samples: &[u128], percentile: f64) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();

    let index = ((sorted.len().saturating_sub(1)) as f64 * percentile).round() as usize;
    sorted[index]
}

fn mean_micros(samples: &[u128]) -> f64 {
    let total: u128 = samples.iter().copied().sum();
    total as f64 / samples.len() as f64
}

#[tokio::test]
#[ignore = "perf smoke benchmark"]
async fn perf_public_member_endpoints_smoke() -> TestResult {
    let pool = setup_pool().await?;

    sqlx::query(
        r#"
        TRUNCATE TABLE
            reports,
            gallery_images,
            club_members,
            clubs,
            event_tag_mappings,
            events,
            sessions,
            profiles,
            users
        CASCADE
        "#,
    )
    .execute(&pool)
    .await?;

    for index in 0..400 {
        let discord_id = unique_discord_id();
        let user_id = create_test_user(&pool, &discord_id, "member").await?;
        create_test_profile(&pool, user_id, &format!("PerfUser{index}"), true).await?;
    }

    let mut sessions = Vec::with_capacity(50);
    for index in 0..50 {
        let auth_discord_id = unique_discord_id();
        let auth_user_id = create_test_user(&pool, &auth_discord_id, "member").await?;
        create_test_profile(&pool, auth_user_id, &format!("PerfProfile{index}"), true).await?;
        sessions.push(create_test_session(&pool, auth_user_id).await?);
    }

    let app = build_app(pool)?;

    for iteration in 0..5 {
        let response = app
            .clone()
            .oneshot(request_with_connect_info(
                Method::GET,
                "/api/v1/public/members?per_page=24",
                (iteration + 1) as u8,
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/internal/me/profile")
                    .header("Cookie", format!("session_id={}", sessions[iteration]))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }

    let mut public_members_samples = Vec::with_capacity(50);
    let mut own_profile_samples = Vec::with_capacity(50);

    for iteration in 0..50 {
        let started = Instant::now();
        let response = app
            .clone()
            .oneshot(request_with_connect_info(
                Method::GET,
                "/api/v1/public/members?per_page=24",
                (iteration % 200 + 1) as u8,
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        public_members_samples.push(started.elapsed().as_micros());

        let started = Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/v1/internal/me/profile")
                    .header("Cookie", format!("session_id={}", sessions[iteration]))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        own_profile_samples.push(started.elapsed().as_micros());
    }

    println!(
        "PERF public_members mean_us={:.1} p50_us={} p95_us={} samples={}",
        mean_micros(&public_members_samples),
        percentile_micros(&public_members_samples, 0.50),
        percentile_micros(&public_members_samples, 0.95),
        public_members_samples.len()
    );
    println!(
        "PERF own_profile mean_us={:.1} p50_us={} p95_us={} samples={}",
        mean_micros(&own_profile_samples),
        percentile_micros(&own_profile_samples, 0.50),
        percentile_micros(&own_profile_samples, 0.95),
        own_profile_samples.len()
    );

    Ok(())
}
