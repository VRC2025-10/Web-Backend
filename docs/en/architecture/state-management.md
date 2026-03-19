# State Management

> **Navigation**: [Docs Home](../README.md) > [Architecture](README.md) > State Management

## Overview

This document describes all state machines and entity lifecycles in the VRC Backend. Each stateful entity uses a PostgreSQL enum to represent its current state, with transitions enforced at the application layer. Invalid transitions result in domain errors.

---

## 1. User Status

Users have two possible states. Suspension can occur via admin action or automatically when a member leaves the Discord guild. Reactivation requires explicit admin intervention.

```mermaid
stateDiagram-v2
    [*] --> Active : User created<br/>(OAuth2 first login)

    Active --> Suspended : Admin suspends user<br/>(POST /admin/users/:id/suspend)
    Active --> Suspended : Member leaves guild<br/>(POST /system/members/leave)

    Suspended --> Active : Admin reactivates user<br/>(POST /admin/users/:id/reactivate)

    note right of Active
        User can:
        - Login and use all features
        - Access Internal API
        - Maintain active sessions
    end note

    note right of Suspended
        User cannot:
        - Login (OAuth2 blocked)
        - Access any authenticated API
        All sessions are destroyed
        Profile is set to private
        Club memberships removed
    end note
```

### Transition Table

| From | To | Trigger | Actor | Side Effects |
|------|----|---------|-------|-------------|
| — | `active` | First OAuth2 login | System | Create user + default profile |
| `active` | `suspended` | Admin action | Admin / Super Admin | Delete all sessions, set profile private |
| `active` | `suspended` | Member leave | Discord Bot (System API) | Delete all sessions, set profile private, remove club memberships |
| `suspended` | `active` | Admin reactivation | Admin / Super Admin | None (user must login again to create session) |

---

## 2. Event Status

Events follow a linear lifecycle with branching terminal states. Events created via GAS are published immediately. Draft events can be created manually via the admin API. Archival is automatic after events age past the configured threshold (30 days default, 60 days for large events).

```mermaid
stateDiagram-v2
    [*] --> Draft : Event created manually<br/>(POST /admin/events)

    [*] --> Published : Event synced from GAS<br/>(POST /system/events)

    Draft --> Published : Admin publishes<br/>(PATCH /admin/events/:id/publish)
    Draft --> Cancelled : Admin cancels<br/>(PATCH /admin/events/:id/cancel)

    Published --> Cancelled : Admin cancels<br/>(PATCH /admin/events/:id/cancel)
    Published --> Archived : Background scheduler<br/>(auto after 30/60 days)

    Cancelled --> [*]
    Archived --> [*]

    note right of Draft
        Not visible to public API.
        Only visible to Admin.
    end note

    note right of Published
        Visible on public API.
        Listed in event feeds.
        Discord webhook sent on creation.
    end note

    note right of Cancelled
        Terminal state.
        Hidden from public API.
        Preserved for audit.
    end note

    note right of Archived
        Terminal state.
        Hidden from public API event listing.
        May be accessible via direct link.
    end note
```

### Transition Table

| From | To | Trigger | Actor | Side Effects |
|------|----|---------|-------|-------------|
| — | `draft` | Manual creation | Admin | None |
| — | `published` | GAS sync | GAS (System API) | Discord webhook notification |
| `draft` | `published` | Admin publishes | Admin | Discord webhook notification |
| `draft` | `cancelled` | Admin cancels | Admin | None |
| `published` | `cancelled` | Admin cancels | Admin | None |
| `published` | `archived` | Age threshold exceeded | Background Scheduler | None |

### Archival Rules

- Events are eligible for archival when `end_time` (or `start_time` if no `end_time`) is older than the configured threshold
- Default threshold: **30 days** past `end_time`
- The background scheduler checks every **24 hours**
- Archival is irreversible — archived events cannot be republished

---

## 3. Gallery Image Status

Gallery images require staff review before becoming publicly visible. The moderation workflow ensures all community-uploaded content meets guidelines.

```mermaid
stateDiagram-v2
    [*] --> Pending : User uploads image<br/>(POST /internal/gallery)

    Pending --> Approved : Staff approves<br/>(PATCH /admin/gallery/:id/approve)
    Pending --> Rejected : Staff rejects<br/>(PATCH /admin/gallery/:id/reject)

    Approved --> [*]
    Rejected --> [*]

    note right of Pending
        Not visible on public API.
        Visible to uploader and Staff+.
        Queued in staff review panel.
    end note

    note right of Approved
        Visible on public API.
        Shown in user's gallery.
    end note

    note right of Rejected
        Not visible on public API.
        Visible to uploader with
        rejection notice.
        May be deleted by uploader.
    end note
```

### Transition Table

| From | To | Trigger | Actor | Side Effects |
|------|----|---------|-------|-------------|
| — | `pending` | Image upload | Any authenticated user | Image stored, added to review queue |
| `pending` | `approved` | Staff review | Staff / Admin / Super Admin | `reviewer_id` and `reviewed_at` set |
| `pending` | `rejected` | Staff review | Staff / Admin / Super Admin | `reviewer_id` and `reviewed_at` set |

---

## 4. Report Status

Reports follow a simple triage workflow. Staff members review reports and either take action (`reviewed`) or determine no action is needed (`dismissed`).

```mermaid
stateDiagram-v2
    [*] --> Pending : User submits report<br/>(POST /internal/reports)

    Pending --> Reviewed : Staff reviews<br/>and takes action<br/>(PATCH /admin/reports/:id)
    Pending --> Dismissed : Staff dismisses<br/>(PATCH /admin/reports/:id)

    Reviewed --> [*]
    Dismissed --> [*]

    note right of Pending
        Visible in staff report queue.
        Target content flagged for review.
    end note

    note right of Reviewed
        Action was taken on the target.
        reviewer_id and reviewer_note recorded.
    end note

    note right of Dismissed
        No action needed.
        reviewer_id and reviewer_note recorded.
    end note
```

### Transition Table

| From | To | Trigger | Actor | Side Effects |
|------|----|---------|-------|-------------|
| — | `pending` | Report submitted | Any authenticated user | Added to staff review queue |
| `pending` | `reviewed` | Staff action | Staff / Admin / Super Admin | `reviewer_id`, `reviewer_note`, `resolved_at` set. Action taken on target (e.g., content removed, user warned). |
| `pending` | `dismissed` | Staff dismissal | Staff / Admin / Super Admin | `reviewer_id`, `reviewer_note`, `resolved_at` set. No action on target. |

---

## 5. Session Lifecycle

Sessions are created during OAuth2 login and destroyed via explicit logout, user suspension, or automatic cleanup of expired sessions.

```mermaid
stateDiagram-v2
    [*] --> Created : OAuth2 callback success<br/>(token generated, hash stored)

    Created --> Active : First request with session cookie

    Active --> Active : Subsequent requests<br/>(last_accessed_at updated)

    Active --> Expired : expires_at exceeded<br/>(no explicit transition—<br/>detected on access or cleanup)

    Active --> Destroyed : User logs out<br/>(DELETE /auth/logout)
    Active --> Destroyed : User suspended<br/>(all sessions bulk-deleted)
    Active --> Destroyed : Admin force-logout<br/>(DELETE /admin/users/:id/sessions)

    Expired --> Destroyed : Background scheduler<br/>cleanup (hourly)

    Destroyed --> [*]

    note right of Created
        Session row exists in DB.
        Cookie set on client.
        Not yet validated by a request.
    end note

    note right of Active
        Session is valid.
        last_accessed_at updates
        on each request (sliding window).
    end note

    note right of Expired
        Session past expires_at.
        Rejected on next access attempt.
        Cleaned up by background job.
    end note

    note right of Destroyed
        Session row deleted from DB.
        Cookie invalidated on client.
    end note
```

### Session Properties

| Property | Value |
|----------|-------|
| Token generation | 32 bytes, `rand::OsRng`, base64url-encoded |
| Storage | SHA-256 hash in `sessions.token_hash` |
| Default TTL | 7 days |
| Sliding window | `last_accessed_at` updated on each authenticated request |
| Cleanup interval | Every 1 hour (background scheduler) |
| Max sessions per user | Unlimited (multi-device support) |

---

## 6. OAuth2 Flow States

The Discord OAuth2 authorization code flow has its own transient state machine. These states exist in memory during the login process and are not persisted to the database.

```mermaid
stateDiagram-v2
    [*] --> Initiated : User clicks "Login with Discord"<br/>(state parameter generated)

    Initiated --> CodeReceived : Discord redirects to callback<br/>(code + state in query params)

    CodeReceived --> StateValidated : state parameter matches<br/>stored value (CSRF check)
    CodeReceived --> Failed : state parameter mismatch<br/>(possible CSRF attack)

    StateValidated --> TokenExchanged : POST /oauth2/token succeeds<br/>(access_token received)
    StateValidated --> Failed : Token exchange fails<br/>(invalid code or Discord error)

    TokenExchanged --> UserFetched : GET /users/@me succeeds<br/>(Discord user info received)
    TokenExchanged --> Failed : User fetch fails

    UserFetched --> GuildVerified : User is member of VRC guild
    UserFetched --> Failed : User not in guild<br/>(403 Forbidden)

    GuildVerified --> SessionCreated : User upserted in DB<br/>Session created

    SessionCreated --> [*] : Redirect to frontend<br/>with session cookie

    Failed --> [*] : Redirect to frontend<br/>with error param

    note right of Failed
        All failure states redirect to
        the frontend with an error
        query parameter describing
        the failure reason.
    end note
```

### Error Handling

| Failure Point | HTTP Response | User-Facing Error |
|--------------|---------------|-------------------|
| State mismatch | 302 → `/login?error=csrf` | "Login failed. Please try again." |
| Token exchange failure | 302 → `/login?error=discord` | "Discord authentication failed." |
| User fetch failure | 302 → `/login?error=discord` | "Could not retrieve your Discord account." |
| Not in guild | 302 → `/login?error=not_member` | "You must be a member of the VRC Discord server." |
| User suspended | 302 → `/login?error=suspended` | "Your account has been suspended." |

---

## Related Documents

- [Data Model](data-model.md) — Enum types and table definitions for each state
- [Data Flow](data-flow.md) — Sequence diagrams showing state transitions in context
- [Components](components.md) — Components responsible for enforcing transitions
- [System Context](system-context.md) — External actors that trigger state changes
- [Module Dependencies](module-dependency.md) — Code modules implementing state logic
