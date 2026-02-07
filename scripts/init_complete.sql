-- =============================================================================
-- ReqMan Complete Database Initialization Script
-- =============================================================================
-- This script creates a complete ReqMan database with all tables, constraints,
-- indexes, and sample data including users with working passwords.
-- 
-- Usage:
-- 1. Create database: CREATE DATABASE reqman;
-- 2. Connect to reqman database
-- 3. Run this script: \i init_complete.sql
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

-- Projects table
CREATE TABLE projects (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    creation_date TIMESTAMP,
    update_date TIMESTAMP,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    owner_id INTEGER
);

-- Users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL DEFAULT ' ',
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    password_hash VARCHAR(255) NOT NULL DEFAULT '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m',
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

-- Requirement Status table
CREATE TABLE requirement_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL
);

-- Test Status table
CREATE TABLE test_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL
);

-- Categories table
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Applicability table
CREATE TABLE applicability (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Verification table
CREATE TABLE verification (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    project_id INTEGER NOT NULL
);

-- Requirements table (logical container; current content in requirement_versions)
CREATE TABLE requirements (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL,
    stable_code VARCHAR NOT NULL DEFAULT ' ',
    current_version_id INTEGER
);

-- Immutable version rows per requirement (issue #94)
CREATE TABLE requirement_versions (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 1,
    author_id INTEGER NOT NULL DEFAULT 0,
    reviewer_id INTEGER NOT NULL DEFAULT 0,
    category_id INTEGER NOT NULL DEFAULT 1,
    parent_id INTEGER,
    applicability_id INTEGER NOT NULL DEFAULT 1,
    justification TEXT,
    deadline_date TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_title_not_blank CHECK (btrim(title) <> '');

-- Verification methods per version (M:N)
CREATE TABLE requirement_version_verification_methods (
    requirement_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    verification_method_id INTEGER NOT NULL REFERENCES verification(id),
    PRIMARY KEY (requirement_version_id, verification_method_id)
);

-- Tests table
CREATE TABLE tests (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL DEFAULT ' ',
    reference_code VARCHAR NOT NULL DEFAULT ' ',
    description VARCHAR NOT NULL DEFAULT ' ',
    source VARCHAR NOT NULL DEFAULT ' ',
    status_id INTEGER NOT NULL DEFAULT 0,
    parent_id INTEGER,
    project_id INTEGER NOT NULL
);

-- Matrix table (traceability between requirements and tests)
CREATE TABLE matrix (
    req_id INTEGER NOT NULL,
    test_id INTEGER NOT NULL,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    project_id INTEGER NOT NULL,
    PRIMARY KEY (req_id, test_id)
);

-- Logs table (audit trail)
CREATE TABLE logs (
    log_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id INTEGER,
    project_id INTEGER,
    old_values TEXT,
    new_values TEXT,
    description TEXT,
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- FOREIGN KEY CONSTRAINTS
-- =============================================================================

ALTER TABLE categories ADD CONSTRAINT fk_categories_project
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE applicability ADD CONSTRAINT fk_applicability_project 
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE verification ADD CONSTRAINT fk_verification_project 
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE requirement_status ADD CONSTRAINT fk_requirement_status_project
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE test_status ADD CONSTRAINT fk_test_status_project
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE requirements ADD CONSTRAINT fk_requirements_project
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE requirement_versions ADD CONSTRAINT fk_requirement_versions_requirement
    FOREIGN KEY (requirement_id) REFERENCES requirements(id) ON DELETE CASCADE;
ALTER TABLE requirement_versions ADD CONSTRAINT fk_requirement_versions_status
    FOREIGN KEY (status_id) REFERENCES requirement_status(id);
ALTER TABLE requirement_versions ADD CONSTRAINT fk_requirement_versions_applicability
    FOREIGN KEY (applicability_id) REFERENCES applicability(id);

ALTER TABLE tests ADD CONSTRAINT fk_tests_project 
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE tests ADD CONSTRAINT fk_tests_status 
    FOREIGN KEY (status_id) REFERENCES test_status(id);

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_project 
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_requirements 
    FOREIGN KEY (req_id) REFERENCES requirements(id) ON DELETE CASCADE;

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_tests 
    FOREIGN KEY (test_id) REFERENCES tests(id) ON DELETE CASCADE;

ALTER TABLE logs ADD CONSTRAINT fk_logs_user_id 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE logs ADD CONSTRAINT fk_logs_project_id 
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

CREATE INDEX project_members_user_idx ON project_members(user_id);

-- =============================================================================
-- INDEXES FOR PERFORMANCE
-- =============================================================================

-- Logs indexes
CREATE INDEX idx_logs_user_id ON logs(user_id);
CREATE INDEX idx_logs_entity_type ON logs(entity_type);
CREATE INDEX idx_logs_entity_id ON logs(entity_id);
CREATE INDEX idx_logs_project_id ON logs(project_id);
CREATE INDEX idx_logs_created_at ON logs(created_at);
CREATE INDEX idx_logs_action_type ON logs(action_type);

-- Requirements (container) indexes
CREATE INDEX idx_requirements_project_id ON requirements(project_id);
CREATE INDEX idx_requirements_current_version_id ON requirements(current_version_id);
CREATE INDEX idx_requirements_project_stable ON requirements(project_id, stable_code);

-- Requirement versions indexes
CREATE INDEX idx_requirement_versions_requirement_id ON requirement_versions(requirement_id);
CREATE INDEX idx_requirement_versions_requirement_created ON requirement_versions(requirement_id, created_at DESC);
CREATE INDEX idx_requirement_versions_created_at ON requirement_versions(created_at DESC);
CREATE INDEX idx_requirement_version_verification_version ON requirement_version_verification_methods(requirement_version_id);

-- Tests indexes
CREATE INDEX idx_tests_project_id ON tests(project_id);
CREATE INDEX idx_tests_status ON tests(status_id);
CREATE INDEX idx_tests_parent ON tests(parent_id);

-- Matrix indexes
CREATE INDEX idx_matrix_project_id ON matrix(project_id);
CREATE INDEX idx_matrix_req_id ON matrix(req_id);
CREATE INDEX idx_matrix_test_id ON matrix(test_id);

-- Users indexes
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_admin ON users(is_admin);

-- Categories indexes
CREATE INDEX idx_categories_project_id ON categories(project_id);
CREATE INDEX idx_categories_tag ON categories(tag);

-- Applicability indexes
CREATE INDEX idx_applicability_project_id ON applicability(project_id);
CREATE INDEX idx_applicability_tag ON applicability(tag);

-- =============================================================================
-- CONSTRAINTS
-- =============================================================================

ALTER TABLE requirements
    ADD CONSTRAINT requirements_current_version_fk
        FOREIGN KEY (current_version_id) REFERENCES requirement_versions(id),
    ADD CONSTRAINT requirements_stable_code_unique UNIQUE (project_id, stable_code);

ALTER TABLE tests
    ADD CONSTRAINT tests_reference_code_unique UNIQUE (reference_code),
    ADD CONSTRAINT tests_name_not_blank CHECK (btrim(name) <> '');

ALTER TABLE projects
    ADD CONSTRAINT projects_name_not_blank CHECK (btrim(name) <> '');

-- Composite indexes (from performance tuning migrations)
CREATE INDEX IF NOT EXISTS idx_tests_project_status
    ON tests (project_id, status_id);

CREATE INDEX IF NOT EXISTS idx_matrix_project_req
    ON matrix (project_id, req_id);

-- =============================================================================
-- SEMANTIC SEARCH
-- =============================================================================

-- Table for storing requirement embeddings (vector similarity search)
CREATE TABLE requirement_embeddings (
    requirement_id INTEGER PRIMARY KEY REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    embedding vector(768),
    embedding_model VARCHAR(100) NOT NULL DEFAULT 'nomic-embed-text',
    content_hash VARCHAR(64) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_requirement_embeddings_project_id
    ON requirement_embeddings(project_id);

CREATE INDEX idx_requirement_embeddings_vector_hnsw
    ON requirement_embeddings
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Table for tracking embedding indexing jobs (for async processing)
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

CREATE INDEX idx_embedding_index_queue_status
    ON embedding_index_queue(status, created_at);

CREATE INDEX idx_embedding_index_queue_project
    ON embedding_index_queue(project_id);

-- =============================================================================
-- INITIAL DATA
-- =============================================================================

-- Projects
INSERT INTO projects (id, name, description, creation_date, status) VALUES
    (1, 'Space Project', 'Space exploration satellite requirements and test management system for advanced satellite missions', NOW(), 'active'),
    (2, 'ReqMan Project', 'Requirements management system development and testing', NOW(), 'active'),
    (3, 'Empty Project', 'Empty project for testing and demonstration purposes', NOW(), 'active');

-- Ensure the projects sequence is aligned with seeded IDs
SELECT setval('projects_id_seq', (SELECT COALESCE(MAX(id), 1) FROM projects));

-- Requirement Status definitions
INSERT INTO requirement_status (title, description, tag, project_id) VALUES
    -- Space Project statuses
    ('Draft', 'The requirement is still being edited and developed', 'Drf', 1),
    ('Proposal', 'The requirement is proposed and awaiting approval', 'Pro', 1),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc', 1),
    ('Rejected', 'The requirement is not accepted and needs revision', 'Rej', 1),
    ('Cancelled', 'The requirement is cancelled and will not be implemented', 'Can', 1),
    ('Finished', 'The requirement is finished and completed', 'Fsh', 1),
    -- ReqMan Project statuses
    ('Draft', 'The requirement is still being edited and developed', 'Drf', 2),
    ('Proposal', 'The requirement is proposed and awaiting approval', 'Pro', 2),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc', 2),
    ('Rejected', 'The requirement is not accepted and needs revision', 'Rej', 2),
    ('Cancelled', 'The requirement is cancelled and will not be implemented', 'Can', 2),
    ('Finished', 'The requirement is finished and completed', 'Fsh', 2);


-- Test Status definitions
INSERT INTO test_status (title, description, tag, project_id) VALUES
    -- Space Project test statuses
    ('Passed', 'The test has passed all criteria', 'Pass', 1),
    ('Failed', 'The test has failed one or more criteria', 'Fail', 1),
    ('Pending', 'The test is pending execution', 'Pend', 1),
    ('In Progress', 'The test is currently being executed', 'Prog', 1),
    -- ReqMan Project test statuses
    ('Passed', 'The test has passed all criteria', 'Pass', 2),
    ('Failed', 'The test has failed one or more criteria', 'Fail', 2),
    ('Pending', 'The test is pending execution', 'Pend', 2),
    ('In Progress', 'The test is currently being executed', 'Prog', 2);

-- Users with working passwords (all users have password: 'password')
-- Password hash: $2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m
INSERT INTO users (username, name, email, is_admin, password_hash) VALUES
    ('alice', 'Alice Johnson', 'alice@reqman.com', true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('dr_smith', 'Dr. Sarah Smith', 'sarah.smith@spacecorp.com', true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('eng_jones', 'Engineer Mike Jones', 'mike.jones@spacecorp.com', false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('tech_lee', 'Technician Lisa Lee', 'lisa.lee@spacecorp.com', false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('qa_wilson', 'QA Specialist Tom Wilson', 'tom.wilson@spacecorp.com', false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('admin', 'System Administrator', 'admin@reqman.com', true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m');

-- Project membership assignments (role: 1=Owner, 2=Manager, 3=Contributor, 4=Viewer)
INSERT INTO project_members (project_id, user_id, role) VALUES
    -- Space Project team
    (1, 2, 1),  -- Dr. Smith owns the Space Project
    (1, 3, 3),  -- Engineer Jones contributes to Space Project
    (1, 4, 3),  -- Technician Lee contributes to Space Project
    (1, 5, 4),  -- QA Wilson views the Space Project
    -- ReqMan Product team
    (2, 1, 1),  -- Alice owns the ReqMan Project
    (2, 6, 2),  -- Admin manages the ReqMan Project
    (2, 5, 3),  -- QA Wilson contributes to ReqMan Project
    -- Empty project defaults
    (3, 6, 1),  -- Admin owns the Empty Project
    (3, 1, 2);  -- Alice manages the Empty Project

-- Categories for Space Project
INSERT INTO categories (title, description, tag, project_id) VALUES
    ('Power System', 'Solar panels, batteries, and power distribution systems', 'PWR', 1),
    ('Communication', 'Antennas, transponders, and data communication links', 'COMM', 1),
    ('Attitude Control', 'Gyroscopes, reaction wheels, and star trackers for orientation', 'ACS', 1),
    ('Thermal Control', 'Heat pipes, radiators, and thermal blankets for temperature management', 'THERM', 1),
    ('Payload', 'Scientific instruments and mission-specific equipment', 'PAY', 1),
    ('Propulsion', 'Thrusters and fuel systems for orbital maneuvers', 'PROP', 1),
    ('Structure', 'Mechanical structure and deployment mechanisms', 'STRUCT', 1),
    ('Software', 'On-board computer systems and control algorithms', 'SW', 1);

-- Categories for ReqMan Project
INSERT INTO categories (title, description, tag, project_id) VALUES
    ('User Interface', 'User interface components and functionality', 'UI', 2),
    ('Backend', 'Server-side logic and API endpoints', 'BE', 2),
    ('Database', 'Database schema and data management', 'DB', 2),
    ('Authentication', 'User authentication and authorization', 'AUTH', 2),
    ('Documentation', 'Technical and user documentation', 'DOC', 2),
    ('Testing', 'Test infrastructure and test cases', 'TEST', 2),
    ('Performance', 'System performance and optimization', 'PERF', 2);

-- Applicability definitions for Space Project
INSERT INTO applicability (title, description, tag, project_id) VALUES
    ('All Missions', 'Applies to all satellite missions regardless of type', 'ALL', 1),
    ('Earth Observation', 'Low Earth orbit observation and imaging satellites', 'EO', 1),
    ('Communication', 'Geostationary and medium Earth orbit communication satellites', 'COMM', 1),
    ('Navigation', 'GPS, GLONASS, and other navigation satellite systems', 'NAV', 1),
    ('Deep Space', 'Interplanetary and deep space exploration missions', 'DEEP', 1),
    ('CubeSat', 'Small satellite missions and CubeSat platforms', 'CUBE', 1);

-- Applicability definitions for ReqMan Project
INSERT INTO applicability (title, description, tag, project_id) VALUES
    ('All Users', 'Applies to all user types', 'ALL', 2),
    ('Administrators', 'Applies to system administrators only', 'ADMIN', 2),
    ('Project Managers', 'Applies to project managers and owners', 'MGR', 2),
    ('Contributors', 'Applies to regular contributors', 'CONT', 2),
    ('Viewers', 'Applies to read-only viewers', 'VIEW', 2);

-- Verification methods for Space Project
INSERT INTO verification (title, description, tag, project_id) VALUES
    ('Inspection', 'Nondestructive examination of a system or component', 'INSP', 1),
    ('Analysis', 'Verification using mathematical models and calculations', 'ANALYSIS', 1),
    ('Demonstration', 'Manipulation of the product as intended in its operational environment', 'DEMO', 1),
    ('Test', 'Controlled verification with predefined inputs and expected outputs', 'TEST', 1);

-- Verification methods for ReqMan Project
INSERT INTO verification (title, description, tag, project_id) VALUES
    ('Code Review', 'Review of source code by peers', 'REVIEW', 2),
    ('Unit Test', 'Automated unit testing', 'UNIT', 2),
    ('Integration Test', 'Testing of integrated components', 'INTEG', 2),
    ('System Test', 'End-to-end system testing', 'SYS', 2),
    ('Manual Test', 'Manual testing by QA team', 'MANUAL', 2);


-- Requirements for Space Project (containers only; content in requirement_versions)
INSERT INTO requirements (id, project_id, stable_code) VALUES
    (1, 1, 'REQ-PWR-001'),
    (2, 1, 'REQ-PWR-002'),
    (3, 1, 'REQ-COMM-001'),
    (4, 1, 'REQ-ACS-001'),
    (5, 1, 'REQ-THERM-001');

SELECT setval('requirements_id_seq', (SELECT COALESCE(MAX(id), 1) FROM requirements));

-- Initial versions (v1) for each requirement
INSERT INTO requirement_versions (requirement_id, title, description, category_id, applicability_id, status_id, author_id, reviewer_id, parent_id, created_at, deadline_date) VALUES
    (1, 'REQ-PWR-001', 'The satellite shall generate minimum 500W of electrical power during daylight operations under AM0 illumination conditions', 1, 1, 1, 1, 2, NULL, '2024-01-15', '2024-06-30'),
    (2, 'REQ-PWR-002', 'The battery system shall provide 200W continuous power for 45 minutes during eclipse periods', 1, 1, 2, 1, 2, NULL, '2024-01-15', '2024-07-15'),
    (3, 'REQ-COMM-001', 'The satellite shall maintain continuous communication with ground stations during 90% of each orbit period', 2, 1, 1, 1, 2, NULL, '2024-01-16', '2024-08-15'),
    (4, 'REQ-ACS-001', 'The satellite shall maintain pointing accuracy of ±0.1 degrees in all three axes during normal operations', 3, 1, 1, 1, 2, NULL, '2024-01-17', '2024-06-15'),
    (5, 'REQ-THERM-001', 'All electronic components shall operate within -20°C to +60°C temperature range throughout the mission', 4, 1, 1, 1, 2, NULL, '2024-01-18', '2024-07-15');

-- Point each requirement container to its current (first) version
UPDATE requirements r
SET current_version_id = (
    SELECT rv.id FROM requirement_versions rv
    WHERE rv.requirement_id = r.id
    ORDER BY rv.id ASC
    LIMIT 1
);

-- Tests for Space Project
INSERT INTO tests (reference_code, name, description, status_id, source, project_id) VALUES
    ('TEST-PWR-001', 'Solar Array Power Output Test', 'Verify solar array generates 500W under AM0 illumination in thermal vacuum chamber', 1, 'Solar array testing in thermal vacuum chamber', 1),
    ('TEST-PWR-002', 'Battery Endurance Discharge Test', 'Verify battery provides 200W for 45 minutes during discharge test cycle', 1, 'Battery cycle testing and capacity verification', 1),
    ('TEST-COMM-001', 'S-Band Communication Performance Test', 'Verify S-band communication link performance and data rate capabilities', 1, 'RF testing in anechoic chamber', 1),
    ('TEST-ACS-001', 'Star Tracker Pointing Accuracy Test', 'Verify star tracker pointing accuracy and attitude determination', 1, 'Star tracker calibration and pointing accuracy testing', 1),
    ('TEST-THERM-001', 'Thermal Vacuum Performance Test', 'Verify thermal control system performance in vacuum environment', 1, 'Thermal vacuum testing and temperature cycling', 1);

-- Traceability Matrix (requirements to tests mapping)
INSERT INTO matrix (req_id, test_id, project_id) VALUES
    (1, 1, 1),  -- REQ-PWR-001 -> TEST-PWR-001
    (2, 2, 1),  -- REQ-PWR-002 -> TEST-PWR-002
    (3, 3, 1),  -- REQ-COMM-001 -> TEST-COMM-001
    (4, 4, 1),  -- REQ-ACS-001 -> TEST-ACS-001
    (5, 5, 1);  -- REQ-THERM-001 -> TEST-THERM-001

-- Requirement version–verification links (Space Project: version 1–5 linked to verification methods 1–4)
INSERT INTO requirement_version_verification_methods (requirement_version_id, verification_method_id) VALUES
    (1, 1),  -- REQ-PWR-001 v1 -> Inspection
    (2, 2),  -- REQ-PWR-002 v1 -> Analysis
    (3, 1),  -- REQ-COMM-001 v1 -> Inspection
    (4, 2),  -- REQ-ACS-001 v1 -> Analysis
    (5, 4);  -- REQ-THERM-001 v1 -> Test

-- Sample audit logs
INSERT INTO logs (user_id, action_type, entity_type, entity_id, project_id, description, created_at) VALUES
    (1, 'CREATE', 'PROJECT', 1, 1, 'Space Project created by system administrator', NOW() - INTERVAL '1 day'),
    (1, 'CREATE', 'REQUIREMENT', 1, 1, 'Power requirement REQ-PWR-001 created by Dr. Smith', NOW() - INTERVAL '12 hours'),
    (2, 'UPDATE', 'REQUIREMENT', 2, 1, 'Power requirement REQ-PWR-002 status updated to Proposal', NOW() - INTERVAL '6 hours'),
    (3, 'CREATE', 'TEST', 1, 1, 'Test TEST-PWR-001 created by Engineer Jones', NOW() - INTERVAL '4 hours'),
    (4, 'UPDATE', 'TEST', 1, 1, 'Test TEST-PWR-001 status updated to Passed by Technician Lee', NOW() - INTERVAL '2 hours');

-- =============================================================================
-- COMPLETION MESSAGE
-- =============================================================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE 'ReqMan Database Initialization Complete';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Database Setup:';
    RAISE NOTICE '- 3 Projects created';
    RAISE NOTICE '- 6 Users created (all with password: password)';
    RAISE NOTICE '- 6 Requirement Status definitions';
    RAISE NOTICE '- 4 Test Status definitions';
    RAISE NOTICE '- 8 Categories for Space Project';
    RAISE NOTICE '- 6 Applicability definitions';
    RAISE NOTICE '- 4 Verification methods';
    RAISE NOTICE '- 5 Requirements (with initial versions) for Space Project';
    RAISE NOTICE '- 5 Requirement version–verification method links';
    RAISE NOTICE '- 5 Tests for Space Project';
    RAISE NOTICE '- 5 Traceability matrix entries';
    RAISE NOTICE '- 9 Project membership assignments';
    RAISE NOTICE '- 5 Sample audit logs';
    RAISE NOTICE '';
    RAISE NOTICE 'Login Credentials:';
    RAISE NOTICE '- Username: alice, Password: password (Admin)';
    RAISE NOTICE '- Username: dr_smith, Password: password (Admin)';
    RAISE NOTICE '- Username: eng_jones, Password: password';
    RAISE NOTICE '- Username: tech_lee, Password: password';
    RAISE NOTICE '- Username: qa_wilson, Password: password';
    RAISE NOTICE '- Username: admin, Password: password (Admin)';
    RAISE NOTICE '';
    RAISE NOTICE 'The database is ready for use!';
    RAISE NOTICE '========================================';
END $$;

-- =============================================================================
-- DIESEL MIGRATION HISTORY
-- =============================================================================
-- So "diesel migration run" in the app does not re-apply migrations already
-- reflected in this script (runs only when DB is created for the first time).
CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
    version VARCHAR(100) PRIMARY KEY NOT NULL
);

INSERT INTO __diesel_schema_migrations (version) VALUES
    ('2026-01-31-000001_baseline_schema'),
    ('2026-02-06-000001_seed_default_user'),
    ('2026-02-07-000001_requirement_versioning')
ON CONFLICT (version) DO NOTHING;