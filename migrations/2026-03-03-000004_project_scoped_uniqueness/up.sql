-- =============================================================================
-- Project-scoped uniqueness constraints
-- =============================================================================
-- Relax tests.reference_code to unique per project.
-- Add (project_id, tag) uniqueness for requirement_status, test_status,
-- categories, applicability, verification.
-- =============================================================================

-- Tests: project-scoped reference_code
ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_reference_code_unique;

ALTER TABLE tests
    ADD CONSTRAINT tests_project_id_reference_code_unique UNIQUE (project_id, reference_code);

-- Status tables: unique tag per project
ALTER TABLE requirement_status
    ADD CONSTRAINT requirement_status_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE test_status
    ADD CONSTRAINT test_status_project_id_tag_unique UNIQUE (project_id, tag);

-- Taxonomy tables: unique tag per project
ALTER TABLE categories
    ADD CONSTRAINT categories_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE applicability
    ADD CONSTRAINT applicability_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE verification
    ADD CONSTRAINT verification_project_id_tag_unique UNIQUE (project_id, tag);
