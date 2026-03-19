# Data Flow

> **Navigation**: [Docs Home](../README.md) > [Architecture](README.md) > Data Flow

## Overview

This document describes the data flow for four key interactions in the VRC Backend. Each flow is illustrated with a Mermaid sequence diagram showing the complete request/response path, including authentication, validation, database operations, and side effects.

---

## 1. Discord OAuth2 Login

The authentication flow uses Discord's OAuth2 Authorization Code Grant. The backend exchanges the authorization code for tokens, fetches user information, verifies guild membership, and establishes a session.

```mermaid
sequenceDiagram
    participant B as Browser
    participant F as Frontend SPA
    participant C as Caddy
    participant BK as Backend (Axum)
    participant D as Discord API
    participant DB as PostgreSQL

    B->>F: Click "Login with Discord"
    F->>B: Redirect to Discord authorize URL<br/>(client_id, redirect_uri, scope, state)

    B->>D: GET /oauth2/authorize
    D->>B: Discord consent screen
    B->>D: User approves
    D->>B: Redirect to callback URL<br/>(code, state)

    B->>C: GET /api/v1/auth/callback?code=…&state=…
    C->>BK: Forward request

    Note over BK: Validate state parameter<br/>(CSRF protection)

    BK->>D: POST /oauth2/token<br/>(code exchange)
    D-->>BK: access_token, refresh_token

    BK->>D: GET /users/@me<br/>(Bearer access_token)
    D-->>BK: Discord user object<br/>(id, username, avatar)

    BK->>D: GET /users/@me/guilds<br/>(Bearer access_token)
    D-->>BK: Guild list

    Note over BK: Verify user is member<br/>of the VRC Discord guild

    BK->>DB: SELECT * FROM users<br/>WHERE discord_id = $1
    DB-->>BK: User row (or null)

    alt New user
        BK->>DB: INSERT INTO users (discord_id, username, role, status)
        BK->>DB: INSERT INTO profiles (user_id, display_name)
    else Existing user
        BK->>DB: UPDATE users SET username = $1,<br/>updated_at = NOW()
    end

    Note over BK: Generate secure random<br/>session token (32 bytes)

    BK->>DB: INSERT INTO sessions<br/>(user_id, token_hash, expires_at)

    BK-->>C: 302 Redirect to Frontend<br/>Set-Cookie: session=<token>; HttpOnly; Secure; SameSite=Lax
    C-->>B: Forward response
    B->>F: Load app with session cookie
```

### Key Points

- The `state` parameter prevents CSRF attacks during the OAuth2 flow
- Only the SHA-256 hash of the session token is stored in the database
- The session cookie is `HttpOnly`, `Secure`, and `SameSite=Lax`
- Guild membership is required — non-members receive `403 Forbidden`
- New users are auto-provisioned with `member` role and a default profile

---

## 2. Profile Update (Internal API)

An authenticated user updates their profile. The flow includes CSRF validation, session authentication, input validation, Markdown rendering, HTML sanitization, and database persistence.

```mermaid
sequenceDiagram
    participant B as Browser
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL

    B->>C: PUT /api/v1/internal/profile<br/>Cookie: session=<token><br/>X-CSRF-Token: <csrf_token><br/>Body: { display_name, bio_markdown, is_public }
    C->>BK: Forward request

    Note over BK: Middleware: request_id,<br/>security_headers, metrics

    Note over BK: CSRF Middleware:<br/>Validate X-CSRF-Token<br/>matches cookie

    Note over BK: AuthenticatedUser<Member>:<br/>Extract session cookie

    BK->>DB: SELECT u.*, s.expires_at<br/>FROM sessions s JOIN users u<br/>ON s.user_id = u.id<br/>WHERE s.token_hash = $1
    DB-->>BK: User + Session

    Note over BK: Verify session not expired<br/>Verify user status = active

    Note over BK: Update last_accessed_at<br/>(sliding window)

    Note over BK: ValidatedJson<ProfileUpdate>:<br/>Deserialize + validate input<br/>- display_name: 1-100 chars<br/>- bio_markdown: max 4000 chars

    Note over BK: Render bio_markdown → HTML<br/>(pulldown-cmark)

    Note over BK: Sanitize HTML<br/>(ammonia — strip scripts,<br/>allow safe tags only)

    BK->>DB: INSERT INTO profiles<br/>(user_id, display_name,<br/>bio_markdown, bio_html, is_public)<br/>ON CONFLICT (user_id) DO UPDATE
    DB-->>BK: Updated profile row

    BK-->>C: 200 OK<br/>{ profile data }
    C-->>B: Forward response
```

### Key Points

- CSRF protection uses the double-submit cookie pattern
- Input validation is performed by the `ValidatedJson` extractor (via `#[derive(Validate)]`)
- Markdown is rendered server-side and sanitized with `ammonia` to prevent XSS
- Both `bio_markdown` (source) and `bio_html` (rendered) are stored to avoid re-rendering on reads
- The `UPSERT` pattern handles both first-time profile creation and subsequent updates

---

## 3. Event Sync from Google Apps Script

GAS pushes event data to the System API. The backend verifies the Bearer token, validates the payload, upserts the event with its tags, and sends a Discord webhook notification for new events.

```mermaid
sequenceDiagram
    participant GAS as Google Apps Script
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL
    participant WH as Discord Webhook

    GAS->>C: POST /api/v1/system/events<br/>Authorization: Bearer <system_token><br/>Body: { source_id, title, description,<br/>start_time, end_time, world_link, tags[] }
    C->>BK: Forward request

    Note over BK: Middleware: request_id,<br/>security_headers, metrics

    Note over BK: Verify Bearer token<br/>matches configured<br/>SYSTEM_API_TOKEN

    Note over BK: ValidatedJson<EventSync>:<br/>Deserialize + validate<br/>- title: required, 1-200 chars<br/>- start_time: required, valid ISO 8601<br/>- tags: optional, max 10 items

    BK->>DB: SELECT * FROM events<br/>WHERE source_id = $1
    DB-->>BK: Existing event (or null)

    alt New event
        BK->>DB: BEGIN TRANSACTION
        BK->>DB: INSERT INTO events<br/>(source_id, title, description,<br/>start_time, end_time, world_link,<br/>status = 'published')
        DB-->>BK: New event row

        loop For each tag
            BK->>DB: INSERT INTO event_tags (name)<br/>ON CONFLICT (name) DO NOTHING
            BK->>DB: INSERT INTO event_tag_mappings<br/>(event_id, tag_id)
        end

        BK->>DB: COMMIT

        BK->>WH: POST Discord Webhook<br/>Embed: new event announcement<br/>(title, time, world link)
        WH-->>BK: 204 No Content

    else Existing event (update)
        BK->>DB: BEGIN TRANSACTION
        BK->>DB: UPDATE events SET<br/>title=$1, description=$2,<br/>start_time=$3, updated_at=NOW()<br/>WHERE source_id = $4

        BK->>DB: DELETE FROM event_tag_mappings<br/>WHERE event_id = $1
        loop For each tag
            BK->>DB: INSERT INTO event_tags (name)<br/>ON CONFLICT (name) DO NOTHING
            BK->>DB: INSERT INTO event_tag_mappings<br/>(event_id, tag_id)
        end

        BK->>DB: COMMIT
    end

    BK-->>C: 200 OK<br/>{ event data }
    C-->>GAS: Forward response
```

### Key Points

- System API uses a static Bearer token (shared secret) — no session management
- The `source_id` field enables idempotent upserts — repeated syncs do not create duplicates
- Tag management uses insert-or-ignore for tag normalization and replace-all for mappings
- All database operations within a sync are wrapped in a transaction
- Discord webhook notification is sent only for new events (not updates)
- If the webhook fails, the event is still created — webhook delivery is best-effort

---

## 4. Member Leave (Discord Bot → System API)

When a member leaves or is banned from the Discord guild, the bot notifies the backend. The backend cascades the departure across all related entities in a single transaction.

```mermaid
sequenceDiagram
    participant BOT as Discord Bot
    participant C as Caddy
    participant BK as Backend (Axum)
    participant DB as PostgreSQL

    BOT->>C: POST /api/v1/system/members/leave<br/>Authorization: Bearer <system_token><br/>Body: { discord_id }
    C->>BK: Forward request

    Note over BK: Verify Bearer token

    Note over BK: ValidatedJson<MemberLeave>:<br/>Validate discord_id format

    BK->>DB: SELECT * FROM users<br/>WHERE discord_id = $1
    DB-->>BK: User row

    alt User not found
        BK-->>C: 404 Not Found
    else User found
        BK->>DB: BEGIN TRANSACTION

        Note over BK: 1. Suspend user account
        BK->>DB: UPDATE users<br/>SET status = 'suspended',<br/>updated_at = NOW()<br/>WHERE id = $1

        Note over BK: 2. Invalidate all sessions
        BK->>DB: DELETE FROM sessions<br/>WHERE user_id = $1

        Note over BK: 3. Set profile to private
        BK->>DB: UPDATE profiles<br/>SET is_public = false,<br/>updated_at = NOW()<br/>WHERE user_id = $1

        Note over BK: 4. Remove club memberships
        BK->>DB: DELETE FROM club_members<br/>WHERE user_id = $1

        BK->>DB: COMMIT

        BK-->>C: 200 OK<br/>{ status: "suspended",<br/>actions: [...] }
    end

    C-->>BOT: Forward response
```

### Key Points

- All cascading changes are wrapped in a single database transaction for atomicity
- The user is **suspended**, not deleted — data is preserved for potential reactivation
- All active sessions are immediately invalidated, forcing logout on all devices
- Profile is set to private to hide the departed member's information
- Club memberships are removed, but the user's owned clubs remain (deactivated separately if needed)
- Gallery images and reports are **not** deleted — they remain for audit purposes
- If the user returns to the guild and logs in again, their status can be reactivated by an admin

---

## Related Documents

- [System Context](system-context.md) — Actors and external systems involved in these flows
- [Components](components.md) — Internal components that process each step
- [Data Model](data-model.md) — Database tables and relationships touched by each flow
- [State Management](state-management.md) — State transitions triggered by these flows
- [Module Dependencies](module-dependency.md) — Code modules involved in request processing
