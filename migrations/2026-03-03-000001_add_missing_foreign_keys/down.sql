-- =============================================================================
-- Rollback: Remove missing-FK constraints added in the forward migration.
-- =============================================================================
-- Drops constraints and the category cross-project trigger, then restores the
-- DEFAULT sentinel values that existed before this migration.
--
-- NOTE: Rolling back does NOT restore orphaned rows that were healed during the
-- forward migration.  The sentinels (DEFAULT 0 / DEFAULT 1) are re-added so
-- that old application code that omits those columns continues to function, but
-- any rows that were repointed to valid IDs by the cleanup pass retain those IDs.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 5. tests.parent_id  (reverse order for clarity)
-- ---------------------------------------------------------------------------
ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_parent_id_fk;

-- ---------------------------------------------------------------------------
-- 4. requirement_versions.category_id
-- ---------------------------------------------------------------------------
DROP TRIGGER  IF EXISTS rv_category_project_consistency ON requirement_versions;
DROP FUNCTION IF EXISTS check_rv_category_project_consistency();

ALTER TABLE requirement_versions
    DROP CONSTRAINT IF EXISTS requirement_versions_category_id_fk;

ALTER TABLE requirement_versions
    ALTER COLUMN category_id SET DEFAULT 1;

-- ---------------------------------------------------------------------------
-- 3. requirement_versions.reviewer_id
-- ---------------------------------------------------------------------------
ALTER TABLE requirement_versions
    DROP CONSTRAINT IF EXISTS requirement_versions_reviewer_id_fk;

ALTER TABLE requirement_versions
    ALTER COLUMN reviewer_id SET DEFAULT 0;

-- ---------------------------------------------------------------------------
-- 2. requirement_versions.author_id
-- ---------------------------------------------------------------------------
ALTER TABLE requirement_versions
    DROP CONSTRAINT IF EXISTS requirement_versions_author_id_fk;

ALTER TABLE requirement_versions
    ALTER COLUMN author_id SET DEFAULT 0;

-- ---------------------------------------------------------------------------
-- 1. projects.owner_id
-- ---------------------------------------------------------------------------
ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_owner_id_fk;
