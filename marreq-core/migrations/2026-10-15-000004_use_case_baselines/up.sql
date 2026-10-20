-- Use cases in baselines (issue #402, epic #388). Creating a baseline also
-- records the current version of every published use case of the project and
-- the use case links at that moment. Like the other baseline snapshots these
-- rows are immutable; only a project purge deletes them. A use case that a
-- baseline records cannot be deleted (RESTRICT).

CREATE TABLE baseline_use_cases (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    use_case_id INTEGER NOT NULL REFERENCES use_cases(id) ON DELETE RESTRICT,
    version_id INTEGER NOT NULL REFERENCES use_case_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (baseline_id, use_case_id)
);

CREATE TABLE baseline_use_case_links (
    id SERIAL PRIMARY KEY,
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    use_case_id INTEGER NOT NULL REFERENCES use_cases(id) ON DELETE RESTRICT,
    element_id VARCHAR(40),
    requirement_id INTEGER REFERENCES requirements(id) ON DELETE CASCADE,
    verification_id INTEGER REFERENCES verifications(id) ON DELETE CASCADE,
    suspect BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX idx_baseline_use_case_links_baseline ON baseline_use_case_links (baseline_id);

CREATE TRIGGER baseline_use_cases_immutable
    BEFORE UPDATE OR DELETE ON baseline_use_cases
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();
CREATE TRIGGER baseline_use_case_links_immutable
    BEFORE UPDATE OR DELETE ON baseline_use_case_links
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();
