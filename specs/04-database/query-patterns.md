# Query Patterns

## Critical Queries with Index Analysis

### Q1: Public Member List

```sql
SELECT u.discord_id, u.discord_display_name, u.discord_avatar_hash, u.joined_at,
       p.nickname, p.vrc_id, p.x_id, p.bio_html, p.avatar_url
FROM users u
LEFT JOIN profiles p ON p.user_id = u.id AND p.is_public = true
WHERE u.status = 'active'
ORDER BY u.joined_at DESC
LIMIT $1 OFFSET $2;
```

**Index usage:**
- `idx_users_status` → filters active users
- `idx_profiles_is_public` (partial: `WHERE is_public = true`) → efficient join condition on profiles

**Count query (for pagination):**
```sql
SELECT COUNT(*) FROM users u
WHERE u.status = 'active';
```

**Expected performance:** < 5ms for 1,000 users (sequential scan is fine at this scale, indexes ensure optimal plan as data grows).

> **Note:** Uses LEFT JOIN so all active members appear in the list. Members without a public profile will have NULL profile fields.

### Q2: Session Lookup (on every authenticated request)

```sql
SELECT s.id, s.user_id, s.expires_at,
       u.id, u.discord_id, u.discord_username, u.avatar_url, u.role, u.status
FROM sessions s
JOIN users u ON s.user_id = u.id
WHERE s.token_hash = $1
  AND s.expires_at > now();
```

**Index usage:**
- `idx_sessions_token_hash` UNIQUE index → direct hash lookup

**Expected performance:** < 1ms. This is the hottest query — executes on every Internal API request.

### Q3: OAuth2 Login — Upsert User

```sql
INSERT INTO users (discord_id, discord_username, avatar_url)
VALUES ($1, $2, $3)
ON CONFLICT (discord_id) DO UPDATE SET
    discord_username = EXCLUDED.discord_username,
    avatar_url = EXCLUDED.avatar_url,
    updated_at = now()
RETURNING id, discord_id, discord_username, avatar_url, role, status;
```

**Index usage:**
- `idx_users_discord_id` UNIQUE index → conflict detection + lookup

### Q4: Event Upsert (System API)

```sql
-- Check existence
SELECT id FROM events WHERE external_source_id = $1;

-- Insert (if not exists)
INSERT INTO events (external_source_id, title, description_markdown, host_user_id, host_name, event_status, start_time, end_time, location)
VALUES ($1, $2, $3, $4, $5, 'published', $6, $7, $8)
RETURNING id;

-- Update (if exists)
UPDATE events SET title=$2, description_markdown=$3, host_user_id=$4, host_name=$5, start_time=$6, end_time=$7, location=$8, updated_at=now()
WHERE id = $1
RETURNING id;
```

**Index usage:**
- `idx_events_external_source_id` (partial unique) → dedup check

### Q5: Member Leave — Atomic Suspension

```sql
BEGIN;

UPDATE users SET status = 'suspended', updated_at = now()
WHERE discord_id = $1 AND status = 'active'
RETURNING id;

DELETE FROM sessions WHERE user_id = $1;

UPDATE profiles SET is_public = false, updated_at = now()
WHERE user_id = $1;

COMMIT;
```

**Index usage:**
- `idx_users_discord_id` → user lookup
- `idx_sessions_user_id` → session deletion
- `profiles(user_id)` PK → profile update

**Expected performance:** < 10ms (three simple operations in one transaction).

### Q6: Event Archival (Background Task)

```sql
UPDATE events SET event_status = 'archived', updated_at = now()
WHERE event_status NOT IN ('cancelled', 'archived')
  AND end_time IS NOT NULL
  AND end_time < now();
```

**Index usage:**
- `idx_events_archival` (partial: `WHERE event_status NOT IN (...)`) → efficient scan of only archivable events

### Q7: Session Cleanup (Background Task)

```sql
DELETE FROM sessions WHERE expires_at < now();
```

**Index usage:**
- `idx_sessions_expires_at` → range scan on expired sessions

### Q8: Public Gallery (Approved Images for Club)

```sql
SELECT id, club_id, uploaded_by, image_url, caption, status, created_at
FROM gallery_images
WHERE club_id = $1 AND status = 'approved'
ORDER BY created_at DESC
LIMIT $2 OFFSET $3;
```

**Index usage:**
- `idx_gallery_images_club_approved` (partial: `WHERE status = 'approved'`) → covers this exact query

### Q9: Duplicate Report Check

```sql
SELECT EXISTS (
    SELECT 1 FROM reports
    WHERE reporter_user_id = $1 AND target_type = $2 AND target_id = $3
);
```

**Index usage:**
- `idx_reports_unique_per_user` UNIQUE index → direct lookup

### Q10: Admin User List with Filters

```sql
SELECT id, discord_id, discord_username, avatar_url, role, status, created_at, updated_at
FROM users
WHERE ($1::user_role IS NULL OR role = $1)
  AND ($2::user_status IS NULL OR status = $2)
ORDER BY created_at DESC
LIMIT $3 OFFSET $4;
```

**Index usage:**
- `idx_users_role` and `idx_users_status` → conditional filter (PostgreSQL optimizer picks the most selective index)

**Note:** At < 1,000 users, PostgreSQL will likely prefer sequential scan. The indexes become valuable if the table grows beyond ~5,000 rows.
