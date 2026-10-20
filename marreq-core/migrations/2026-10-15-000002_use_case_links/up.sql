-- Traceability links from use cases (issue #400, epic #388).
--
-- A link anchors a use case, or one element of it (`element_id`, a step or
-- condition id of its document), to a requirement or a verification. Links
-- point at the artifact, not at a version, as the matrix does;
-- `linked_requirement_version_id` records the requirement version the link
-- was made or last cleared against. When the requirement changes, the link
-- goes suspect like a matrix row. A verification link never touches the
-- matrix.

CREATE TABLE use_case_links (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    use_case_id INTEGER NOT NULL REFERENCES use_cases(id) ON DELETE CASCADE,
    element_id VARCHAR(40),
    requirement_id INTEGER REFERENCES requirements(id) ON DELETE CASCADE,
    verification_id INTEGER REFERENCES verifications(id) ON DELETE CASCADE,
    linked_requirement_version_id INTEGER REFERENCES requirement_versions(id) ON DELETE SET NULL,
    rationale TEXT,
    suspect BOOLEAN NOT NULL DEFAULT false,
    suspect_at TIMESTAMP,
    suspect_reason TEXT,
    triggering_version_id INTEGER REFERENCES requirement_versions(id) ON DELETE SET NULL,
    triggering_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    cleared_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    cleared_at TIMESTAMP,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    CONSTRAINT use_case_links_one_target CHECK ((requirement_id IS NULL) <> (verification_id IS NULL))
);

CREATE UNIQUE INDEX use_case_links_unique ON use_case_links (
    use_case_id, COALESCE(element_id, ''), COALESCE(requirement_id, 0), COALESCE(verification_id, 0)
);
CREATE INDEX idx_use_case_links_project ON use_case_links (project_id);
CREATE INDEX idx_use_case_links_requirement ON use_case_links (requirement_id) WHERE requirement_id IS NOT NULL;
CREATE INDEX idx_use_case_links_verification ON use_case_links (verification_id) WHERE verification_id IS NOT NULL;
