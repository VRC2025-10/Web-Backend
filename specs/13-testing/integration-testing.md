# Integration Testing

## Strategy

Integration tests run against a real PostgreSQL instance and test the full HTTP request → handler → DB → response cycle using Axum's `TestServer` (from `axum-test` crate or manual Tower service calls).

## Test Database Setup

```rust
// tests/common/mod.rs

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            "postgres://vrc_app:test_password@localhost:5432/vrc_class_reunion_test".into()
        });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test DB");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Clean all tables (order matters for FK constraints)
    sqlx::query!("TRUNCATE club_members, gallery_images, clubs, reports, event_tag_mappings, event_tags, events, sessions, profiles, users CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to clean test DB");

    pool
}

pub async fn create_test_user(pool: &PgPool, discord_id: &str, role: &str) -> i32 {
    sqlx::query_scalar!(
        r#"INSERT INTO users (discord_id, discord_display_name, role, status)
           VALUES ($1, $2, $3::user_role, 'active')
           RETURNING id"#,
        discord_id,
        format!("TestUser_{}", discord_id),
        role
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub async fn create_test_session(pool: &PgPool, user_id: i32) -> String {
    let raw_token = generate_random_token(); // 32 bytes
    let token_hash = sha256(&raw_token);
    let cookie_value = base64url_encode(&raw_token);

    sqlx::query!(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, NOW() + INTERVAL '7 days')",
        user_id,
        &token_hash[..]
    )
    .execute(pool)
    .await
    .unwrap();

    cookie_value
}
```

## API Test Examples

### Auth Flow

```rust
#[tokio::test]
async fn test_auth_me_returns_user_info() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "111", "member").await;
    let session_cookie = create_test_session(&pool, user_id).await;

    let app = build_app(pool.clone()).await;
    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={}", session_cookie))
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["user"]["discord_id"], "111");
    assert_eq!(body["user"]["role"], "member");
}

#[tokio::test]
async fn test_auth_me_without_session_returns_401() {
    let pool = setup_test_db().await;
    let app = build_app(pool).await;

    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["error"], "ERR-AUTH-003");
}

#[tokio::test]
async fn test_suspended_user_returns_403() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "222", "member").await;
    let session_cookie = create_test_session(&pool, user_id).await;

    // Suspend the user
    sqlx::query!("UPDATE users SET status = 'suspended' WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();

    let app = build_app(pool).await;
    let response = app
        .oneshot(
            Request::get("/api/v1/internal/auth/me")
                .header("Cookie", format!("session_id={}", session_cookie))
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["error"], "ERR-AUTH-004");
}
```

### Profile CRUD

```rust
#[tokio::test]
async fn test_create_profile_via_put() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "333", "member").await;
    let session = create_test_session(&pool, user_id).await;

    let app = build_app(pool.clone()).await;
    let response = app
        .oneshot(
            Request::put("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={}", session))
                .header("Origin", "https://vrc-classreunion.example.com")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "nickname": "テストユーザー",
                    "vrc_id": "usr_12345678-1234-1234-1234-123456789abc",
                    "bio_markdown": "# Hello\nWorld",
                    "is_public": true
                })).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["nickname"], "テストユーザー");
    assert!(body["bio_html"].as_str().unwrap().contains("<h1>Hello</h1>"));
    assert_eq!(body["is_public"], true);
}

#[tokio::test]
async fn test_profile_validation_rejects_invalid_vrc_id() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "444", "member").await;
    let session = create_test_session(&pool, user_id).await;

    let app = build_app(pool).await;
    let response = app
        .oneshot(
            Request::put("/api/v1/internal/me/profile")
                .header("Cookie", format!("session_id={}", session))
                .header("Origin", "https://vrc-classreunion.example.com")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "vrc_id": "invalid_id_format",
                    "is_public": true
                })).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["error"], "ERR-PROF-001");
    assert!(body["details"]["vrc_id"].is_string());
}
```

### Role Change Authorization

```rust
#[tokio::test]
async fn test_admin_can_change_member_to_staff() {
    let pool = setup_test_db().await;
    let admin_id = create_test_user(&pool, "admin1", "admin").await;
    let member_id = create_test_user(&pool, "member1", "member").await;
    let session = create_test_session(&pool, admin_id).await;

    let app = build_app(pool).await;
    let response = app
        .oneshot(
            Request::patch(&format!("/api/v1/internal/admin/users/{}/role", member_id))
                .header("Cookie", format!("session_id={}", session))
                .header("Origin", "https://vrc-classreunion.example.com")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"role":"staff"}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_cannot_grant_admin_role() {
    let pool = setup_test_db().await;
    let admin_id = create_test_user(&pool, "admin2", "admin").await;
    let member_id = create_test_user(&pool, "member2", "member").await;
    let session = create_test_session(&pool, admin_id).await;

    let app = build_app(pool).await;
    let response = app
        .oneshot(
            Request::patch(&format!("/api/v1/internal/admin/users/{}/role", member_id))
                .header("Cookie", format!("session_id={}", session))
                .header("Origin", "https://vrc-classreunion.example.com")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"role":"admin"}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = parse_body(response).await;
    assert_eq!(body["error"], "ERR-ROLE-001");
}
```

### Member Leave Atomicity

```rust
#[tokio::test]
async fn test_member_leave_atomic_cleanup() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool, "leaving_user", "member").await;
    let _session = create_test_session(&pool, user_id).await;

    // Create profile and club membership
    sqlx::query!("INSERT INTO profiles (user_id, is_public) VALUES ($1, true)", user_id)
        .execute(&pool).await.unwrap();
    let club_id = sqlx::query_scalar!("INSERT INTO clubs (name, owner_id) VALUES ('Test Club', $1) RETURNING id", user_id)
        .fetch_one(&pool).await.unwrap();
    sqlx::query!("INSERT INTO club_members (club_id, user_id, role) VALUES ($1, $2, 'owner')", club_id, user_id)
        .execute(&pool).await.unwrap();

    let app = build_app(pool.clone()).await;
    let response = app
        .oneshot(
            Request::post("/api/v1/system/sync/users/leave")
                .header("Authorization", format!("Bearer {}", test_system_token()))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"discord_id":"leaving_user"}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify atomicity: all cleanup operations completed
    let user_status: String = sqlx::query_scalar!("SELECT status::text FROM users WHERE id = $1", user_id)
        .fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(user_status, "suspended");

    let session_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM sessions WHERE user_id = $1", user_id)
        .fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(session_count, 0);

    let is_public: bool = sqlx::query_scalar!("SELECT is_public FROM profiles WHERE user_id = $1", user_id)
        .fetch_one(&pool).await.unwrap();
    assert!(!is_public);

    let club_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM club_members WHERE user_id = $1", user_id)
        .fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(club_count, 0);
}
```

## Running Integration Tests

```bash
# Ensure test DB is running
docker compose up -d db

# Run integration tests (sequential — shared DB)
cargo test --test '*' -- --test-threads=1

# Run specific test
cargo test --test api_tests test_member_leave_atomic_cleanup
```
