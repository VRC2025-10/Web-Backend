# Error Handling

## Error Response Format

All errors return the following JSON structure (except 204 responses):

```json
{
  "error": "ERR-XXXX-NNN",
  "message": "Human-readable description (Japanese)",
  "details": null | { "field_name": "Validation message" }
}
```

## Complete Error Code Catalog

### Authentication Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-AUTH-001` | — | (internal, not exposed) | Discord token exchange failed |
| `ERR-AUTH-002` | — | (internal, not exposed) | User is not a guild member |
| `ERR-AUTH-003` | 401 | `ApiError::SessionInvalid` | Session cookie missing, invalid, or expired |
| `ERR-AUTH-004` | 403 | `ApiError::AccountSuspended` | User's status is `suspended` |

### CSRF Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-CSRF-001` | 403 | `ApiError::CsrfFailed` | `Origin` header does not match `FRONTEND_ORIGIN` |

### Profile Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-PROF-001` | 400 | `ApiError::ProfileValidation(details)` | Field validation failure (`vrc_id`, `x_id`, `bio_markdown` length) |
| `ERR-PROF-002` | 400 | `ApiError::BioDangerous` | XSS-like content detected in `bio_markdown` |
| `ERR-PROF-004` | 404 | `ApiError::ProfileNotFound` | Profile does not exist, is non-public, or user is suspended |

### Permission Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-PERM-001` | 403 | `ApiError::InsufficientRole` | Requires `staff`+ but user has lower role |
| `ERR-PERM-002` | 403 | `ApiError::InsufficientRole` | Requires `admin`+ but user has lower role |

### Role Change Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-ROLE-001` | 403 | `ApiError::AdminRoleEscalation` | Non-super_admin attempting to grant `admin` role |
| `ERR-ROLE-002` | 403 | `ApiError::SuperAdminRoleEscalation` | Non-super_admin attempting to grant `super_admin` role |
| `ERR-ROLE-003` | 403 | `ApiError::SuperAdminProtected` | Non-super_admin attempting to modify a `super_admin` user |
| `ERR-ROLE-004` | 403 | `ApiError::RoleLevelInsufficient` | User below `admin` attempting any role change |

### Moderation Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-MOD-001` | 404 | `ApiError::ReportTargetNotFound` | `target_type` + `target_id` does not resolve to an existing entity |
| `ERR-MOD-002` | 409 | `ApiError::DuplicateReport` | Same user already reported same target |
| `ERR-MOD-003` | 400 | `ApiError::ReportReasonLength` | `reason` length outside 10–1000 range |

### Gallery Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-GALLERY-003` | 400 | `ApiError::InvalidGalleryStatus` | Invalid status value (not `pending`/`approved`/`rejected`) |

### System API Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-SYNC-001` | 401 | `ApiError::SystemTokenInvalid` | Missing or invalid Bearer token |
| `ERR-SYNC-002` | 400 | `ApiError::SystemValidation(details)` | Request body validation failure |

### Generic Errors

| Code | HTTP | Rust Variant | Condition |
|------|------|-------------|-----------|
| `ERR-NOT-FOUND` | 404 | `ApiError::EventNotFound` / `ClubNotFound` / `GalleryImageNotFound` | Requested resource does not exist |
| `ERR-USER-001` | 404 | `ApiError::UserNotFound` | Target user for role change does not exist |
| `ERR-VALIDATION` | 400 | `ApiError::ValidationError(details)` | Generic query parameter validation failure |
| `ERR-RATELIMIT-001` | 429 | `ApiError::RateLimited` | Rate limit exceeded |
| `ERR-INTERNAL` | 500 | `ApiError::Internal(msg)` | Server error (details logged, not exposed to client) |

## Error Conversion Chain

```
DomainError → ApiError:
  ProfileValidation(d)     → ProfileValidation(d)       [400, ERR-PROF-001]
  BioDangerous             → BioDangerous                [400, ERR-PROF-002]
  ProfileNotFound          → ProfileNotFound             [404, ERR-PROF-004]
  SessionInvalid           → SessionInvalid              [401, ERR-AUTH-003]
  AccountSuspended         → AccountSuspended            [403, ERR-AUTH-004]
  CsrfMismatch             → CsrfFailed                 [403, ERR-CSRF-001]
  InsufficientRole{..}     → InsufficientRole{..}        [403, ERR-PERM-*]
  AdminRoleEscalation      → AdminRoleEscalation         [403, ERR-ROLE-001]
  SuperAdminRoleEscalation → SuperAdminRoleEscalation    [403, ERR-ROLE-002]
  SuperAdminProtected      → SuperAdminProtected         [403, ERR-ROLE-003]
  ReportTargetNotFound     → ReportTargetNotFound        [404, ERR-MOD-001]
  DuplicateReport          → DuplicateReport             [409, ERR-MOD-002]
  ReportReasonLength       → ReportReasonLength          [400, ERR-MOD-003]
  EventNotFound            → EventNotFound               [404, ERR-NOT-FOUND]
  ClubNotFound             → ClubNotFound                [404, ERR-NOT-FOUND]
  GalleryImageNotFound     → GalleryImageNotFound        [404, ERR-NOT-FOUND]
  InvalidGalleryStatus     → InvalidGalleryStatus        [400, ERR-GALLERY-003]
  UserNotFound             → UserNotFound                [404, ERR-USER-001]

InfraError → ApiError:
  Database(sqlx::Error)    → Internal("Database error")  [500, ERR-INTERNAL]
  DiscordApi(msg)          → Internal("Discord error")   [500, ERR-INTERNAL]
  Webhook(msg)             → Internal("Webhook error")   [500, ERR-INTERNAL]
  TokenExchange            → Internal("Auth error")      [500, ERR-INTERNAL]
```

Every `InfraError` logs the full error at `error!` level but returns a generic 500 to the client (no internal details leaked).
