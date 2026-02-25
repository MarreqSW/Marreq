-- =============================================================================
-- Drop deprecated parent_id column from requirement_versions.
-- All parent relationships now live exclusively in requirement_version_links.
-- The 2026-02-24 migration already backfilled existing parent_id values into
-- requirement_version_links rows, so no data is lost.
-- =============================================================================

-- Drop the index on the old requirements table (legacy — pre-versioning).
DROP INDEX IF EXISTS idx_requirements_parent;

-- Drop the column from requirement_versions.
ALTER TABLE requirement_versions DROP COLUMN IF EXISTS parent_id;
