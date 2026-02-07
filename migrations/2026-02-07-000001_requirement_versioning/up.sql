-- =============================================================================
-- Immutable requirement versioning (issue #94)
-- =============================================================================
-- requirements becomes a logical container (id, project_id, stable_code, current_version_id).
-- requirement_versions holds immutable snapshots; one version per requirement is "current".
-- Traceability (matrix, requirement_embeddings) keeps using requirements.id.
-- =============================================================================

-- 1. Create requirement_versions table (immutable version rows)
CREATE TABLE requirement_versions (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 1 REFERENCES requirement_status(id),
    author_id INTEGER NOT NULL DEFAULT 0,
    reviewer_id INTEGER NOT NULL DEFAULT 0,
    category_id INTEGER NOT NULL DEFAULT 1,
    parent_id INTEGER,
    applicability_id INTEGER NOT NULL DEFAULT 1 REFERENCES applicability(id),
    justification TEXT,
    deadline_date TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_title_not_blank CHECK (btrim(title) <> '');

-- 2. Junction table for verification methods per version
CREATE TABLE requirement_version_verification_methods (
    requirement_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    verification_method_id INTEGER NOT NULL REFERENCES verification(id),
    PRIMARY KEY (requirement_version_id, verification_method_id)
);

-- 3. Add new columns to requirements (keep old columns for data copy)
ALTER TABLE requirements
    ADD COLUMN IF NOT EXISTS stable_code VARCHAR NOT NULL DEFAULT ' ',
    ADD COLUMN IF NOT EXISTS current_version_id INTEGER;

-- 4. Migrate existing rows into initial version (v1)
INSERT INTO requirement_versions (
    requirement_id,
    title,
    description,
    status_id,
    author_id,
    reviewer_id,
    category_id,
    parent_id,
    applicability_id,
    justification,
    deadline_date,
    created_at
)
SELECT
    id,
    title,
    description,
    status_id,
    author_id,
    reviewer_id,
    category_id,
    parent_id,
    applicability_id,
    justification,
    deadline_date,
    COALESCE(update_date, creation_date)
FROM requirements;

-- 5. Set stable_code and current_version_id (one version per requirement, same insert order)
UPDATE requirements r
SET
    stable_code = COALESCE(r.reference_code, ' '),
    current_version_id = (
        SELECT rv.id
        FROM requirement_versions rv
        WHERE rv.requirement_id = r.id
        ORDER BY rv.id ASC
        LIMIT 1
    );

-- 6. Copy verification methods into versioned table
-- Use requirement_verification_methods (M:N) if it exists, else requirements.verification_method_id (baseline)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'requirement_verification_methods') THEN
        INSERT INTO requirement_version_verification_methods (requirement_version_id, verification_method_id)
        SELECT rv.id, rvm.verification_method_id
        FROM requirement_versions rv
        JOIN requirement_verification_methods rvm ON rvm.requirement_id = rv.requirement_id;
    ELSE
        INSERT INTO requirement_version_verification_methods (requirement_version_id, verification_method_id)
        SELECT rv.id, r.verification_method_id
        FROM requirement_versions rv
        JOIN requirements r ON r.id = rv.requirement_id
        WHERE r.verification_method_id IS NOT NULL AND r.verification_method_id > 0;
    END IF;
END $$;

-- 7. Drop full-text trigger on requirements (columns will be removed)
DROP TRIGGER IF EXISTS requirements_search_vector_trigger ON requirements;

-- 8. Drop old requirement columns
ALTER TABLE requirements
    DROP COLUMN IF EXISTS title,
    DROP COLUMN IF EXISTS description,
    DROP COLUMN IF EXISTS status_id,
    DROP COLUMN IF EXISTS author_id,
    DROP COLUMN IF EXISTS reviewer_id,
    DROP COLUMN IF EXISTS reference_code,
    DROP COLUMN IF EXISTS category_id,
    DROP COLUMN IF EXISTS parent_id,
    DROP COLUMN IF EXISTS creation_date,
    DROP COLUMN IF EXISTS update_date,
    DROP COLUMN IF EXISTS deadline_date,
    DROP COLUMN IF EXISTS applicability_id,
    DROP COLUMN IF EXISTS justification,
    DROP COLUMN IF EXISTS search_vector,
    DROP COLUMN IF EXISTS verification_method_id;

-- 9. Drop old constraints that referenced dropped columns
ALTER TABLE requirements DROP CONSTRAINT IF EXISTS requirements_reference_code_unique;
ALTER TABLE requirements DROP CONSTRAINT IF EXISTS requirements_title_not_blank;

-- 10. Add FK for current_version_id (nullable to allow insert-then-update for new requirements)
ALTER TABLE requirements
    ADD CONSTRAINT requirements_current_version_fk
        FOREIGN KEY (current_version_id) REFERENCES requirement_versions(id);
-- After migration all existing rows have current_version_id set; new rows get it set in same transaction

-- 11. Unique stable code (per project to allow same code in different projects)
ALTER TABLE requirements
    ADD CONSTRAINT requirements_stable_code_unique UNIQUE (project_id, stable_code);

-- 12. Drop legacy M:N table if present (app will use requirement_version_verification_methods)
DROP TABLE IF EXISTS requirement_verification_methods;

-- 13. Indexes for history and current-version lookups
CREATE INDEX idx_requirement_versions_requirement_id ON requirement_versions(requirement_id);
CREATE INDEX idx_requirement_versions_requirement_created ON requirement_versions(requirement_id, created_at DESC);
CREATE INDEX idx_requirement_versions_created_at ON requirement_versions(created_at DESC);

CREATE INDEX idx_requirement_version_verification_version ON requirement_version_verification_methods(requirement_version_id);
CREATE INDEX idx_requirements_current_version_id ON requirements(current_version_id);
CREATE INDEX idx_requirements_project_stable ON requirements(project_id, stable_code);
