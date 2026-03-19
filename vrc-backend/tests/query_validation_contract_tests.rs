/// Query validation contract tests.
/// Spec refs:
/// - pagination.md
/// - public-api.md
/// - error-handling.md
/// Coverage:
/// - invalid query parameters return JSON ERR-VALIDATION
/// - public routes still emit cache and security headers on validation failures
mod support;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

use support::{TestResult, build_app, parse_json};

#[tokio::test]
async fn test_public_members_rejects_page_zero_with_json_error_contract() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/public/members?page=0")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("public, max-age=30, stale-while-revalidate=60")
    );
    assert_eq!(
        response
            .headers()
            .get("x-frame-options")
            .and_then(|value| value.to_str().ok()),
        Some("DENY")
    );

    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    assert_eq!(body["details"]["query"], "クエリパラメータが不正です");
    Ok(())
}

#[tokio::test]
async fn test_public_members_rejects_non_integer_page_with_json_error_contract() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/public/members?page=abc")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    assert_eq!(body["details"]["query"], "クエリパラメータが不正です");
    Ok(())
}

#[tokio::test]
async fn test_public_events_rejects_per_page_above_100() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/public/events?per_page=101")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    assert_eq!(body["details"]["query"], "クエリパラメータが不正です");
    Ok(())
}

#[tokio::test]
async fn test_public_clubs_rejects_per_page_zero() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/public/clubs?per_page=0")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    assert_eq!(body["details"]["query"], "クエリパラメータが不正です");
    Ok(())
}

#[tokio::test]
async fn test_public_gallery_rejects_invalid_page_parameter() -> TestResult {
    let app = build_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/public/clubs/550e8400-e29b-41d4-a716-446655440000/gallery?page=0")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await?;
    assert_eq!(body["error"], "ERR-VALIDATION");
    Ok(())
}