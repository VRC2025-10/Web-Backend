/// System API validation and auth contract tests.
/// Spec refs:
/// - system-api.md
/// - error-handling.md
/// Coverage:
/// - Bearer auth failures
/// - ValidatedJson request failures
/// - Cross-field validation in system handlers before database access
mod support;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use support::{TestResult, build_app, parse_json};

fn valid_event_body() -> serde_json::Value {
    json!({
        "external_id": "gas_event_001",
        "title": "Sample Event",
        "description_markdown": "hello",
        "status": "published",
        "host_discord_id": null,
        "start_time": "2026-03-19T10:00:00Z",
        "end_time": "2026-03-19T11:00:00Z",
        "location": "VRChat world",
        "tags": ["official"]
    })
}

#[tokio::test]
async fn test_system_events_requires_bearer_token() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&valid_event_body())?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-SYNC-001");
    Ok(())
}

#[tokio::test]
async fn test_system_events_rejects_structural_validation_errors() -> TestResult {
    let app = build_app()?;
    let body = json!({
        "external_id": "",
        "title": "",
        "description_markdown": "hello",
        "status": "published",
        "host_discord_id": null,
        "start_time": "2026-03-19T10:00:00Z",
        "end_time": "2026-03-19T11:00:00Z",
        "location": "VRChat world",
        "tags": ["official"]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/events")
                .header("authorization", "Bearer 0123456789abcdefghijklmnopqrstuvwxyz")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = parse_json(response).await?;
    assert_eq!(payload["error"], "ERR-SYNC-002");
    assert!(payload["details"]["external_id"].is_string());
    assert!(payload["details"]["title"].is_string());
    Ok(())
}

#[tokio::test]
async fn test_system_events_rejects_too_many_tags_before_database_access() -> TestResult {
    let app = build_app()?;
    let mut body = valid_event_body();
    body["tags"] = json!([
        "1","2","3","4","5","6","7","8","9","10","11"
    ]);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/events")
                .header("authorization", "Bearer 0123456789abcdefghijklmnopqrstuvwxyz")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = parse_json(response).await?;
    assert_eq!(payload["error"], "ERR-SYNC-002");
    assert_eq!(payload["details"]["tags"], "タグは最大10個までです");
    Ok(())
}

#[tokio::test]
async fn test_system_events_rejects_end_time_not_after_start_time() -> TestResult {
    let app = build_app()?;
    let mut body = valid_event_body();
    body["end_time"] = json!("2026-03-19T10:00:00Z");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/events")
                .header("authorization", "Bearer 0123456789abcdefghijklmnopqrstuvwxyz")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = parse_json(response).await?;
    assert_eq!(payload["error"], "ERR-SYNC-002");
    assert_eq!(payload["details"]["end_time"], "end_time は start_time より後にしてください");
    Ok(())
}

#[tokio::test]
async fn test_system_member_leave_rejects_invalid_discord_id() -> TestResult {
    let app = build_app()?;
    let body = json!({ "discord_id": "not-a-snowflake" });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/system/sync/users/leave")
                .header("authorization", "Bearer 0123456789abcdefghijklmnopqrstuvwxyz")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = parse_json(response).await?;
    assert_eq!(payload["error"], "ERR-SYNC-002");
    assert_eq!(payload["details"]["discord_id"], "17〜20桁の数値で入力してください");
    Ok(())
}