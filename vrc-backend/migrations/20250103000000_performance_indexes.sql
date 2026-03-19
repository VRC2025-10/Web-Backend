-- Performance indexes for hot query paths
-- Identified through query analysis of all SQL statements in the codebase

CREATE INDEX IF NOT EXISTS idx_sessions_token_hash_expires
    ON sessions (token_hash, expires_at);
-- Public member listing: users WHERE status = 'active' ORDER BY joined_at DESC
-- The existing idx_users_status is non-covering. Add a composite for the sort.
CREATE INDEX IF NOT EXISTS idx_users_active_joined
    ON users (joined_at DESC)
    WHERE status = 'active';

-- Reports admin filtering by status with ORDER BY created_at DESC
CREATE INDEX IF NOT EXISTS idx_reports_status_created
    ON reports (status, created_at DESC);

-- Events listing: ORDER BY start_time DESC with optional status filter
-- Existing idx_events_status and idx_events_start_time are separate.
-- Composite index enables index-only scan for the common query pattern.
CREATE INDEX IF NOT EXISTS idx_events_status_start_time
    ON events (event_status, start_time DESC);

-- Gallery images: club_id + status = 'approved' is already indexed.
-- Add index for the admin panel which queries all statuses per club.
-- Already exists as idx_gallery_images_club_all - no action needed.

-- Club member count subquery optimization: ensure club_id index for COUNT(*)
-- Already have PRIMARY KEY (club_id, user_id) which covers this. No action needed.

-- Event tag mappings: the batch tag fetch uses event_id = ANY($1)
-- Already have PRIMARY KEY (event_id, tag_id). No action needed.

-- Background session cleanup: DELETE WHERE expires_at < now()
-- idx_sessions_expires_at already exists. No action needed.
