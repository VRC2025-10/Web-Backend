# API Examples

> **Navigation**: [Docs Home](../README.md) > [Getting Started](README.md) > Examples

Curated examples for interacting with the VRC Backend API using curl. All examples assume the server is running at `http://localhost:3000`.

---

## Table of Contents

- [Health Check](#health-check)
- [Public API](#public-api) — No authentication required
- [Auth Flow](#auth-flow) — Discord OAuth2 login
- [Internal API](#internal-api) — Requires session cookie
- [System API](#system-api) — Requires Bearer token
- [Admin API](#admin-api) — Requires admin session

---

## Health Check

```bash
curl http://localhost:3000/health
```

Returns the server health status including database connectivity.

---

## Public API

Public endpoints require no authentication. Base path: `/api/v1/public`

### List Members (Paginated)

```bash
# Default pagination
curl http://localhost:3000/api/v1/public/members

# With pagination parameters
curl "http://localhost:3000/api/v1/public/members?page=1&per_page=20"
```

### Get Member Detail

```bash
curl http://localhost:3000/api/v1/public/members/{member_id}
```

### List Events

```bash
# All events
curl http://localhost:3000/api/v1/public/events

# Filter by status
curl "http://localhost:3000/api/v1/public/events?status=upcoming"
```

### Get Event Detail

```bash
curl http://localhost:3000/api/v1/public/events/{event_id}
```

### List Clubs

```bash
curl http://localhost:3000/api/v1/public/clubs
```

### Get Club Detail

```bash
curl http://localhost:3000/api/v1/public/clubs/{club_id}
```

### List Gallery Images

```bash
curl http://localhost:3000/api/v1/public/gallery
```

---

## Auth Flow

The VRC Backend uses Discord OAuth2 for authentication. Base path: `/api/v1/auth`

### Step 1: Initiate Login

Redirect the user's browser to the login endpoint:

```bash
# This returns a redirect to Discord's OAuth2 authorization page
curl -v http://localhost:3000/api/v1/auth/login
```

In a web application, navigate the user to this URL:

```
http://localhost:3000/api/v1/auth/login
```

### Step 2: Discord Authorization

The user authorizes your application on Discord. Discord then redirects back to your configured `DISCORD_REDIRECT_URI`.

### Step 3: Callback

The callback is handled automatically by the server at:

```
GET /api/v1/auth/callback?code=<authorization_code>&state=<state>
```

On success, the server:
1. Exchanges the authorization code for a Discord access token
2. Fetches the user's Discord profile and guild membership
3. Creates or updates the user record
4. Sets a session cookie
5. Redirects to `FRONTEND_ORIGIN`

### Step 4: Use the Session

After login, the session cookie is included automatically in browser requests. For curl, capture and reuse it:

```bash
# Store cookies from the auth flow
curl -c cookies.txt -L http://localhost:3000/api/v1/auth/login
# (Complete the Discord OAuth2 flow in your browser)

# Use the session cookie for authenticated requests
curl -b cookies.txt http://localhost:3000/api/v1/internal/users/me
```

---

## Internal API

Internal endpoints require an authenticated session (cookie). Base path: `/api/v1/internal`

> **Note**: Replace `-b cookies.txt` with your actual session cookie. You can obtain one by completing the [Auth Flow](#auth-flow).

### Get Current User

```bash
curl -b cookies.txt http://localhost:3000/api/v1/internal/users/me
```

### Get Profile

```bash
curl -b cookies.txt http://localhost:3000/api/v1/internal/profile
```

### Update Profile

```bash
curl -b cookies.txt \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{
    "nickname": "New Nickname",
    "bio": "Updated bio text"
  }' \
  http://localhost:3000/api/v1/internal/profile
```

### Create Report

```bash
curl -b cookies.txt \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "target_type": "member",
    "target_id": "target-uuid-here",
    "reason": "Reason for the report"
  }' \
  http://localhost:3000/api/v1/internal/reports
```

---

## System API

System endpoints are for service-to-service communication and require a Bearer token (`SYSTEM_API_TOKEN`). Base path: `/api/v1/system`

### Upsert Event

```bash
curl -X PUT \
  -H "Authorization: Bearer $SYSTEM_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Weekly Meetup",
    "description": "Our regular weekly gathering",
    "start_time": "2026-03-20T18:00:00Z",
    "end_time": "2026-03-20T20:00:00Z",
    "location": "VRChat World"
  }' \
  http://localhost:3000/api/v1/system/events
```

### Handle Member Leave

```bash
curl -X POST \
  -H "Authorization: Bearer $SYSTEM_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "discord_id": "123456789012345678"
  }' \
  http://localhost:3000/api/v1/system/members/leave
```

---

## Admin API

Admin endpoints require an authenticated session with admin privileges. Base path: `/api/v1/internal/admin`

> **Note**: Your user account must have an admin role. Set `SUPER_ADMIN_DISCORD_ID` in `.env` to your Discord ID to bootstrap the first admin.

### List Users with Filters

```bash
# All users
curl -b cookies.txt http://localhost:3000/api/v1/internal/admin/users

# Filter by role
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?role=member"

# Filter by status
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?status=active"

# Paginated
curl -b cookies.txt "http://localhost:3000/api/v1/internal/admin/users?page=1&per_page=25"
```

### Change User Role

```bash
curl -b cookies.txt \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{
    "role": "moderator"
  }' \
  http://localhost:3000/api/v1/internal/admin/users/{user_id}/role
```

### Change User Status

```bash
curl -b cookies.txt \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{
    "status": "suspended"
  }' \
  http://localhost:3000/api/v1/internal/admin/users/{user_id}/status
```

---

## Tips

- **Pagination**: Most list endpoints support `?page=N&per_page=N` query parameters.
- **JSON responses**: All API responses are JSON. Pipe through `jq` for readable output:
  ```bash
  curl -s http://localhost:3000/api/v1/public/members | jq .
  ```
- **Verbose output**: Use `curl -v` to inspect headers and status codes.
- **Environment variable**: Export your system token for convenience:
  ```bash
  export SYSTEM_API_TOKEN=$(grep SYSTEM_API_TOKEN .env | cut -d= -f2)
  ```

---

## Related Documents

- [Public API Reference](../../api/public.md)
- [Internal API Reference](../../api/internal.md)
- [System API Reference](../../api/system.md)
- [Auth API Reference](../../api/auth.md)
- [Error Handling](../../../specs/05-api/error-handling.md)
- [Pagination](../../../specs/05-api/pagination.md)
- [Rate Limiting](../../../specs/05-api/rate-limiting.md)
