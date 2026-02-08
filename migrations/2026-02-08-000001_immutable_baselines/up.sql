-- =============================================================================
-- Immutable project baselines
-- =============================================================================
-- Baselines reference requirement_versions (snapshot at creation time).
-- baseline_requirements and baseline_traceability are append-only; no UPDATE/DELETE.
-- =============================================================================

-- Baselines: one row per snapshot (name, description, created_at, created_by).
CREATE TABLE baselines (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_baselines_project_id ON baselines(project_id);
CREATE INDEX idx_baselines_created_at ON baselines(created_at DESC);

-- Snapshot of requirement versions in the baseline (requirement_id -> version_id at baseline time).
CREATE TABLE baseline_requirements (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (baseline_id, requirement_id)
);

CREATE INDEX idx_baseline_requirements_baseline_id ON baseline_requirements(baseline_id);
CREATE INDEX idx_baseline_requirements_version_id ON baseline_requirements(version_id);

-- Snapshot of traceability matrix at baseline time.
CREATE TABLE baseline_traceability (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    test_id INTEGER NOT NULL REFERENCES tests(id) ON DELETE CASCADE,
    PRIMARY KEY (baseline_id, requirement_id, test_id)
);

CREATE INDEX idx_baseline_traceability_baseline_id ON baseline_traceability(baseline_id);

-- =============================================================================
-- Immutability: forbid UPDATE and DELETE on baselines and child tables
-- =============================================================================

CREATE OR REPLACE FUNCTION forbid_baseline_update_delete() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'Baselines are immutable: UPDATE and DELETE are not allowed';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER baselines_immutable
    BEFORE UPDATE OR DELETE ON baselines
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();

CREATE TRIGGER baseline_requirements_immutable
    BEFORE UPDATE OR DELETE ON baseline_requirements
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();

CREATE TRIGGER baseline_traceability_immutable
    BEFORE UPDATE OR DELETE ON baseline_traceability
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();
