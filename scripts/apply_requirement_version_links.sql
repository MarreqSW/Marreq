-- =============================================================================
-- Add requirement_version_links table if missing
-- =============================================================================
-- Use this when your database was created from an older init_complete.sql
-- (or setup_database.sh) that did not include requirement_version_links.
-- The app needs this table for requirement parent/child links and show requirement.
--
-- Usage (from project root, with DB reachable):
--   psql "$DATABASE_URL" -f scripts/apply_requirement_version_links.sql
--
-- Or with Docker Compose (from project root; uses db service and reqman DB):
--   docker compose exec -T db psql -U rust -d reqman < scripts/apply_requirement_version_links.sql
-- =============================================================================

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
