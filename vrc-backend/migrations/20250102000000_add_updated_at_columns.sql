-- Add updated_at columns to reports and gallery_images tables
-- Required for Phase 3 admin endpoints (report resolution, gallery review)

ALTER TABLE reports
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE gallery_images
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
