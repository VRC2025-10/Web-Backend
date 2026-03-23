CREATE TYPE gallery_target_type AS ENUM ('community', 'club');

ALTER TABLE gallery_images
    ADD COLUMN target_type gallery_target_type NOT NULL DEFAULT 'club';

ALTER TABLE gallery_images
    ALTER COLUMN club_id DROP NOT NULL;

ALTER TABLE gallery_images
    ADD CONSTRAINT gallery_images_target_scope_check
    CHECK (
        (target_type = 'community' AND club_id IS NULL)
        OR (target_type = 'club' AND club_id IS NOT NULL)
    );

DROP INDEX IF EXISTS idx_gallery_images_club_approved;
DROP INDEX IF EXISTS idx_gallery_images_club_all;

CREATE INDEX idx_gallery_images_club_approved
    ON gallery_images (club_id, created_at DESC)
    WHERE status = 'approved' AND target_type = 'club';

CREATE INDEX idx_gallery_images_club_all
    ON gallery_images (club_id, created_at DESC)
    WHERE target_type = 'club';

CREATE INDEX idx_gallery_images_target_status_created
    ON gallery_images (target_type, status, created_at DESC);