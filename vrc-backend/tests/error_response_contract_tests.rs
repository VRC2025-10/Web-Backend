/// ApiError response contract tests.
/// Spec refs:
/// - error-handling.md
/// Coverage:
/// - status/code/message/details mapping for representative error variants
/// - domain/infrastructure conversion behavior
mod support;

use std::collections::HashMap;

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use vrc_backend::errors::api::ApiError;
use vrc_backend::errors::domain::DomainError;
use vrc_backend::errors::infrastructure::InfraError;

use support::TestResult;

async fn assert_error_contract(
    error: ApiError,
    expected_status: StatusCode,
    expected_code: &str,
    expected_message: &str,
) -> TestResult<serde_json::Value> {
    let response = error.into_response();
    assert_eq!(response.status(), expected_status);
    let body: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX).await?,
    )?;
    assert_eq!(body["error"], expected_code);
    assert_eq!(body["message"], expected_message);
    Ok(body)
}

#[tokio::test]
async fn test_profile_validation_error_includes_details() -> TestResult {
    let body = assert_error_contract(
        ApiError::ProfileValidation(HashMap::from([(
            "vrc_id".to_owned(),
            "VRC IDの形式が正しくありません".to_owned(),
        )])),
        StatusCode::BAD_REQUEST,
        "ERR-PROF-001",
        "プロフィールのバリデーションに失敗しました",
    )
    .await?;

    assert_eq!(body["details"]["vrc_id"], "VRC IDの形式が正しくありません");
    Ok(())
}

#[tokio::test]
async fn test_insufficient_role_maps_staff_requirement_to_err_perm_001() -> TestResult {
    let body = assert_error_contract(
        ApiError::InsufficientRole {
            required: "staff",
            actual: "member".to_owned(),
        },
        StatusCode::FORBIDDEN,
        "ERR-PERM-001",
        "権限が不足しています",
    )
    .await?;

    assert_eq!(body["details"], serde_json::Value::Null);
    Ok(())
}

#[tokio::test]
async fn test_insufficient_role_maps_admin_requirement_to_err_perm_002() -> TestResult {
    let body = assert_error_contract(
        ApiError::InsufficientRole {
            required: "admin",
            actual: "staff".to_owned(),
        },
        StatusCode::FORBIDDEN,
        "ERR-PERM-002",
        "権限が不足しています",
    )
    .await?;

    assert_eq!(body["details"], serde_json::Value::Null);
    Ok(())
}

#[tokio::test]
async fn test_generic_validation_error_returns_err_validation() -> TestResult {
    let body = assert_error_contract(
        ApiError::ValidationError(HashMap::from([(
            "query".to_owned(),
            "クエリパラメータが不正です".to_owned(),
        )])),
        StatusCode::BAD_REQUEST,
        "ERR-VALIDATION",
        "バリデーションに失敗しました",
    )
    .await?;

    assert_eq!(body["details"]["query"], "クエリパラメータが不正です");
    Ok(())
}

#[tokio::test]
async fn test_internal_error_hides_details() -> TestResult {
    let body = assert_error_contract(
        ApiError::Internal("Database connection failed".to_owned()),
        StatusCode::INTERNAL_SERVER_ERROR,
        "ERR-INTERNAL",
        "内部サーバーエラー",
    )
    .await?;

    assert_eq!(body["details"], serde_json::Value::Null);
    Ok(())
}

#[tokio::test]
async fn test_domain_error_conversion_for_validation_error_preserves_details() -> TestResult {
    let api_error = ApiError::from(DomainError::ValidationError(HashMap::from([(
        "field".to_owned(),
        "invalid".to_owned(),
    )])));

    let body = assert_error_contract(
        api_error,
        StatusCode::BAD_REQUEST,
        "ERR-VALIDATION",
        "バリデーションに失敗しました",
    )
    .await?;

    assert_eq!(body["details"]["field"], "invalid");
    Ok(())
}

#[tokio::test]
async fn test_domain_error_conversion_for_not_guild_member_becomes_internal_error() -> TestResult {
    let api_error = ApiError::from(DomainError::NotGuildMember);

    let body = assert_error_contract(
        api_error,
        StatusCode::INTERNAL_SERVER_ERROR,
        "ERR-INTERNAL",
        "内部サーバーエラー",
    )
    .await?;

    assert_eq!(body["details"], serde_json::Value::Null);
    Ok(())
}

#[tokio::test]
async fn test_infrastructure_error_conversion_is_never_exposed() -> TestResult {
    let api_error = ApiError::from(InfraError::Database(sqlx::Error::RowNotFound));

    let body = assert_error_contract(
        api_error,
        StatusCode::INTERNAL_SERVER_ERROR,
        "ERR-INTERNAL",
        "内部サーバーエラー",
    )
    .await?;

    assert_eq!(body["details"], serde_json::Value::Null);
    Ok(())
}