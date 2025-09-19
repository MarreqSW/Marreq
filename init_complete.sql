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
-- CORE TABLES
-- =============================================================================

-- Projects table
CREATE TABLE projects (
    project_id SERIAL PRIMARY KEY,
    project_name VARCHAR(255) NOT NULL,
    project_description TEXT,
    project_creation_date TIMESTAMP,
    project_update_date TIMESTAMP,
    project_status VARCHAR(50),
    project_owner_id INTEGER
);

-- Users table
CREATE TABLE users (
    user_id SERIAL PRIMARY KEY,
    user_username VARCHAR NOT NULL,
    user_name VARCHAR NOT NULL,
    user_email VARCHAR NOT NULL DEFAULT ' ',
    user_level INTEGER NOT NULL DEFAULT 0,
    user_creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_last_login TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_password VARCHAR(255) NOT NULL DEFAULT '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m',
    project_id INTEGER,
    is_admin BOOLEAN NOT NULL DEFAULT false
);

-- Requirement Status table
CREATE TABLE requirement_status (
    req_st_id SERIAL PRIMARY KEY,
    req_st_title VARCHAR NOT NULL DEFAULT ' ',
    req_st_description VARCHAR NOT NULL DEFAULT ' ',
    req_st_short_name VARCHAR NOT NULL DEFAULT ' '
);

-- Test Status table
CREATE TABLE test_status (
    test_st_id SERIAL PRIMARY KEY,
    test_st_title VARCHAR NOT NULL DEFAULT ' ',
    test_st_description VARCHAR NOT NULL DEFAULT ' ',
    test_st_short_name VARCHAR NOT NULL DEFAULT ' '
);

-- Categories table
CREATE TABLE categories (
    cat_id SERIAL PRIMARY KEY,
    cat_title VARCHAR NOT NULL DEFAULT ' ',
    cat_description VARCHAR NOT NULL DEFAULT ' ',
    cat_tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Applicability table
CREATE TABLE applicability (
    app_id SERIAL PRIMARY KEY,
    app_title VARCHAR NOT NULL DEFAULT ' ',
    app_description VARCHAR NOT NULL DEFAULT ' ',
    app_tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Verification table
CREATE TABLE verification (
    verification_id SERIAL PRIMARY KEY,
    verification_name VARCHAR NOT NULL DEFAULT ' ',
    verification_description VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Requirements table
CREATE TABLE requirements (
    req_id SERIAL PRIMARY KEY,
    req_title VARCHAR NOT NULL DEFAULT ' ',
    req_description VARCHAR NOT NULL DEFAULT ' ',
    req_verification INTEGER NOT NULL DEFAULT 1,
    req_current_status INTEGER NOT NULL DEFAULT 1,
    req_author INTEGER NOT NULL DEFAULT 0,
    req_reviewer INTEGER NOT NULL DEFAULT 0,
    req_link VARCHAR NOT NULL DEFAULT ' ',
    req_reference VARCHAR NOT NULL DEFAULT ' ',
    req_category INTEGER NOT NULL DEFAULT 1,
    req_parent INTEGER NOT NULL DEFAULT 0,
    req_creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_update_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_deadline_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_applicability INTEGER NOT NULL DEFAULT 1,
    req_justification TEXT,
    project_id INTEGER NOT NULL
);

-- Tests table
CREATE TABLE tests (
    test_id SERIAL PRIMARY KEY,
    test_name VARCHAR NOT NULL DEFAULT ' ',
    test_description VARCHAR NOT NULL DEFAULT ' ',
    test_source VARCHAR NOT NULL DEFAULT ' ',
    test_status INTEGER NOT NULL DEFAULT 0,
    test_parent INTEGER NOT NULL DEFAULT 0,
    project_id INTEGER NOT NULL
);

-- Matrix table (traceability between requirements and tests)
CREATE TABLE matrix (
    matrix_req_id INTEGER NOT NULL,
    matrix_test_id INTEGER NOT NULL,
    matrix_creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    project_id INTEGER NOT NULL,
    PRIMARY KEY (matrix_req_id, matrix_test_id)
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

ALTER TABLE users ADD CONSTRAINT fk_users_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE categories ADD CONSTRAINT fk_categories_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE applicability ADD CONSTRAINT fk_applicability_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE verification ADD CONSTRAINT fk_verification_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE requirements ADD CONSTRAINT fk_requirements_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE requirements ADD CONSTRAINT fk_requirements_applicability 
    FOREIGN KEY (req_applicability) REFERENCES applicability(app_id);

ALTER TABLE requirements ADD CONSTRAINT fk_requirements_status 
    FOREIGN KEY (req_current_status) REFERENCES requirement_status(req_st_id);

ALTER TABLE tests ADD CONSTRAINT fk_tests_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE tests ADD CONSTRAINT fk_tests_status 
    FOREIGN KEY (test_status) REFERENCES test_status(test_st_id);

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_requirements 
    FOREIGN KEY (matrix_req_id) REFERENCES requirements(req_id) ON DELETE CASCADE;

ALTER TABLE matrix ADD CONSTRAINT fk_matrix_tests 
    FOREIGN KEY (matrix_test_id) REFERENCES tests(test_id) ON DELETE CASCADE;

ALTER TABLE logs ADD CONSTRAINT fk_logs_user_id 
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE;

ALTER TABLE logs ADD CONSTRAINT fk_logs_project_id 
    FOREIGN KEY (project_id) REFERENCES projects(project_id) ON DELETE CASCADE;

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

-- Requirements indexes
CREATE INDEX idx_requirements_project_id ON requirements(project_id);
CREATE INDEX idx_requirements_category ON requirements(req_category);
CREATE INDEX idx_requirements_status ON requirements(req_current_status);
CREATE INDEX idx_requirements_author ON requirements(req_author);
CREATE INDEX idx_requirements_reviewer ON requirements(req_reviewer);
CREATE INDEX idx_requirements_parent ON requirements(req_parent);

-- Tests indexes
CREATE INDEX idx_tests_project_id ON tests(project_id);
CREATE INDEX idx_tests_status ON tests(test_status);
CREATE INDEX idx_tests_parent ON tests(test_parent);

-- Matrix indexes
CREATE INDEX idx_matrix_project_id ON matrix(project_id);
CREATE INDEX idx_matrix_req_id ON matrix(matrix_req_id);
CREATE INDEX idx_matrix_test_id ON matrix(matrix_test_id);

-- Users indexes
CREATE INDEX idx_users_username ON users(user_username);
CREATE INDEX idx_users_project_id ON users(project_id);
CREATE INDEX idx_users_admin ON users(is_admin);

-- Categories indexes
CREATE INDEX idx_categories_project_id ON categories(project_id);
CREATE INDEX idx_categories_tag ON categories(cat_tag);

-- Applicability indexes
CREATE INDEX idx_applicability_project_id ON applicability(project_id);
CREATE INDEX idx_applicability_tag ON applicability(app_tag);

-- =============================================================================
-- DEFAULT DATA
-- =============================================================================

-- Projects
INSERT INTO projects (project_id, project_name, project_description, project_creation_date, project_status) VALUES
    (1, 'Space Project', 'Space exploration satellite requirements and test management system for advanced satellite missions', NOW(), 'Active'),
    (2, 'ReqMan Project', 'Requirements management system development and testing', NOW(), 'Active'),
    (3, 'Empty Project', 'Empty project for testing and demonstration purposes', NOW(), 'Active');

-- Requirement Status definitions
INSERT INTO requirement_status (req_st_title, req_st_description, req_st_short_name) VALUES
    ('Draft', 'The requirement is still being edited and developed', 'Drf'),
    ('Proposal', 'The requirement is proposed and awaiting approval', 'Pro'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc'),
    ('Rejected', 'The requirement is not accepted and needs revision', 'Rej'),
    ('Cancelled', 'The requirement is cancelled and will not be implemented', 'Can'),
    ('Finished', 'The requirement is finished and completed', 'Fsh');

-- Test Status definitions
INSERT INTO test_status (test_st_title, test_st_description, test_st_short_name) VALUES
    ('Passed', 'The test has passed all criteria', 'Pass'),
    ('Failed', 'The test has failed one or more criteria', 'Fail'),
    ('Pending', 'The test is pending execution', 'Pend'),
    ('In Progress', 'The test is currently being executed', 'Prog');

-- Users with working passwords (all users have password: 'password')
-- Password hash: $2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m
INSERT INTO users (user_username, user_name, user_email, user_level, project_id, is_admin, user_password) VALUES
    ('alice', 'Alice Johnson', 'alice@reqman.com', 1, 2, true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('dr_smith', 'Dr. Sarah Smith', 'sarah.smith@spacecorp.com', 1, 1, true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('eng_jones', 'Engineer Mike Jones', 'mike.jones@spacecorp.com', 1, 1, false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('tech_lee', 'Technician Lisa Lee', 'lisa.lee@spacecorp.com', 1, 1, false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('qa_wilson', 'QA Specialist Tom Wilson', 'tom.wilson@spacecorp.com', 1, 1, false, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'),
    ('admin', 'System Administrator', 'admin@reqman.com', 1, 2, true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m');

-- Categories for Space Project
INSERT INTO categories (cat_title, cat_description, cat_tag, project_id) VALUES
    ('Power System', 'Solar panels, batteries, and power distribution systems', 'PWR', 1),
    ('Communication', 'Antennas, transponders, and data communication links', 'COMM', 1),
    ('Attitude Control', 'Gyroscopes, reaction wheels, and star trackers for orientation', 'ACS', 1),
    ('Thermal Control', 'Heat pipes, radiators, and thermal blankets for temperature management', 'THERM', 1),
    ('Payload', 'Scientific instruments and mission-specific equipment', 'PAY', 1),
    ('Propulsion', 'Thrusters and fuel systems for orbital maneuvers', 'PROP', 1),
    ('Structure', 'Mechanical structure and deployment mechanisms', 'STRUCT', 1),
    ('Software', 'On-board computer systems and control algorithms', 'SW', 1);

-- Applicability definitions for Space Project
INSERT INTO applicability (app_title, app_description, app_tag, project_id) VALUES
    ('All Missions', 'Applies to all satellite missions regardless of type', 'ALL', 1),
    ('Earth Observation', 'Low Earth orbit observation and imaging satellites', 'EO', 1),
    ('Communication', 'Geostationary and medium Earth orbit communication satellites', 'COMM', 1),
    ('Navigation', 'GPS, GLONASS, and other navigation satellite systems', 'NAV', 1),
    ('Deep Space', 'Interplanetary and deep space exploration missions', 'DEEP', 1),
    ('CubeSat', 'Small satellite missions and CubeSat platforms', 'CUBE', 1);

-- Verification methods
INSERT INTO verification (verification_name, verification_description, project_id) VALUES
    ('Inspection', 'Nondestructive examination of a system or component', 1),
    ('Analysis', 'Verification using mathematical models and calculations', 1),
    ('Demonstration', 'Manipulation of the product as intended in its operational environment', 1),
    ('Test', 'Controlled verification with predefined inputs and expected outputs', 1);

-- Requirements for Space Project
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-PWR-001', 'The satellite shall generate minimum 500W of electrical power during daylight operations under AM0 illumination conditions', 'REQ-PWR-001', 1, 1, 1, 1, 1, 2, 0, 'https://spacecorp.com/power-specs', '2024-01-15', '2024-01-15', '2024-06-30', 1),
    ('REQ-PWR-002', 'The battery system shall provide 200W continuous power for 45 minutes during eclipse periods', 'REQ-PWR-002', 1, 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-20', '2024-07-15', 1),
    ('REQ-COMM-001', 'The satellite shall maintain continuous communication with ground stations during 90% of each orbit period', 'REQ-COMM-001', 2, 1, 3, 1, 1, 2, 0, '', '2024-01-16', '2024-01-16', '2024-08-15', 1),
    ('REQ-ACS-001', 'The satellite shall maintain pointing accuracy of ±0.1 degrees in all three axes during normal operations', 'REQ-ACS-001', 3, 1, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-17', '2024-06-15', 1),
    ('REQ-THERM-001', 'All electronic components shall operate within -20°C to +60°C temperature range throughout the mission', 'REQ-THERM-001', 4, 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-18', '2024-07-15', 1);

-- Tests for Space Project
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-PWR-001', 'Verify solar array generates 500W under AM0 illumination in thermal vacuum chamber', 1, 'Solar array testing in thermal vacuum chamber', 1),
    ('TEST-PWR-002', 'Verify battery provides 200W for 45 minutes during discharge test cycle', 1, 'Battery cycle testing and capacity verification', 1),
    ('TEST-COMM-001', 'Verify S-band communication link performance and data rate capabilities', 1, 'RF testing in anechoic chamber', 1),
    ('TEST-ACS-001', 'Verify star tracker pointing accuracy and attitude determination', 1, 'Star tracker calibration and pointing accuracy testing', 1),
    ('TEST-THERM-001', 'Verify thermal control system performance in vacuum environment', 1, 'Thermal vacuum testing and temperature cycling', 1);

-- Traceability Matrix (requirements to tests mapping)
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (1, 1, 1),  -- REQ-PWR-001 -> TEST-PWR-001
    (2, 2, 1),  -- REQ-PWR-002 -> TEST-PWR-002
    (3, 3, 1),  -- REQ-COMM-001 -> TEST-COMM-001
    (4, 4, 1),  -- REQ-ACS-001 -> TEST-ACS-001
    (5, 5, 1);  -- REQ-THERM-001 -> TEST-THERM-001

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
    RAISE NOTICE '- 5 Requirements for Space Project';
    RAISE NOTICE '- 5 Tests for Space Project';
    RAISE NOTICE '- 5 Traceability matrix entries';
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
