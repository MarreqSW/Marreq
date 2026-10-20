-- Use cases (issue #398, epic #388).
--
-- A use case is a project artifact with a reference code. Its content is a
-- `marreq.use-case.v1` JSON document (see src/use_case_document). Published
-- versions are immutable; edits go to one shared working draft per use case,
-- whose revision counter guards against stale writes. Publishing the draft
-- creates the next version.

CREATE TABLE use_cases (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    reference_code VARCHAR(50) NOT NULL,
    current_version_id INTEGER,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT use_cases_project_code_unique UNIQUE (project_id, reference_code)
);
CREATE INDEX idx_use_cases_project ON use_cases (project_id);

CREATE TABLE use_case_versions (
    id SERIAL PRIMARY KEY,
    use_case_id INTEGER NOT NULL REFERENCES use_cases(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    document JSONB NOT NULL,
    -- Copied from the document for lists and search.
    title VARCHAR(255) NOT NULL,
    primary_actor VARCHAR(255),
    approval_state VARCHAR(16) NOT NULL DEFAULT 'draft'
        CHECK (approval_state IN ('draft', 'reviewed', 'approved')),
    reviewed_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMP,
    approved_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMP,
    change_note TEXT,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT use_case_versions_number_unique UNIQUE (use_case_id, version_number)
);

ALTER TABLE use_cases
    ADD CONSTRAINT use_cases_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES use_case_versions(id) ON DELETE SET NULL;

-- At most one working draft per use case.
CREATE TABLE use_case_drafts (
    use_case_id INTEGER PRIMARY KEY REFERENCES use_cases(id) ON DELETE CASCADE,
    base_version_id INTEGER REFERENCES use_case_versions(id) ON DELETE SET NULL,
    document JSONB NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
