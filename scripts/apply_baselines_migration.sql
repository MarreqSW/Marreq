-- =============================================================================
-- Apply only the immutable baselines migration
-- =============================================================================
-- Use this when your database already has the full schema (e.g. from
-- init_complete.sql or earlier migrations) but does not have the baselines
-- tables. Running "diesel migration run" fails because an earlier migration
-- (e.g. 2026-01-31) tries to create tables that already exist.
--
-- Usage (from project root, with DB reachable):
--   psql "$DATABASE_URL" -f scripts/apply_baselines_migration.sql
--
-- Or with Docker (replace CONTAINER and DB name if needed):
--   docker exec -i CONTAINER psql -U postgres -d reqman < scripts/apply_baselines_migration.sql
-- =============================================================================

-- Baselines: one row per snapshot (name, description, created_at, created_by).
CREATE TABLE IF NOT EXISTS baselines (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_baselines_project_id ON baselines(project_id);
CREATE INDEX IF NOT EXISTS idx_baselines_created_at ON baselines(created_at DESC);

-- Snapshot of requirement versions in the baseline (requirement_id -> version_id at baseline time).
CREATE TABLE IF NOT EXISTS baseline_requirements (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (baseline_id, requirement_id)
);

CREATE INDEX IF NOT EXISTS idx_baseline_requirements_baseline_id ON baseline_requirements(baseline_id);
CREATE INDEX IF NOT EXISTS idx_baseline_requirements_version_id ON baseline_requirements(version_id);

-- Snapshot of traceability matrix at baseline time.
CREATE TABLE IF NOT EXISTS baseline_traceability (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    test_id INTEGER NOT NULL REFERENCES tests(id) ON DELETE CASCADE,
    PRIMARY KEY (baseline_id, requirement_id, test_id)
);

CREATE INDEX IF NOT EXISTS idx_baseline_traceability_baseline_id ON baseline_traceability(baseline_id);

-- Immutability: forbid UPDATE and DELETE on baselines and child tables
CREATE OR REPLACE FUNCTION forbid_baseline_update_delete() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'Baselines are immutable: UPDATE and DELETE are not allowed';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS baselines_immutable ON baselines;
CREATE TRIGGER baselines_immutable
    BEFORE UPDATE OR DELETE ON baselines
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();

DROP TRIGGER IF EXISTS baseline_requirements_immutable ON baseline_requirements;
CREATE TRIGGER baseline_requirements_immutable
    BEFORE UPDATE OR DELETE ON baseline_requirements
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();

DROP TRIGGER IF EXISTS baseline_traceability_immutable ON baseline_traceability;
CREATE TRIGGER baseline_traceability_immutable
    BEFORE UPDATE OR DELETE ON baseline_traceability
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();

-- Record that this migration was applied
INSERT INTO __diesel_schema_migrations (version) VALUES ('2026-02-08-000001_immutable_baselines')
ON CONFLICT (version) DO NOTHING;
