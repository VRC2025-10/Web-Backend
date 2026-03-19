-- Spec compliance migration:
-- 1. Rename report_status 'pending' → 'open' (matches API spec response)
-- 2. Change reports.target_id from UUID to TEXT (spec uses String identifiers)

-- Rename 'pending' to 'open' in report_status enum
ALTER TYPE report_status RENAME VALUE 'pending' TO 'open';

-- Change reports.target_id from UUID to TEXT
ALTER TABLE reports ALTER COLUMN target_id TYPE TEXT USING target_id::TEXT;
