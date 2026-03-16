use std::collections::HashMap;

/// Business rule violations. No HTTP concepts here.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    // Profile errors
    #[error("Profile validation failed")]
    ProfileValidation(HashMap<String, String>),
    #[error("XSS attempt detected in bio")]
    BioDangerous,
    #[error("Profile not found")]
    ProfileNotFound,

    // Auth errors
    #[error("Session expired or invalid")]
    SessionInvalid,
    #[error("Account suspended")]
    AccountSuspended,
    #[error("Not a guild member")]
    NotGuildMember,
    #[error("CSRF state mismatch")]
    CsrfMismatch,

    // Role errors
    #[error("Insufficient role: requires {required}, has {actual}")]
    InsufficientRole {
        required: &'static str,
        actual: String,
    },
    #[error("Only super_admin can grant admin role")]
    AdminRoleEscalation,
    #[error("Only super_admin can grant super_admin role")]
    SuperAdminRoleEscalation,
    #[error("Cannot modify super_admin without being super_admin")]
    SuperAdminProtected,
    #[error("Role level insufficient for role changes")]
    RoleLevelInsufficient,

    // Moderation errors
    #[error("Report target not found")]
    ReportTargetNotFound,
    #[error("Duplicate report")]
    DuplicateReport,
    #[error("Report reason out of range")]
    ReportReasonLength,

    // Event errors
    #[error("Event not found")]
    EventNotFound,

    // Club/Gallery errors
    #[error("Club not found")]
    ClubNotFound,
    #[error("Gallery image not found")]
    GalleryImageNotFound,
    #[error("Invalid gallery status")]
    InvalidGalleryStatus,

    // User errors
    #[error("User not found")]
    UserNotFound,

    // Generic validation
    #[error("Validation error")]
    ValidationError(HashMap<String, String>),
}
