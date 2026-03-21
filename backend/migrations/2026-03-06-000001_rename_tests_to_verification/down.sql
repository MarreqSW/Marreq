-- Reverse: restore tests, test_status, verification table names and matrix/baseline_traceability columns.

DROP TRIGGER IF EXISTS matrix_project_consistency ON matrix;

-- check_rvvm: verification_methods -> verification (table rename happens at end)
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
    FROM verification WHERE id = NEW.verification_method_id;

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

-- Indexes: drop new names
DROP INDEX IF EXISTS idx_verification_status_project_id;
DROP INDEX IF EXISTS idx_verifications_project_status;
DROP INDEX IF EXISTS idx_verifications_parent;
DROP INDEX IF EXISTS idx_verifications_status;
DROP INDEX IF EXISTS idx_verifications_project_id;
DROP INDEX IF EXISTS idx_matrix_verification_id;

-- verification_status constraint
ALTER TABLE verification_status RENAME CONSTRAINT verification_status_project_id_tag_unique TO test_status_project_id_tag_unique;

-- verifications constraints back to tests_*
ALTER TABLE verifications RENAME CONSTRAINT verifications_status_id_fkey TO tests_status_id_fkey;
ALTER TABLE verifications RENAME CONSTRAINT verifications_project_id_reference_code_unique TO tests_project_id_reference_code_unique;
ALTER TABLE verifications RENAME CONSTRAINT verifications_name_not_blank TO tests_name_not_blank;
ALTER TABLE verifications RENAME CONSTRAINT verifications_parent_id_fk TO tests_parent_id_fk;

-- Table renames: verifications -> tests so matrix/baseline_traceability can reference tests(id)
ALTER TABLE verifications RENAME TO tests;
ALTER TABLE verification_status RENAME TO test_status;
ALTER TABLE verification_methods RENAME TO verification;

-- matrix: verification_id -> test_id, FK to tests(id)
ALTER TABLE matrix DROP CONSTRAINT matrix_verification_id_fkey;
ALTER TABLE matrix DROP CONSTRAINT matrix_pkey;
ALTER TABLE matrix RENAME COLUMN verification_id TO test_id;
ALTER TABLE matrix ADD CONSTRAINT matrix_pkey PRIMARY KEY (req_id, test_id);
ALTER TABLE matrix ADD CONSTRAINT matrix_test_id_fkey
    FOREIGN KEY (test_id) REFERENCES tests(id) ON DELETE CASCADE;

-- baseline_traceability: verification_id -> test_id
ALTER TABLE baseline_traceability DROP CONSTRAINT baseline_traceability_verification_id_fkey;
ALTER TABLE baseline_traceability DROP CONSTRAINT baseline_traceability_pkey;
ALTER TABLE baseline_traceability RENAME COLUMN verification_id TO test_id;
ALTER TABLE baseline_traceability ADD CONSTRAINT baseline_traceability_pkey
    PRIMARY KEY (baseline_id, requirement_id, test_id);
ALTER TABLE baseline_traceability ADD CONSTRAINT baseline_traceability_test_id_fkey
    FOREIGN KEY (test_id) REFERENCES tests(id) ON DELETE CASCADE;

-- Recreate old indexes
CREATE INDEX idx_test_status_project_id ON test_status(project_id);
CREATE INDEX idx_tests_project_id     ON tests(project_id);
CREATE INDEX idx_tests_status         ON tests(status_id);
CREATE INDEX idx_tests_parent         ON tests(parent_id);
CREATE INDEX idx_tests_project_status ON tests(project_id, status_id);
CREATE INDEX idx_matrix_test_id       ON matrix(test_id);

COMMENT ON COLUMN tests.parent_id IS
    'Self-referencing FK → tests(id) ON DELETE SET NULL; NULL when parent test is deleted.';

-- Restore matrix trigger
CREATE OR REPLACE FUNCTION check_matrix_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    req_project_id  INTEGER;
    test_project_id INTEGER;
BEGIN
    SELECT project_id INTO req_project_id
    FROM requirements WHERE id = NEW.req_id;

    SELECT project_id INTO test_project_id
    FROM tests WHERE id = NEW.test_id;

    IF req_project_id IS NULL THEN
        RAISE EXCEPTION '[cross_project] requirement % does not exist', NEW.req_id;
    END IF;
    IF test_project_id IS NULL THEN
        RAISE EXCEPTION '[cross_project] test % does not exist', NEW.test_id;
    END IF;
    IF req_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] requirement % belongs to project % but matrix row declares project_id=%',
            NEW.req_id, req_project_id, NEW.project_id;
    END IF;
    IF test_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] test % belongs to project % but matrix row declares project_id=%',
            NEW.test_id, test_project_id, NEW.project_id;
    END IF;

    RETURN NEW;
END;
$$;
CREATE TRIGGER matrix_project_consistency
    BEFORE INSERT OR UPDATE ON matrix
    FOR EACH ROW EXECUTE FUNCTION check_matrix_project_consistency();
