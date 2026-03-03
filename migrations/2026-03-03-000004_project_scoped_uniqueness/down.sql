-- =============================================================================
-- Revert project-scoped uniqueness constraints
-- =============================================================================
-- Note: Restoring tests_reference_code_unique will fail if the database
-- contains the same reference_code in more than one project. Resolve duplicates
-- before running this down migration.
-- =============================================================================

-- Taxonomy: drop project-scoped unique
ALTER TABLE verification
    DROP CONSTRAINT IF EXISTS verification_project_id_tag_unique;

ALTER TABLE applicability
    DROP CONSTRAINT IF EXISTS applicability_project_id_tag_unique;

ALTER TABLE categories
    DROP CONSTRAINT IF EXISTS categories_project_id_tag_unique;

-- Status tables
ALTER TABLE test_status
    DROP CONSTRAINT IF EXISTS test_status_project_id_tag_unique;

ALTER TABLE requirement_status
    DROP CONSTRAINT IF EXISTS requirement_status_project_id_tag_unique;

-- Tests: restore global uniqueness on reference_code
ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_project_id_reference_code_unique;

ALTER TABLE tests
    ADD CONSTRAINT tests_reference_code_unique UNIQUE (reference_code);
