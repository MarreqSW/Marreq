-- Add requirement_version_links table for version-to-version parent/derives-from links.
-- Safe to run if the table was created by the baseline migration (CREATE TABLE IF NOT EXISTS not used
-- so that diesel migration run is idempotent; run this only on DBs that lack the table).

CREATE TABLE IF NOT EXISTS requirement_version_links (
    id SERIAL PRIMARY KEY,
    source_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    target_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    link_type VARCHAR(32) NOT NULL,
    rationale TEXT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    metadata JSONB
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_rvl_source_target_type ON requirement_version_links(source_version_id, target_version_id, link_type);
CREATE INDEX IF NOT EXISTS idx_rvl_source ON requirement_version_links(source_version_id);
CREATE INDEX IF NOT EXISTS idx_rvl_target ON requirement_version_links(target_version_id);
CREATE INDEX IF NOT EXISTS idx_rvl_project ON requirement_version_links(project_id);
