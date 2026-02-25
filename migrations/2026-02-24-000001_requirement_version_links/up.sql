-- =============================================================================
-- Requirement version links (multi-parent, typed traceability) — issue #118
-- =============================================================================
-- Links requirement_versions (version-to-version) with typed relationship.
-- requirement_versions.parent_id is retained but deprecated; backfill creates
-- one DERIVES_FROM link per existing parent_id (parent = requirement container id).
-- =============================================================================

CREATE TABLE requirement_version_links (
    id SERIAL PRIMARY KEY,
    source_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    target_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    link_type VARCHAR(32) NOT NULL,
    rationale TEXT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    metadata JSONB
);

CREATE UNIQUE INDEX idx_rvl_source_target_type ON requirement_version_links(source_version_id, target_version_id, link_type);
CREATE INDEX idx_rvl_source ON requirement_version_links(source_version_id);
CREATE INDEX idx_rvl_target ON requirement_version_links(target_version_id);
CREATE INDEX idx_rvl_project ON requirement_version_links(project_id);

-- Backfill: requirement_versions.parent_id is the parent *requirement* (container) id.
-- Resolve parent's current_version_id at migration time and insert one DERIVES_FROM link.
INSERT INTO requirement_version_links (source_version_id, target_version_id, link_type, project_id, created_at)
SELECT rv.id, r_parent.current_version_id, 'DERIVES_FROM', r.project_id, now()
FROM requirement_versions rv
JOIN requirements r ON r.id = rv.requirement_id
JOIN requirements r_parent ON r_parent.id = rv.parent_id
WHERE rv.parent_id IS NOT NULL
  AND r_parent.current_version_id IS NOT NULL
ON CONFLICT (source_version_id, target_version_id, link_type) DO NOTHING;
