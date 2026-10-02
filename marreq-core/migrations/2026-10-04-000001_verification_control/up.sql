-- Verification control data for the Verification Control Document (issue #353).
--
-- 1. verification_status.outcome: what a status means for close-out. Statuses
--    are a per-project catalog, so the title alone is not reliable.
-- 2. verification_control: level, stage and evidence of a verification.
-- 3. requirement_compliance: the reviewer's compliance assessment of a
--    requirement (C / PC / NC). Not versioned: like a verification status it
--    records progress, not the specification.

ALTER TABLE verification_status
    ADD COLUMN outcome VARCHAR(12) NOT NULL DEFAULT 'not_run'
        CHECK (outcome IN ('passed', 'failed', 'in_progress', 'not_run'));

UPDATE verification_status SET outcome = CASE
    WHEN lower(btrim(title)) IN ('passed', 'pass') THEN 'passed'
    WHEN lower(btrim(title)) IN ('failed', 'fail') THEN 'failed'
    WHEN lower(btrim(title)) IN ('in progress', 'in-progress', 'running') THEN 'in_progress'
    ELSE 'not_run'
END;

CREATE TABLE verification_control (
    verification_id INTEGER PRIMARY KEY REFERENCES verifications(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    verification_level VARCHAR(40),
    verification_stage VARCHAR(40),
    evidence_reference TEXT,
    updated_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_verification_control_project ON verification_control (project_id);

CREATE TABLE requirement_compliance (
    requirement_id INTEGER PRIMARY KEY REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    compliance CHAR(2) NOT NULL CHECK (compliance IN ('C', 'PC', 'NC')),
    note TEXT,
    set_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    set_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_requirement_compliance_project ON requirement_compliance (project_id);
