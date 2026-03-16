# Auth API Endpoints

Authentication uses Discord OAuth2 Authorization Code flow. Rate limited at 10 req/min/IP to prevent abuse.

---

## GET `/api/v1/auth/discord/login`

Redirect user to Discord's authorization page.

### Query Parameters

| Param | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `redirect_to` | string | No | `/` | Frontend URL to redirect after successful auth (validated against allowlist) |

### Response — `302 Found`

```
Location: https://discord.com/oauth2/authorize
  ?client_id=<DISCORD_CLIENT_ID>
  &redirect_uri=<BACKEND_BASE_URL>/api/v1/auth/discord/callback
  &response_type=code
  &scope=identify+guilds
  &state=<signed_state_token>
```

### State Token

The `state` parameter is a signed HMAC-SHA256 token containing:

```json
{
  "nonce": "<random 32-byte hex>",
  "redirect_to": "/dashboard",
  "expires_at": "<now + 10 minutes>"
}
```

Signed with `SESSION_SECRET`. The nonce is also stored in a short-lived HttpOnly cookie (`oauth_state`) to prevent CSRF during the OAuth flow.

### Server-Side Processing

1. Generate cryptographically random nonce (32 bytes)
2. Build state payload with nonce, redirect_to, and expiry
3. HMAC-SHA256 sign the payload
4. Set `oauth_state` cookie (HttpOnly, Secure, SameSite=Lax, Max-Age=600)
5. Redirect to Discord authorization URL

---

## GET `/api/v1/auth/discord/callback`

Handle the OAuth2 callback from Discord.

### Query Parameters (set by Discord)

| Param | Type | Description |
|-------|------|-------------|
| `code` | string | Authorization code |
| `state` | string | Signed state token from login |

### Success Flow → `302 Found`

```
Location: <FRONTEND_ORIGIN><redirect_to>
Set-Cookie: session_id=<session_token>; 
  Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800
```

### Error Flows

| Condition | Response |
|-----------|----------|
| `state` signature invalid | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=invalid_state` |
| `state` expired (> 10 min) | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=expired` |
| `oauth_state` cookie nonce mismatch | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=csrf` |
| Discord token exchange fails | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=discord_error` |
| User is not in the guild | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=not_member` |
| User is suspended | `302` → `<FRONTEND_ORIGIN>/auth/error?reason=suspended` |

All error cases redirect to the frontend error page (never expose raw error details in the callback URL).

### Server-Side Processing (Step by Step)

1. **Verify state token**:
   - Extract state from query
   - Verify HMAC-SHA256 signature with `SESSION_SECRET`
   - Check expiry
   - Compare nonce with `oauth_state` cookie
   - Delete `oauth_state` cookie

2. **Exchange code for tokens**:
   ```
   POST https://discord.com/api/v10/oauth2/token
   Content-Type: application/x-www-form-urlencoded

   client_id=<DISCORD_CLIENT_ID>
   &client_secret=<DISCORD_CLIENT_SECRET>
   &grant_type=authorization_code
   &code=<code>
   &redirect_uri=<callback_url>
   ```

3. **Fetch user identity**:
   ```
   GET https://discord.com/api/v10/users/@me
   Authorization: Bearer <access_token>
   ```

4. **Verify guild membership**:
   ```
   GET https://discord.com/api/v10/users/@me/guilds
   Authorization: Bearer <access_token>
   ```
   Check if `DISCORD_GUILD_ID` is in the response. If not → redirect with `not_member`.

5. **Upsert user**:
   ```sql
   INSERT INTO users (discord_id, discord_display_name, discord_avatar_hash, status, role, joined_at)
   VALUES ($1, $2, $3, 'active', 'member', NOW())
   ON CONFLICT (discord_id) DO UPDATE SET
     discord_display_name = EXCLUDED.discord_display_name,
     discord_avatar_hash = EXCLUDED.discord_avatar_hash,
     last_login_at = NOW()
   RETURNING id, status, role;
   ```

6. **Check suspension**: If returned `status = 'suspended'` → redirect with `suspended`.

7. **Create session**:
   - Generate 32-byte cryptographically random session token
   - Hash the token with SHA-256 for storage (raw token goes to cookie, hash to DB)
   ```sql
   INSERT INTO sessions (user_id, token_hash, expires_at)
   VALUES ($1, $2, NOW() + INTERVAL '7 days');
   ```

8. **Cleanup expired sessions** (async background task):
   ```sql
   DELETE FROM sessions WHERE expires_at < NOW();
   ```

9. **Set session cookie and redirect** to `redirect_to` from state token.

### Session Token Security

| Property | Value |
|----------|-------|
| Generation | `ring::rand::SystemRandom` → 32 bytes |
| Storage | SHA-256 hashed in `sessions.token_hash` |
| Cookie name | `session_id` |
| Cookie flags | `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/` |
| Max lifetime | 7 days (`Max-Age=604800`) |
| Validation | SHA-256 hash raw cookie → lookup in `sessions` → check `expires_at` |

The raw session token NEVER touches the database. Only the hash is stored. This means a database breach does not compromise active sessions (attacker cannot reconstruct the cookie value from the hash).

### Rust Types

```rust
#[derive(Deserialize)]
pub struct LoginQuery {
    pub redirect_to: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Internal: never serialized to client
pub struct OAuthState {
    pub nonce: [u8; 32],
    pub redirect_to: String,
    pub expires_at: DateTime<Utc>,
}

/// Discord API response
#[derive(Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Deserialize)]
pub struct DiscordGuild {
    pub id: String,
}
```
