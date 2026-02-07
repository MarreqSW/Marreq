-- =============================================================================
-- Revert requirement versioning (restore requirements to flat structure)
-- =============================================================================

-- Drop new indexes
DROP INDEX IF EXISTS idx_requirements_project_stable;
DROP INDEX IF EXISTS idx_requirements_current_version_id;
DROP INDEX IF EXISTS idx_requirement_version_verification_version;
DROP INDEX IF EXISTS idx_requirement_versions_created_at;
DROP INDEX IF EXISTS idx_requirement_versions_requirement_created;
DROP INDEX IF EXISTS idx_requirement_versions_requirement_id;

-- Restore requirement columns from current version
ALTER TABLE requirements
    DROP CONSTRAINT IF EXISTS requirements_stable_code_unique,
    DROP CONSTRAINT IF EXISTS requirements_current_version_fk;

ALTER TABLE requirements ALTER COLUMN current_version_id DROP NOT NULL;

ALTER TABLE requirements
    ADD COLUMN title VARCHAR NOT NULL DEFAULT ' ',
    ADD COLUMN description VARCHAR NOT NULL DEFAULT ' ',
    ADD COLUMN status_id INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN author_id INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN reviewer_id INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN reference_code VARCHAR NOT NULL DEFAULT ' ',
    ADD COLUMN category_id INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN parent_id INTEGER,
    ADD COLUMN creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN update_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN deadline_date TIMESTAMP,
    ADD COLUMN applicability_id INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN justification TEXT;

-- Copy current version back to requirements
UPDATE requirements r
SET
    title = rv.title,
    description = rv.description,
    status_id = rv.status_id,
    author_id = rv.author_id,
    reviewer_id = rv.reviewer_id,
    reference_code = r.stable_code,
    category_id = rv.category_id,
    parent_id = rv.parent_id,
    creation_date = rv.created_at,
    update_date = rv.created_at,
    deadline_date = rv.deadline_date,
    applicability_id = rv.applicability_id,
    justification = rv.justification
FROM requirement_versions rv
WHERE r.current_version_id = rv.id;

-- Recreate requirement_verification_methods from current versions
CREATE TABLE IF NOT EXISTS requirement_verification_methods (
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    verification_method_id INTEGER NOT NULL REFERENCES verification(id),
    PRIMARY KEY (requirement_id, verification_method_id)
);

INSERT INTO requirement_verification_methods (requirement_id, verification_method_id)
SELECT rv.requirement_id, rvvm.verification_method_id
FROM requirement_version_verification_methods rvvm
JOIN requirement_versions rv ON rv.id = rvvm.requirement_version_id
JOIN requirements r ON r.id = rv.requirement_id AND r.current_version_id = rv.id
ON CONFLICT (requirement_id, verification_method_id) DO NOTHING;

-- Drop new tables (CASCADE will drop requirement_version_verification_methods refs)
DROP TABLE IF EXISTS requirement_version_verification_methods;
DROP TABLE IF EXISTS requirement_versions;

-- Drop new columns from requirements
ALTER TABLE requirements
    DROP COLUMN IF EXISTS stable_code,
    DROP COLUMN IF EXISTS current_version_id;

-- Restore constraints
ALTER TABLE requirements
    ADD CONSTRAINT requirements_reference_code_unique UNIQUE (reference_code),
    ADD CONSTRAINT requirements_title_not_blank CHECK (btrim(title) <> '');

-- Recreate search vector trigger if function exists
-- (Optional: recreate requirements_search_vector_update and trigger)
