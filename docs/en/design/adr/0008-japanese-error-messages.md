# ADR-0008: Japanese Error Messages

> **Navigation**: [Docs Home](../../README.md) > [Design](../README.md) > [ADRs](README.md) > ADR-0008

## Status

**Accepted**

## Date

2025-01-10

## Context

The VRC Web-Backend serves the VRChat October Class Reunion community, which is a predominantly Japanese-speaking community. When users encounter errors (validation failures, authorization issues, etc.), the error messages are displayed in the frontend UI.

We need to decide what language to use for user-facing error messages.

### Forces

- The primary audience is Japanese-speaking
- Adding i18n (internationalization) introduces framework dependencies and complexity
- Error codes (e.g., `ERR-AUTH-001`) provide language-independent machine-readable identifiers
- The codebase and internal documentation are in English
- The community is unlikely to expand internationally in the near term

## Decision

We will use **Japanese for all user-facing error messages**. Machine-readable error codes are provided alongside messages for programmatic handling.

### Implementation

```rust
// Error messages are in Japanese
impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::UserNotFound => ApiError::NotFound {
                code: "ERR-USR-001",
                message: "ユーザーが見つかりません".into(),
            },
            DomainError::UserSuspended => ApiError::Forbidden {
                code: "ERR-USR-002",
                message: "アカウントが停止されています".into(),
            },
            DomainError::InvalidInput(msg) => ApiError::BadRequest {
                code: "ERR-VAL-001",
                message: msg, // Validation messages are also in Japanese
            },
            // ...
        }
    }
}
```

### Response Format

```json
{
  "error": {
    "code": "ERR-USR-001",
    "message": "ユーザーが見つかりません"
  }
}
```

- `code`: Language-independent, machine-readable identifier — always English
- `message`: Human-readable description — always Japanese

### Boundaries

| Context | Language |
|---------|----------|
| User-facing error messages | Japanese |
| Error codes | English (machine-readable) |
| Log messages | English |
| Code comments | English |
| Documentation (internal) | English |
| API documentation | English (with Japanese examples) |

## Consequences

### Positive

- **Better UX for primary audience**: Users see errors in their native language
- **No i18n framework overhead**: No message catalogs, locale negotiation, or runtime translation
- **Simpler codebase**: Error messages are simple string literals
- **Error codes are universal**: Frontend can use codes for programmatic handling regardless of message language

### Negative

- **Non-Japanese speakers see untranslated messages**: If the community ever includes non-Japanese speakers, they'll need to rely on error codes
- **Retrofitting i18n is harder**: Adding translations later requires extracting all message strings
- **Testing requires Japanese knowledge**: Error message assertions in tests contain Japanese text
- **Mixed-language codebase**: Rust code contains embedded Japanese strings

### Neutral

- Error codes provide a language-independent fallback for programmatic error handling
- The frontend could theoretically maintain its own code-to-message mapping in any language

## Alternatives Considered

### Alternative 1: English-Only Messages

**Description**: All error messages in English.

**Pros**:
- Consistent language throughout the codebase
- Easier for English-speaking contributors

**Cons**:
- Poor UX for the Japanese-speaking primary audience
- Users would need to understand English error messages

**Why Rejected**: The primary audience is Japanese-speaking. Forcing English error messages creates unnecessary friction.

### Alternative 2: i18n with Message Keys

**Description**: Use a message key system (e.g., `fluent`, `gettext`) with translation files.

```rust
message: t!("error.user_not_found", locale)
```

**Pros**:
- Supports multiple languages
- Clean separation of code and translations
- Standard approach for multilingual applications

**Cons**:
- Additional framework dependency
- Message catalogs to maintain
- Locale negotiation logic
- Overkill for a single-language community

**Why Rejected**: The overhead of i18n is unjustified for a community that speaks one language. If multilingual support becomes necessary, error codes provide a migration path.

### Alternative 3: Bilingual Responses

**Description**: Include both Japanese and English messages in every error response.

```json
{
  "error": {
    "code": "ERR-USR-001",
    "message_ja": "ユーザーが見つかりません",
    "message_en": "User not found"
  }
}
```

**Pros**:
- Serves both audiences
- No locale negotiation needed

**Cons**:
- Double the message maintenance
- Larger response payloads
- Non-standard API pattern

**Why Rejected**: Doubles the maintenance burden with no clear benefit for the current audience.

## Related

- [Trade-offs](../trade-offs.md) — Trade-off 8: Japanese over English
- [Error Reference](../../reference/errors.md) — complete error code listing with messages
