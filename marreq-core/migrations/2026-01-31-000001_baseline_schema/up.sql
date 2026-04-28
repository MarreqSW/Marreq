-- =============================================================================
-- Marreq Origin Schema
-- =============================================================================
-- Single baseline migration representing the complete schema at v0.1 (pre-release).
-- All previous incremental migrations have been squashed into this file.
--
-- Password hashing: Argon2id (no bcrypt).
-- Default demo password for seeded users: ChangeMe123!
-- =============================================================================

-- =============================================================================
-- DIESEL HELPER FUNCTIONS
-- =============================================================================

CREATE OR REPLACE FUNCTION diesel_manage_updated_at(_tbl regclass) RETURNS VOID AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION diesel_set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- EXTENSIONS
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS vector;

-- =============================================================================
-- CORE TABLES
-- =============================================================================

CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    creation_date TIMESTAMP,
    update_date TIMESTAMP,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    owner_id INTEGER,
    CONSTRAINT projects_name_not_blank CHECK (btrim(name) <> '')
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL DEFAULT ' ',
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    password_hash VARCHAR(255) NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE project_members (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role INTEGER NOT NULL DEFAULT 2,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, user_id)
);

-- API tokens for headless auth (e.g. MCP).
-- token_hash: SHA-256 hex of the raw token; raw token is never stored.
CREATE TABLE user_api_tokens (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL,
    name VARCHAR(255),
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP,
    UNIQUE(token_hash)
);

CREATE TABLE requirement_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    is_system BOOLEAN NOT NULL DEFAULT false,
    tag_color VARCHAR(20) NULL
);

CREATE TABLE test_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    is_system BOOLEAN NOT NULL DEFAULT false,
    tag_color VARCHAR(20) NULL
);

CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL REFERENCES projects(id)
);

CREATE TABLE applicability (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL REFERENCES projects(id)
);

CREATE TABLE verification (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id)
);

CREATE TABLE custom_field_definitions (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    label VARCHAR(255) NOT NULL,
    field_type VARCHAR(20) NOT NULL CHECK (field_type IN ('text', 'enum', 'boolean', 'number')),
    enum_values JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT custom_field_definitions_enum_values_for_enum_type
        CHECK ((field_type <> 'enum') OR (enum_values IS NOT NULL AND jsonb_typeof(enum_values) = 'array'))
);

-- Requirements: logical container; current content lives in requirement_versions.
-- current_version_id FK added after requirement_versions is created (below).
CREATE TABLE requirements (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    stable_code VARCHAR NOT NULL DEFAULT ' ',
    current_version_id INTEGER
);

-- Immutable requirement versions: each edit creates a new row.
CREATE TABLE requirement_versions (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 1 REFERENCES requirement_status(id),
    author_id INTEGER NOT NULL DEFAULT 0,
    reviewer_id INTEGER NOT NULL DEFAULT 0,
    category_id INTEGER NOT NULL DEFAULT 1,
    applicability_id INTEGER NOT NULL DEFAULT 1 REFERENCES applicability(id),
    justification TEXT,
    deadline_date TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    search_vector tsvector,
    approval_state VARCHAR(20) NOT NULL DEFAULT 'draft',
    approved_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMP,
    CONSTRAINT requirement_versions_title_not_blank CHECK (btrim(title) <> ''),
    CONSTRAINT requirement_versions_approval_state_check
        CHECK (approval_state IN ('draft', 'reviewed', 'approved'))
);

COMMENT ON COLUMN requirement_versions.approval_state IS 'Workflow: draft | reviewed | approved';
COMMENT ON COLUMN requirement_versions.approved_by    IS 'User who approved this version';
COMMENT ON COLUMN requirement_versions.approved_at    IS 'When this version was approved';

-- Deferred FK: requirements -> requirement_versions (circular dependency)
ALTER TABLE requirements
    ADD CONSTRAINT requirements_current_version_fk
        FOREIGN KEY (current_version_id) REFERENCES requirement_versions(id),
    ADD CONSTRAINT requirements_stable_code_unique UNIQUE (project_id, stable_code);

CREATE TABLE requirement_version_verification_methods (
    requirement_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    verification_method_id INTEGER NOT NULL REFERENCES verification(id),
    PRIMARY KEY (requirement_version_id, verification_method_id)
);

CREATE TABLE custom_field_values (
    requirement_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    custom_field_definition_id INTEGER NOT NULL REFERENCES custom_field_definitions(id) ON DELETE CASCADE,
    value TEXT,
    PRIMARY KEY (requirement_version_id, custom_field_definition_id)
);

-- Immutable comments attached to a requirement or a specific version.
CREATE TABLE requirement_comments (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    requirement_version_id INTEGER REFERENCES requirement_versions(id) ON DELETE SET NULL,
    author_id INTEGER NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Typed version-to-version links (replaces deprecated parent_id column).
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

CREATE TABLE tests (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL DEFAULT ' ',
    reference_code VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    source VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 0 REFERENCES test_status(id),
    parent_id INTEGER,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    CONSTRAINT tests_reference_code_unique UNIQUE (reference_code),
    CONSTRAINT tests_name_not_blank CHECK (btrim(name) <> '')
);

CREATE TABLE matrix (
    req_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    test_id INTEGER NOT NULL REFERENCES tests(id) ON DELETE CASCADE,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    suspect BOOLEAN NOT NULL DEFAULT false,
    suspect_at TIMESTAMP,
    suspect_reason TEXT,
    cleared_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    cleared_at TIMESTAMP,
    triggering_version_id INTEGER REFERENCES requirement_versions(id) ON DELETE SET NULL,
    triggering_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (req_id, test_id)
);

COMMENT ON COLUMN matrix.suspect               IS 'True when the link needs re-review';
COMMENT ON COLUMN matrix.triggering_version_id IS 'Requirement version that caused the suspect flag';
COMMENT ON COLUMN matrix.triggering_user_id    IS 'User who triggered the change';

-- Immutable baseline snapshots.
CREATE TABLE baselines (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
);

CREATE TABLE baseline_requirements (
    baseline_id    INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    version_id     INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE RESTRICT,
    PRIMARY KEY (baseline_id, requirement_id)
);

CREATE TABLE baseline_traceability (
    baseline_id    INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    test_id        INTEGER NOT NULL REFERENCES tests(id) ON DELETE CASCADE,
    suspect        BOOLEAN NOT NULL DEFAULT false,
    suspect_at     TIMESTAMP NULL,
    suspect_reason TEXT NULL,
    PRIMARY KEY (baseline_id, requirement_id, test_id)
);

CREATE TABLE logs (
    log_id      SERIAL PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id   INTEGER,
    project_id  INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    old_values  TEXT,
    new_values  TEXT,
    description TEXT,
    ip_address  VARCHAR(45),
    user_agent  TEXT,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- SEMANTIC SEARCH TABLES
-- =============================================================================

CREATE TABLE requirement_embeddings (
    requirement_id  INTEGER PRIMARY KEY REFERENCES requirements(id) ON DELETE CASCADE,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    embedding       vector(768),
    embedding_model VARCHAR(100) NOT NULL DEFAULT 'nomic-embed-text',
    content_hash    VARCHAR(64) NOT NULL,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE embedding_index_queue (
    id             SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    project_id     INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status         VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message  TEXT,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at   TIMESTAMP,
    UNIQUE(requirement_id)
);

-- =============================================================================
-- IMMUTABILITY TRIGGERS
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

-- =============================================================================
-- FULL-TEXT SEARCH TRIGGER
-- =============================================================================

CREATE OR REPLACE FUNCTION requirement_versions_search_vector_update() RETURNS trigger AS $$
DECLARE
    stable_code_val VARCHAR;
BEGIN
    SELECT COALESCE(r.stable_code, '') INTO stable_code_val
    FROM requirements r WHERE r.id = NEW.requirement_id;
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(stable_code_val, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.justification, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER requirement_versions_search_vector_trigger
    BEFORE INSERT OR UPDATE OF title, description, justification, requirement_id
    ON requirement_versions
    FOR EACH ROW
    EXECUTE FUNCTION requirement_versions_search_vector_update();

-- =============================================================================
-- INDEXES
-- =============================================================================

-- Projects
CREATE INDEX idx_projects_status ON projects(status);

-- Users
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_admin    ON users(is_admin);

-- Project members
CREATE INDEX project_members_user_idx ON project_members(user_id);

-- API tokens
CREATE INDEX idx_user_api_tokens_token_hash ON user_api_tokens(token_hash);
CREATE INDEX idx_user_api_tokens_user_id    ON user_api_tokens(user_id);

-- Logs
CREATE INDEX idx_logs_user_id     ON logs(user_id);
CREATE INDEX idx_logs_entity_type ON logs(entity_type);
CREATE INDEX idx_logs_entity_id   ON logs(entity_id);
CREATE INDEX idx_logs_project_id  ON logs(project_id);
CREATE INDEX idx_logs_created_at  ON logs(created_at);
CREATE INDEX idx_logs_action_type ON logs(action_type);

-- Requirements (container)
CREATE INDEX idx_requirements_project_id      ON requirements(project_id);
CREATE INDEX idx_requirements_current_version ON requirements(current_version_id);
CREATE INDEX idx_requirements_project_stable  ON requirements(project_id, stable_code);

-- Requirement versions
CREATE INDEX idx_requirement_versions_requirement_id      ON requirement_versions(requirement_id);
CREATE INDEX idx_requirement_versions_requirement_created ON requirement_versions(requirement_id, created_at DESC);
CREATE INDEX idx_requirement_versions_created_at          ON requirement_versions(created_at DESC);
CREATE INDEX idx_requirement_versions_search_vector       ON requirement_versions USING gin(search_vector);
CREATE INDEX idx_requirement_versions_approval_state      ON requirement_versions(approval_state);
CREATE INDEX idx_requirement_version_verification_version ON requirement_version_verification_methods(requirement_version_id);

-- Requirement comments
CREATE INDEX idx_requirement_comments_requirement ON requirement_comments(requirement_id);
CREATE INDEX idx_requirement_comments_version     ON requirement_comments(requirement_version_id);

-- Requirement version links
CREATE UNIQUE INDEX idx_rvl_source_target_type ON requirement_version_links(source_version_id, target_version_id, link_type);
CREATE INDEX        idx_rvl_source             ON requirement_version_links(source_version_id);
CREATE INDEX        idx_rvl_target             ON requirement_version_links(target_version_id);
CREATE INDEX        idx_rvl_project            ON requirement_version_links(project_id);

-- Tests
CREATE INDEX idx_tests_project_id     ON tests(project_id);
CREATE INDEX idx_tests_status         ON tests(status_id);
CREATE INDEX idx_tests_parent         ON tests(parent_id);
CREATE INDEX idx_tests_project_status ON tests(project_id, status_id);

-- Matrix
CREATE INDEX idx_matrix_project_id  ON matrix(project_id);
CREATE INDEX idx_matrix_req_id      ON matrix(req_id);
CREATE INDEX idx_matrix_test_id     ON matrix(test_id);
CREATE INDEX idx_matrix_suspect     ON matrix(suspect) WHERE suspect = true;
CREATE INDEX idx_matrix_project_req ON matrix(project_id, req_id);

-- Baselines
CREATE INDEX idx_baselines_project_id           ON baselines(project_id);
CREATE INDEX idx_baselines_created_at           ON baselines(created_at DESC);
CREATE INDEX idx_baseline_requirements_baseline ON baseline_requirements(baseline_id);
CREATE INDEX idx_baseline_requirements_version  ON baseline_requirements(version_id);
CREATE INDEX idx_baseline_traceability_baseline ON baseline_traceability(baseline_id);

-- Custom fields
CREATE INDEX idx_custom_field_definitions_project ON custom_field_definitions(project_id);
CREATE INDEX idx_custom_field_values_version      ON custom_field_values(requirement_version_id);
CREATE INDEX idx_custom_field_values_definition   ON custom_field_values(custom_field_definition_id);

-- Categories / applicability
CREATE INDEX idx_categories_project_id    ON categories(project_id);
CREATE INDEX idx_categories_tag           ON categories(tag);
CREATE INDEX idx_applicability_project_id ON applicability(project_id);
CREATE INDEX idx_applicability_tag        ON applicability(tag);

-- Semantic search
CREATE INDEX idx_requirement_embeddings_project_id ON requirement_embeddings(project_id);

CREATE INDEX idx_requirement_embeddings_vector_hnsw
    ON requirement_embeddings
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_embedding_index_queue_status  ON embedding_index_queue(status, created_at);
CREATE INDEX idx_embedding_index_queue_project ON embedding_index_queue(project_id);

-- =============================================================================
-- DEFAULT ADMIN USER  (password: ChangeMe123! — Argon2id)
-- =============================================================================

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'alice', 'Alice Johnson', 'alice@marreq.com', true,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users LIMIT 1);

-- =============================================================================
-- SQUASHED: UNIQUE USER IDENTITY (CASE-INSENSITIVE)
-- =============================================================================

DO $$
DECLARE
    dup_user_count  INTEGER;
    dup_email_count INTEGER;
    dup_report      TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_user_count
    FROM (
        SELECT lower(username)
        FROM users
        GROUP BY lower(username)
        HAVING COUNT(*) > 1
    ) t;

    SELECT COUNT(*) INTO dup_email_count
    FROM (
        SELECT lower(email)
        FROM users
        GROUP BY lower(email)
        HAVING COUNT(*) > 1
    ) t;

    IF dup_user_count > 0 OR dup_email_count > 0 THEN
        SELECT string_agg(line, E'\n') INTO dup_report
        FROM (
            SELECT '  [username] ' || lower(username) ||
                   ' — ' || COUNT(*) || ' rows (ids: ' ||
                   string_agg(id::text, ', ' ORDER BY id) || ')' AS line
            FROM users
            GROUP BY lower(username)
            HAVING COUNT(*) > 1

            UNION ALL

            SELECT '  [email]    ' || lower(email) ||
                   ' — ' || COUNT(*) || ' rows (ids: ' ||
                   string_agg(id::text, ', ' ORDER BY id) || ')' AS line
            FROM users
            GROUP BY lower(email)
            HAVING COUNT(*) > 1
        ) problems;

        RAISE EXCEPTION
            E'Cannot enforce unique user identity: % duplicate group(s) detected.\n'
            'Resolve the following conflicts before re-running this migration:\n%',
            dup_user_count + dup_email_count,
            dup_report;
    END IF;
END;
$$;

DROP INDEX IF EXISTS idx_users_username;
CREATE UNIQUE INDEX idx_users_username_lower ON users (lower(username));
CREATE UNIQUE INDEX idx_users_email_lower    ON users (lower(email));

-- =============================================================================
-- SQUASHED: DB-LEVEL CROSS-PROJECT INTEGRITY TRIGGERS
-- =============================================================================

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

CREATE TRIGGER rvvm_project_consistency
    BEFORE INSERT OR UPDATE ON requirement_version_verification_methods
    FOR EACH ROW EXECUTE FUNCTION check_rvvm_project_consistency();

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

-- =============================================================================
-- SQUASHED: MISSING FOREIGN KEYS + CATEGORY CROSS-PROJECT CHECK
-- =============================================================================

UPDATE projects
SET    owner_id = NULL
WHERE  owner_id IS NOT NULL
  AND  owner_id NOT IN (SELECT id FROM users);

ALTER TABLE projects
    ADD CONSTRAINT projects_owner_id_fk
        FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL;

COMMENT ON COLUMN projects.owner_id IS
    'FK → users(id) ON DELETE SET NULL; NULL means the owning account was removed.';

ALTER TABLE requirement_versions
    ALTER COLUMN author_id DROP DEFAULT;

UPDATE requirement_versions
SET    author_id = (SELECT id FROM users ORDER BY id LIMIT 1)
WHERE  author_id NOT IN (SELECT id FROM users);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_author_id_fk
        FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.author_id IS
    'FK → users(id) ON DELETE RESTRICT; the authoring user must exist before deletion.';

ALTER TABLE requirement_versions
    ALTER COLUMN reviewer_id DROP DEFAULT;

UPDATE requirement_versions
SET    reviewer_id = (SELECT id FROM users ORDER BY id LIMIT 1)
WHERE  reviewer_id NOT IN (SELECT id FROM users);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_reviewer_id_fk
        FOREIGN KEY (reviewer_id) REFERENCES users(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.reviewer_id IS
    'FK → users(id) ON DELETE RESTRICT; the reviewer must exist before deletion.';

ALTER TABLE requirement_versions
    ALTER COLUMN category_id DROP DEFAULT;

UPDATE requirement_versions rv
SET    category_id = (
           SELECT c.id
           FROM   categories c
           JOIN   requirements req ON req.id = rv.requirement_id
           WHERE  c.project_id = req.project_id
           ORDER  BY c.id
           LIMIT  1
       )
WHERE  NOT EXISTS (
           SELECT 1
           FROM   categories c
           JOIN   requirements req ON req.id = rv.requirement_id
           WHERE  c.id = rv.category_id
             AND  c.project_id = req.project_id
       );

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_category_id_fk
        FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.category_id IS
    'FK → categories(id) ON DELETE RESTRICT; must be a category belonging to the same project.';

CREATE OR REPLACE FUNCTION check_rv_category_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    cat_project_id INTEGER;
    req_project_id INTEGER;
BEGIN
    SELECT project_id INTO cat_project_id
    FROM   categories WHERE id = NEW.category_id;

    SELECT r.project_id INTO req_project_id
    FROM   requirements r WHERE r.id = NEW.requirement_id;

    IF cat_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] category % does not exist', NEW.category_id;
    END IF;
    IF req_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] requirement % does not exist', NEW.requirement_id;
    END IF;
    IF cat_project_id <> req_project_id THEN
        RAISE EXCEPTION
            '[cross_project] category % belongs to project % but requirement version belongs to project %',
            NEW.category_id, cat_project_id, req_project_id;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER rv_category_project_consistency
    BEFORE INSERT OR UPDATE ON requirement_versions
    FOR EACH ROW EXECUTE FUNCTION check_rv_category_project_consistency();

UPDATE tests
SET    parent_id = NULL
WHERE  parent_id IS NOT NULL
  AND  parent_id NOT IN (SELECT id FROM tests);

ALTER TABLE tests
    ADD CONSTRAINT tests_parent_id_fk
        FOREIGN KEY (parent_id) REFERENCES tests(id) ON DELETE SET NULL;

COMMENT ON COLUMN tests.parent_id IS
    'Self-referencing FK → tests(id) ON DELETE SET NULL; NULL when parent test is deleted.';

-- =============================================================================
-- SQUASHED: PROJECT-SCOPED UNIQUENESS CONSTRAINTS
-- =============================================================================

ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_reference_code_unique;

ALTER TABLE tests
    ADD CONSTRAINT tests_project_id_reference_code_unique UNIQUE (project_id, reference_code);

ALTER TABLE requirement_status
    ADD CONSTRAINT requirement_status_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE test_status
    ADD CONSTRAINT test_status_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE categories
    ADD CONSTRAINT categories_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE applicability
    ADD CONSTRAINT applicability_project_id_tag_unique UNIQUE (project_id, tag);

ALTER TABLE verification
    ADD CONSTRAINT verification_project_id_tag_unique UNIQUE (project_id, tag);

-- =============================================================================
-- SQUASHED: PERFORMANCE INDEXES
-- =============================================================================

CREATE INDEX idx_requirement_status_project_id ON requirement_status(project_id);
CREATE INDEX idx_test_status_project_id ON test_status(project_id);
CREATE INDEX idx_requirement_versions_status_id ON requirement_versions(status_id);
CREATE INDEX idx_requirement_versions_category_id ON requirement_versions(category_id);
CREATE INDEX idx_requirement_versions_applicability_id ON requirement_versions(applicability_id);
CREATE INDEX idx_rvvm_verification_method_id ON requirement_version_verification_methods(verification_method_id, requirement_version_id);
CREATE INDEX idx_custom_field_values_definition_value ON custom_field_values(custom_field_definition_id, value);
