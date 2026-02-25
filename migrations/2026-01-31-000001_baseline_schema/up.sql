-- =============================================================================
-- ReqMan Baseline Schema (squashed migrations)
-- =============================================================================
-- This migration represents the latest schema as of 2026-01-31.
-- It replaces the historical migration chain for pre-release development.
--
-- Notes:
-- - `scripts/init_complete.sql` remains the recommended dev setup (schema + seed).
-- - This migration focuses on schema objects only (no sample data).
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

-- pgvector extension for vector similarity search
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
    owner_id INTEGER
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL DEFAULT ' ',
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    password_hash VARCHAR(255) NOT NULL DEFAULT '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0',
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

CREATE TABLE requirement_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id)
);

CREATE TABLE test_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL REFERENCES projects(id)
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

CREATE TABLE requirements (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    verification_method_id INTEGER NOT NULL DEFAULT 1,
    status_id INTEGER NOT NULL DEFAULT 1 REFERENCES requirement_status(id),
    author_id INTEGER NOT NULL DEFAULT 0,
    reviewer_id INTEGER NOT NULL DEFAULT 0,
    reference_code VARCHAR NOT NULL DEFAULT ' ',
    category_id INTEGER NOT NULL DEFAULT 1,
    parent_id INTEGER,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    update_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deadline_date TIMESTAMP,
    applicability_id INTEGER NOT NULL DEFAULT 1 REFERENCES applicability(id),
    justification TEXT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    search_vector tsvector
);

CREATE TABLE tests (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL DEFAULT ' ',
    reference_code VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    source VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 0 REFERENCES test_status(id),
    parent_id INTEGER,
    project_id INTEGER NOT NULL REFERENCES projects(id)
);

CREATE TABLE matrix (
    req_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    test_id INTEGER NOT NULL REFERENCES tests(id) ON DELETE CASCADE,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    PRIMARY KEY (req_id, test_id)
);

CREATE TABLE logs (
    log_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id INTEGER,
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    old_values TEXT,
    new_values TEXT,
    description TEXT,
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- SEMANTIC SEARCH TABLES
-- =============================================================================

CREATE TABLE requirement_embeddings (
    requirement_id INTEGER PRIMARY KEY REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    embedding vector(768),
    embedding_model VARCHAR(100) NOT NULL DEFAULT 'nomic-embed-text',
    content_hash VARCHAR(64) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE embedding_index_queue (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMP,
    UNIQUE(requirement_id)
);

-- =============================================================================
-- CONSTRAINTS
-- =============================================================================

ALTER TABLE requirements
    ADD CONSTRAINT requirements_reference_code_unique UNIQUE (reference_code),
    ADD CONSTRAINT requirements_title_not_blank CHECK (btrim(title) <> '');

ALTER TABLE tests
    ADD CONSTRAINT tests_reference_code_unique UNIQUE (reference_code),
    ADD CONSTRAINT tests_name_not_blank CHECK (btrim(name) <> '');

ALTER TABLE projects
    ADD CONSTRAINT projects_name_not_blank CHECK (btrim(name) <> '');

-- =============================================================================
-- FULL-TEXT SEARCH TRIGGER
-- =============================================================================

CREATE OR REPLACE FUNCTION requirements_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.reference_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.justification, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER requirements_search_vector_trigger
    BEFORE INSERT OR UPDATE OF title, description, justification, reference_code
    ON requirements
    FOR EACH ROW
    EXECUTE FUNCTION requirements_search_vector_update();

-- =============================================================================
-- INDEXES
-- =============================================================================

-- Projects
CREATE INDEX idx_projects_status ON projects(status);

-- Users
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_admin ON users(is_admin);

-- Project members
CREATE INDEX project_members_user_idx ON project_members(user_id);

-- Logs
CREATE INDEX idx_logs_user_id ON logs(user_id);
CREATE INDEX idx_logs_entity_type ON logs(entity_type);
CREATE INDEX idx_logs_entity_id ON logs(entity_id);
CREATE INDEX idx_logs_project_id ON logs(project_id);
CREATE INDEX idx_logs_created_at ON logs(created_at);
CREATE INDEX idx_logs_action_type ON logs(action_type);

-- Requirements
CREATE INDEX idx_requirements_project_id ON requirements(project_id);
CREATE INDEX idx_requirements_category ON requirements(category_id);
CREATE INDEX idx_requirements_status ON requirements(status_id);
CREATE INDEX idx_requirements_author ON requirements(author_id);
CREATE INDEX idx_requirements_reviewer ON requirements(reviewer_id);
CREATE INDEX idx_requirements_parent ON requirements(parent_id);
CREATE INDEX idx_requirements_project_status ON requirements(project_id, status_id);
CREATE INDEX idx_requirements_search_vector ON requirements USING gin(search_vector);

-- Tests
CREATE INDEX idx_tests_project_id ON tests(project_id);
CREATE INDEX idx_tests_status ON tests(status_id);
CREATE INDEX idx_tests_parent ON tests(parent_id);
CREATE INDEX idx_tests_project_status ON tests(project_id, status_id);

-- Matrix
CREATE INDEX idx_matrix_project_id ON matrix(project_id);
CREATE INDEX idx_matrix_req_id ON matrix(req_id);
CREATE INDEX idx_matrix_test_id ON matrix(test_id);
CREATE INDEX idx_matrix_project_req ON matrix(project_id, req_id);

-- Categories / applicability
CREATE INDEX idx_categories_project_id ON categories(project_id);
CREATE INDEX idx_categories_tag ON categories(tag);
CREATE INDEX idx_applicability_project_id ON applicability(project_id);
CREATE INDEX idx_applicability_tag ON applicability(tag);

-- Semantic search
CREATE INDEX idx_requirement_embeddings_project_id ON requirement_embeddings(project_id);

CREATE INDEX idx_requirement_embeddings_vector_hnsw
    ON requirement_embeddings
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_embedding_index_queue_status ON embedding_index_queue(status, created_at);
CREATE INDEX idx_embedding_index_queue_project ON embedding_index_queue(project_id);
