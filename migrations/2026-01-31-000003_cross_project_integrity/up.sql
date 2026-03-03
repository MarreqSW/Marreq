-- =============================================================================
-- Migration: DB-level cross-project integrity triggers
-- =============================================================================
-- Without these triggers the DB silently permits rows such as:
--   • matrix (req from project A, test from project B)
--   • requirement_version_links (source version from A, target from B)
--   • requirement_version_verification_methods (version from A, method from B)
--   • custom_field_values (version from A, field definition from B)
--
-- Each trigger function raises an exception whose message begins with the
-- token "[cross_project] " so that application code can identify and surface
-- the error distinctly from other database failures.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 1. matrix : requirements.project_id = matrix.project_id = tests.project_id
-- ---------------------------------------------------------------------------
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

-- ---------------------------------------------------------------------------
-- 2. requirement_version_links : source, target, and link must share project
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION check_rvl_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    src_project_id INTEGER;
    tgt_project_id INTEGER;
BEGIN
    SELECT r.project_id INTO src_project_id
    FROM requirement_versions rv
    JOIN requirements r ON r.id = rv.requirement_id
    WHERE rv.id = NEW.source_version_id;

    SELECT r.project_id INTO tgt_project_id
    FROM requirement_versions rv
    JOIN requirements r ON r.id = rv.requirement_id
    WHERE rv.id = NEW.target_version_id;

    IF src_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] source requirement version % does not exist', NEW.source_version_id;
    END IF;
    IF tgt_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] target requirement version % does not exist', NEW.target_version_id;
    END IF;
    IF src_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] source version % belongs to project % but link declares project_id=%',
            NEW.source_version_id, src_project_id, NEW.project_id;
    END IF;
    IF tgt_project_id <> NEW.project_id THEN
        RAISE EXCEPTION
            '[cross_project] target version % belongs to project % but link declares project_id=%',
            NEW.target_version_id, tgt_project_id, NEW.project_id;
    END IF;
    IF src_project_id <> tgt_project_id THEN
        RAISE EXCEPTION
            '[cross_project] source version % (project %) and target version % (project %) are in different projects',
            NEW.source_version_id, src_project_id, NEW.target_version_id, tgt_project_id;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER rvl_project_consistency
    BEFORE INSERT OR UPDATE ON requirement_version_links
    FOR EACH ROW EXECUTE FUNCTION check_rvl_project_consistency();

-- ---------------------------------------------------------------------------
-- 3. requirement_version_verification_methods : version and method share project
-- ---------------------------------------------------------------------------
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

    -- NULL means the referenced FK row does not exist; let FK constraints handle that
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

CREATE TRIGGER rvvm_project_consistency
    BEFORE INSERT OR UPDATE ON requirement_version_verification_methods
    FOR EACH ROW EXECUTE FUNCTION check_rvvm_project_consistency();

-- ---------------------------------------------------------------------------
-- 4. custom_field_values : version and field definition share project
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION check_cfv_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    ver_project_id INTEGER;
    def_project_id INTEGER;
BEGIN
    SELECT r.project_id INTO ver_project_id
    FROM requirement_versions rv
    JOIN requirements r ON r.id = rv.requirement_id
    WHERE rv.id = NEW.requirement_version_id;

    SELECT project_id INTO def_project_id
    FROM custom_field_definitions WHERE id = NEW.custom_field_definition_id;

    IF ver_project_id IS NULL OR def_project_id IS NULL THEN
        RETURN NEW;
    END IF;

    IF ver_project_id <> def_project_id THEN
        RAISE EXCEPTION
            '[cross_project] requirement version % (project %) cannot use custom field definition % from project %',
            NEW.requirement_version_id, ver_project_id,
            NEW.custom_field_definition_id, def_project_id;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER cfv_project_consistency
    BEFORE INSERT OR UPDATE ON custom_field_values
    FOR EACH ROW EXECUTE FUNCTION check_cfv_project_consistency();
