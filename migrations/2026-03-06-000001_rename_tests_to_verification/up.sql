-- Rename verification (methods) -> verification_methods so "Verification" can denote former tests.
ALTER TABLE verification RENAME TO verification_methods;

-- Rename test_status -> verification_status.
ALTER TABLE test_status RENAME TO verification_status;

-- Rename tests -> verifications.
ALTER TABLE tests RENAME TO verifications;

-- verifications.status_id still references verification_status(id) (same table, just renamed).
-- verifications.parent_id: constraint tests_parent_id_fk references verifications(id). Rename constraint.
ALTER TABLE verifications RENAME CONSTRAINT tests_parent_id_fk TO verifications_parent_id_fk;
ALTER TABLE verifications RENAME CONSTRAINT tests_name_not_blank TO verifications_name_not_blank;
ALTER TABLE verifications RENAME CONSTRAINT tests_project_id_reference_code_unique TO verifications_project_id_reference_code_unique;
-- status_id FK: was tests_status_id_fkey -> verification_status. Rename for clarity.
ALTER TABLE verifications RENAME CONSTRAINT tests_status_id_fkey TO verifications_status_id_fkey;

-- matrix: test_id -> verification_id, FK to verifications(id).
ALTER TABLE matrix DROP CONSTRAINT matrix_test_id_fkey;
ALTER TABLE matrix DROP CONSTRAINT matrix_pkey;
ALTER TABLE matrix RENAME COLUMN test_id TO verification_id;
ALTER TABLE matrix ADD CONSTRAINT matrix_pkey PRIMARY KEY (req_id, verification_id);
ALTER TABLE matrix ADD CONSTRAINT matrix_verification_id_fkey
    FOREIGN KEY (verification_id) REFERENCES verifications(id) ON DELETE CASCADE;

-- baseline_traceability: test_id -> verification_id.
ALTER TABLE baseline_traceability DROP CONSTRAINT baseline_traceability_test_id_fkey;
ALTER TABLE baseline_traceability DROP CONSTRAINT baseline_traceability_pkey;
ALTER TABLE baseline_traceability RENAME COLUMN test_id TO verification_id;
ALTER TABLE baseline_traceability ADD CONSTRAINT baseline_traceability_pkey
    PRIMARY KEY (baseline_id, requirement_id, verification_id);
ALTER TABLE baseline_traceability ADD CONSTRAINT baseline_traceability_verification_id_fkey
    FOREIGN KEY (verification_id) REFERENCES verifications(id) ON DELETE CASCADE;

-- requirement_version_verification_methods: FK pointed to verification(id). Table is now verification_methods;
-- PostgreSQL keeps the FK valid on table rename.

-- Triggers and functions that reference tests/test_id or verification table.
DROP TRIGGER IF EXISTS matrix_project_consistency ON matrix;
CREATE OR REPLACE FUNCTION check_matrix_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    req_project_id  INTEGER;
    ver_project_id  INTEGER;
BEGIN
    SELECT project_id INTO req_project_id
    FROM requirements WHERE id = NEW.req_id;

    SELECT project_id INTO ver_project_id
    FROM verifications WHERE id = NEW.verification_id;

    IF req_project_id IS NULL THEN
        RAISE EXCEPTION '[cross_project] requirement % does not exist', NEW.req_id;
    END IF;
    IF ver_project_id IS NULL THEN
        RAISE EXCEPTION '[cross_project] verification % does not exist', NEW.verification_id;
    END IF;
    IF req_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] requirement % belongs to project % but matrix row declares project_id=%',
            NEW.req_id, req_project_id, NEW.project_id;
    END IF;
    IF ver_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] verification % belongs to project % but matrix row declares project_id=%',
            NEW.verification_id, ver_project_id, NEW.project_id;
    END IF;

    RETURN NEW;
END;
$$;
CREATE TRIGGER matrix_project_consistency
    BEFORE INSERT OR UPDATE ON matrix
    FOR EACH ROW EXECUTE FUNCTION check_matrix_project_consistency();

-- check_rvvm_project_consistency: verification -> verification_methods
CREATE OR REPLACE FUNCTION check_rvvm_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    ver_project_id INTEGER;
    vm_project_id  INTEGER;
BEGIN
    SELECT r.project_id INTO ver_project_id
    FROM requirement_versions rv
    JOIN requirements r ON r.id = rv.requirement_id
    WHERE rv.id = NEW.requirement_version_id;

    SELECT project_id INTO vm_project_id
    FROM verification_methods WHERE id = NEW.verification_method_id;

    IF ver_project_id IS NULL OR vm_project_id IS NULL THEN
        RETURN NEW;
    END IF;

    IF ver_project_id <> vm_project_id THEN
        RAISE EXCEPTION
            '[cross_project] requirement version % (project %) cannot use verification method % from project %',
            NEW.requirement_version_id, ver_project_id,
            NEW.verification_method_id, vm_project_id;
    END IF;

    RETURN NEW;
END;
$$;

-- verification_status constraint name
ALTER TABLE verification_status RENAME CONSTRAINT test_status_project_id_tag_unique TO verification_status_project_id_tag_unique;

-- Indexes: drop old, create new.
DROP INDEX IF EXISTS idx_tests_project_id;
DROP INDEX IF EXISTS idx_tests_status;
DROP INDEX IF EXISTS idx_tests_parent;
DROP INDEX IF EXISTS idx_tests_project_status;
CREATE INDEX idx_verifications_project_id     ON verifications(project_id);
CREATE INDEX idx_verifications_status         ON verifications(status_id);
CREATE INDEX idx_verifications_parent         ON verifications(parent_id);
CREATE INDEX idx_verifications_project_status ON verifications(project_id, status_id);

DROP INDEX IF EXISTS idx_matrix_test_id;
CREATE INDEX idx_matrix_verification_id ON matrix(verification_id);

DROP INDEX IF EXISTS idx_test_status_project_id;
CREATE INDEX idx_verification_status_project_id ON verification_status(project_id);

-- Comment
COMMENT ON COLUMN verifications.parent_id IS
    'Self-referencing FK → verifications(id) ON DELETE SET NULL; NULL when parent verification is deleted.';
