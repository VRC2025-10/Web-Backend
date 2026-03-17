use std::collections::HashMap;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::domain::DomainError;
use super::infrastructure::InfraError;

/// The final error type returned to clients.
/// Maps to HTTP status + JSON body with error codes.
#[derive(Debug)]
pub enum ApiError {
    // Domain errors
    ProfileValidation(HashMap<String, String>),
    BioDangerous,
    ProfileNotFound,
    SessionInvalid,
    AccountSuspended,
    CsrfFailed,
    InsufficientRole {
        required: &'static str,
        actual: String,
    },
    AdminRoleEscalation,
    SuperAdminRoleEscalation,
    SuperAdminProtected,
    RoleLevelInsufficient,
    ReportTargetNotFound,
    DuplicateReport,
    ReportReasonLength,
    EventNotFound,
    ClubNotFound,
    GalleryImageNotFound,
    InvalidGalleryStatus,
    UserNotFound,
    SystemTokenInvalid,
    SystemValidation(HashMap<String, String>),
    RateLimited,
    ValidationError(HashMap<String, String>),

    // Infrastructure errors (logged, not exposed)
    Internal(String),
}

impl IntoResponse for ApiError {
    #[allow(clippy::too_many_lines)] // exhaustive error-to-response mapping
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            Self::ProfileValidation(d) => (
                StatusCode::BAD_REQUEST,
                "ERR-PROF-001",
                "プロフィールのバリデーションに失敗しました",
                Some(d.clone()),
            ),
            Self::BioDangerous => (
                StatusCode::BAD_REQUEST,
                "ERR-PROF-002",
                "危険なコンテンツが検出されました",
                None,
            ),
            Self::ProfileNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-PROF-004",
                "プロフィールが見つかりません",
                None,
            ),
            Self::SessionInvalid => (
                StatusCode::UNAUTHORIZED,
                "ERR-AUTH-003",
                "セッションが無効です",
                None,
            ),
            Self::AccountSuspended => (
                StatusCode::FORBIDDEN,
                "ERR-AUTH-004",
                "アカウントが停止されています",
                None,
            ),
            Self::CsrfFailed => (
                StatusCode::FORBIDDEN,
                "ERR-CSRF-001",
                "CSRF検証に失敗しました",
                None,
            ),
            Self::InsufficientRole { .. } => (
                StatusCode::FORBIDDEN,
                "ERR-PERM-001",
                "権限が不足しています",
                None,
            ),
            Self::AdminRoleEscalation => (
                StatusCode::FORBIDDEN,
                "ERR-ROLE-001",
                "admin権限の付与にはsuper_adminが必要です",
                None,
            ),
            Self::SuperAdminRoleEscalation => (
                StatusCode::FORBIDDEN,
                "ERR-ROLE-002",
                "super_admin権限の付与にはsuper_adminが必要です",
                None,
            ),
            Self::SuperAdminProtected => (
                StatusCode::FORBIDDEN,
                "ERR-ROLE-003",
                "super_adminの変更にはsuper_adminが必要です",
                None,
            ),
            Self::RoleLevelInsufficient => (
                StatusCode::FORBIDDEN,
                "ERR-ROLE-004",
                "ロール変更にはadmin以上の権限が必要です",
                None,
            ),
            Self::ReportTargetNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-MOD-001",
                "通報対象が見つかりません",
                None,
            ),
            Self::DuplicateReport => (
                StatusCode::CONFLICT,
                "ERR-MOD-002",
                "同じ対象に対する通報は既に存在します",
                None,
            ),
            Self::ReportReasonLength => (
                StatusCode::BAD_REQUEST,
                "ERR-MOD-003",
                "通報理由は10〜1000文字で入力してください",
                None,
            ),
            Self::EventNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-NOT-FOUND",
                "イベントが見つかりません",
                None,
            ),
            Self::ClubNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-NOT-FOUND",
                "部活が見つかりません",
                None,
            ),
            Self::GalleryImageNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-NOT-FOUND",
                "ギャラリー画像が見つかりません",
                None,
            ),
            Self::InvalidGalleryStatus => (
                StatusCode::BAD_REQUEST,
                "ERR-GALLERY-003",
                "無効なギャラリーステータスです",
                None,
            ),
            Self::UserNotFound => (
                StatusCode::NOT_FOUND,
                "ERR-USER-001",
                "ユーザーが見つかりません",
                None,
            ),
            Self::SystemTokenInvalid => (
                StatusCode::UNAUTHORIZED,
                "ERR-SYNC-001",
                "システムトークンが無効です",
                None,
            ),
            Self::SystemValidation(d) => (
                StatusCode::BAD_REQUEST,
                "ERR-SYNC-002",
                "バリデーションに失敗しました",
                Some(d.clone()),
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "ERR-RATELIMIT-001",
                "リクエスト制限を超えました",
                None,
            ),
            Self::ValidationError(d) => (
                StatusCode::BAD_REQUEST,
                "ERR-VALIDATION",
                "バリデーションに失敗しました",
                Some(d.clone()),
            ),
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ERR-INTERNAL",
                    "内部サーバーエラー",
                    None,
                )
            }
        };

        let body = json!({
            "error": code,
            "message": message,
            "details": details,
        });

        (status, Json(body)).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::ProfileValidation(d) => Self::ProfileValidation(d),
            DomainError::BioDangerous => Self::BioDangerous,
            DomainError::ProfileNotFound => Self::ProfileNotFound,
            DomainError::SessionInvalid => Self::SessionInvalid,
            DomainError::AccountSuspended => Self::AccountSuspended,
            DomainError::CsrfMismatch => Self::CsrfFailed,
            DomainError::NotGuildMember => Self::Internal("Not a guild member".to_owned()),
            DomainError::InsufficientRole { required, actual } => {
                Self::InsufficientRole { required, actual }
            }
            DomainError::AdminRoleEscalation => Self::AdminRoleEscalation,
            DomainError::SuperAdminRoleEscalation => Self::SuperAdminRoleEscalation,
            DomainError::SuperAdminProtected => Self::SuperAdminProtected,
            DomainError::RoleLevelInsufficient => Self::RoleLevelInsufficient,
            DomainError::ReportTargetNotFound => Self::ReportTargetNotFound,
            DomainError::DuplicateReport => Self::DuplicateReport,
            DomainError::ReportReasonLength => Self::ReportReasonLength,
            DomainError::EventNotFound => Self::EventNotFound,
            DomainError::ClubNotFound => Self::ClubNotFound,
            DomainError::GalleryImageNotFound => Self::GalleryImageNotFound,
            DomainError::InvalidGalleryStatus => Self::InvalidGalleryStatus,
            DomainError::UserNotFound => Self::UserNotFound,
            DomainError::ValidationError(d) => Self::ValidationError(d),
        }
    }
}

impl From<InfraError> for ApiError {
    fn from(err: InfraError) -> Self {
        // All infrastructure errors map to Internal — never expose details to clients
        tracing::error!(error = %err, "Infrastructure error");
        match err {
            InfraError::Database(_) => Self::Internal("Database error".to_owned()),
            InfraError::DiscordApi(_) => Self::Internal("Discord error".to_owned()),
            InfraError::Webhook(_) => Self::Internal("Webhook error".to_owned()),
            InfraError::TokenExchange => Self::Internal("Auth error".to_owned()),
        }
    }
}
