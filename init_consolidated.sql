-- =============================================================================
-- ReqMan Database Initialization Script
-- Consolidated version merging all diesel migrations with space project data
-- =============================================================================

-- Drop existing tables if they exist (in reverse dependency order)
DROP TABLE IF EXISTS matrix CASCADE;
DROP TABLE IF EXISTS logs CASCADE;
DROP TABLE IF EXISTS requirements CASCADE;
DROP TABLE IF EXISTS tests CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS applicability CASCADE;
DROP TABLE IF EXISTS verification CASCADE;
DROP TABLE IF EXISTS status CASCADE;
DROP TABLE IF EXISTS projects CASCADE;

-- =============================================================================
-- DIESEL HELPER FUNCTIONS
-- =============================================================================

-- Sets up a trigger for the given table to automatically set a column called
-- `updated_at` whenever the row is modified (unless `updated_at` was included
-- in the modified columns)
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

-- Projects table (multi-project support)
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
    user_creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_last_login TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_password VARCHAR(255) NOT NULL DEFAULT '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/HS.iK8i',
    is_admin BOOLEAN NOT NULL DEFAULT false
);

-- Status table
CREATE TABLE status (
    st_id SERIAL PRIMARY KEY,
    st_title VARCHAR NOT NULL DEFAULT ' ',
    st_description VARCHAR NOT NULL DEFAULT ' ',
    st_short_name VARCHAR NOT NULL DEFAULT ' '
);

-- Categories table (project-specific)
CREATE TABLE categories (
    cat_id SERIAL PRIMARY KEY,
    cat_title VARCHAR NOT NULL DEFAULT ' ',
    cat_description VARCHAR NOT NULL DEFAULT ' ',
    cat_tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Applicability table (project-specific)
CREATE TABLE applicability (
    app_id SERIAL PRIMARY KEY,
    app_title VARCHAR NOT NULL DEFAULT ' ',
    app_description VARCHAR NOT NULL DEFAULT ' ',
    app_tag VARCHAR NOT NULL DEFAULT ' ',
    project_id INTEGER NOT NULL
);

-- Verification table (project-specific)
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

-- Categories -> Projects
ALTER TABLE categories ADD CONSTRAINT fk_categories_project
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Applicability -> Projects
ALTER TABLE applicability ADD CONSTRAINT fk_applicability_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Verification -> Projects
ALTER TABLE verification ADD CONSTRAINT fk_verification_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Requirements -> Projects
ALTER TABLE requirements ADD CONSTRAINT fk_requirements_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Requirements -> Applicability
ALTER TABLE requirements ADD CONSTRAINT fk_requirements_applicability 
    FOREIGN KEY (req_applicability) REFERENCES applicability(app_id);

-- Tests -> Projects
ALTER TABLE tests ADD CONSTRAINT fk_tests_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Matrix -> Projects
ALTER TABLE matrix ADD CONSTRAINT fk_matrix_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);

-- Matrix -> Requirements
ALTER TABLE matrix ADD CONSTRAINT fk_matrix_requirements 
    FOREIGN KEY (matrix_req_id) REFERENCES requirements(req_id) ON DELETE CASCADE;

-- Matrix -> Tests
ALTER TABLE matrix ADD CONSTRAINT fk_matrix_tests 
    FOREIGN KEY (matrix_test_id) REFERENCES tests(test_id) ON DELETE CASCADE;

-- Logs -> Users
ALTER TABLE logs ADD CONSTRAINT fk_logs_user_id 
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE;

-- Logs -> Projects
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

-- Tests indexes
CREATE INDEX idx_tests_project_id ON tests(project_id);
CREATE INDEX idx_tests_status ON tests(test_status);

-- Matrix indexes
CREATE INDEX idx_matrix_project_id ON matrix(project_id);
CREATE INDEX idx_matrix_req_id ON matrix(matrix_req_id);
CREATE INDEX idx_matrix_test_id ON matrix(matrix_test_id);

-- =============================================================================
-- DEFAULT DATA
-- =============================================================================

-- Insert default projects
INSERT INTO projects (project_id, project_name, project_description, project_creation_date, project_status) VALUES
    (1, 'Space Project', 'Space exploration satellite requirements and test management', NOW(), 'Active'),
    (2, 'ReqMan Project', 'Requirements management system development', NOW(), 'Active'),
    (3, 'Empty Project', 'Empty project for testing and demonstration', NOW(), 'Active');

-- Insert default status values
INSERT INTO status (st_title, st_description, st_short_name) VALUES
    ('Draft', 'The requirement is still being edited', 'Drf'),
    ('Proposal', 'The requirement is still to be approved', 'Pro'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc'),
    ('Rejected', 'The requirement is not accepted', 'Rej'),
    ('Cancelled', 'The requirement is cancelled', 'Can'),
    ('Finished', 'The requirement is finished', 'Fsh'),
    ('Passed', 'The test has passed', 'Pass'),
    ('Failed', 'The test has failed', 'Fail');

-- Insert default users (Space Project team)
INSERT INTO users (user_username, user_name, user_email, is_admin) VALUES
    ('dr_smith', 'Dr. Sarah Smith', 'sarah.smith@spacecorp.com', true),
    ('eng_jones', 'Engineer Mike Jones', 'mike.jones@spacecorp.com', false),
    ('tech_lee', 'Technician Lisa Lee', 'lisa.lee@spacecorp.com', false),
    ('qa_wilson', 'QA Specialist Tom Wilson', 'tom.wilson@spacecorp.com', false),
    ('admin', 'System Administrator', 'admin@reqman.com', true);

-- Insert categories for Space Project
INSERT INTO categories (cat_title, cat_description, cat_tag, project_id) VALUES
    ('Power System', 'Solar panels, batteries, and power distribution', 'PWR', 1),
    ('Communication', 'Antennas, transponders, and data links', 'COMM', 1),
    ('Attitude Control', 'Gyroscopes, reaction wheels, and star trackers', 'ACS', 1),
    ('Thermal Control', 'Heat pipes, radiators, and thermal blankets', 'THERM', 1),
    ('Payload', 'Scientific instruments and mission equipment', 'PAY', 1),
    ('Propulsion', 'Thrusters and fuel systems', 'PROP', 1),
    ('Structure', 'Mechanical structure and deployment mechanisms', 'STRUCT', 1),
    ('Software', 'On-board computer systems and algorithms', 'SW', 1);

-- Insert applicability for Space Project
INSERT INTO applicability (app_title, app_description, app_tag, project_id) VALUES
    ('All Missions', 'Applies to all satellite missions', 'ALL', 1),
    ('Earth Observation', 'Low Earth orbit observation satellites', 'EO', 1),
    ('Communication', 'Geostationary communication satellites', 'COMM', 1),
    ('Navigation', 'GPS and navigation satellites', 'NAV', 1),
    ('Deep Space', 'Interplanetary and deep space missions', 'DEEP', 1),
    ('CubeSat', 'Small satellite missions', 'CUBE', 1);

-- Insert verification methods for Space Project
INSERT INTO verification (verification_name, verification_description, project_id) VALUES
    ('Inspection', 'Nondestructive examination of a system', 1),
    ('Analysis', 'Verification of a product or system using models, calculations and testing equipment', 1),
    ('Demonstration', 'The manipulation of the product or system as it is intended to be used to verify that the results are as planned or expected.', 1),
    ('Test', 'Verification of a product or system using a controlled and predefined series of inputs, data, or stimuli', 1);

-- =============================================================================
-- SPACE PROJECT REQUIREMENTS
-- =============================================================================

-- Power System Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-PWR-001', 'The satellite shall generate minimum 500W of electrical power during daylight operations', 'The solar array system must provide sufficient power to operate all subsystems including payload, communication, and attitude control systems during sunlit portions of the orbit.', 'REQ-PWR-001', 1, 1, 1, 1, 2, 0, 'https://spacecorp.com/power-specs', '2024-01-15', '2024-01-15', '2024-06-30', 1),
    ('REQ-PWR-002', 'The battery system shall provide 200W continuous power for 45 minutes during eclipse', 'During orbital eclipse periods, the battery system must maintain critical systems operation without degradation for the maximum expected eclipse duration.', 'REQ-PWR-002', 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-20', '2024-07-15', 1),
    ('REQ-PWR-003', 'The power distribution system shall provide redundant power paths to all critical subsystems', 'Each critical subsystem must have at least two independent power paths to ensure system reliability and fault tolerance.', 'REQ-PWR-003', 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-25', '2024-07-30', 1);

-- Communication System Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-COMM-001', 'The satellite shall maintain continuous communication with ground stations during 90% of each orbit', 'The communication system must provide reliable uplink and downlink capabilities with ground stations, accounting for orbital geometry and atmospheric conditions.', 'REQ-COMM-001', 2, 3, 1, 1, 2, 0, '', '2024-01-16', '2024-01-16', '2024-08-15', 1),
    ('REQ-COMM-002', 'The downlink data rate shall be minimum 10 Mbps for science data transmission', 'The communication system must support high-speed data transmission to accommodate payload data volume requirements.', 'REQ-COMM-002', 2, 2, 1, 1, 2, 0, '', '2024-01-16', '2024-01-18', '2024-08-30', 1),
    ('REQ-COMM-003', 'The system shall support S-band and X-band communication frequencies', 'Dual-frequency communication capability is required for redundancy and different mission phases.', 'REQ-COMM-003', 2, 2, 1, 1, 2, 0, '', '2024-01-16', '2024-01-19', '2024-09-15', 1);

-- Attitude Control Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-ACS-001', 'The satellite shall maintain pointing accuracy of ±0.1 degrees in all axes', 'The attitude control system must provide precise pointing for payload operations and communication antenna alignment.', 'REQ-ACS-001', 3, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-17', '2024-06-15', 1),
    ('REQ-ACS-002', 'The system shall perform autonomous attitude determination using star trackers', 'Star tracker-based attitude determination must provide continuous, accurate orientation data without ground intervention.', 'REQ-ACS-002', 3, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-19', '2024-06-30', 1),
    ('REQ-ACS-003', 'The reaction wheels shall provide momentum storage capacity of 20 Nms', 'Sufficient momentum storage is required for attitude control during various mission phases and disturbance conditions.', 'REQ-ACS-003', 3, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-20', '2024-07-15', 1);

-- Thermal Control Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-THERM-001', 'All electronic components shall operate within -20°C to +60°C temperature range', 'Thermal control system must maintain component temperatures within specified limits for reliable operation.', 'REQ-THERM-001', 4, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-18', '2024-07-15', 1),
    ('REQ-THERM-002', 'The payload compartment shall maintain temperature stability of ±2°C', 'Payload instruments require stable thermal environment for accurate measurements and calibration.', 'REQ-THERM-002', 4, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-20', '2024-07-30', 1),
    ('REQ-THERM-003', 'The thermal control system shall operate passively during normal operations', 'Passive thermal control using radiators and thermal blankets is preferred to minimize power consumption.', 'REQ-THERM-003', 4, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-22', '2024-08-15', 1);

-- Payload Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-PAY-001', 'The optical payload shall provide 1-meter ground resolution at 500km altitude', 'Earth observation payload must meet specified resolution requirements for mission objectives.', 'REQ-PAY-001', 5, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-19', '2024-08-15', 1),
    ('REQ-PAY-002', 'The payload shall operate in visible and near-infrared spectral bands', 'Multi-spectral imaging capability is required for comprehensive Earth observation data collection.', 'REQ-PAY-002', 5, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-21', '2024-08-30', 1),
    ('REQ-PAY-003', 'The payload data storage shall accommodate 24 hours of continuous imaging', 'On-board storage must support extended imaging operations without data loss.', 'REQ-PAY-003', 5, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-23', '2024-09-15', 1);

-- Propulsion Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-PROP-001', 'The propulsion system shall provide delta-V capability of 500 m/s', 'Sufficient delta-V is required for orbit maintenance, collision avoidance, and end-of-life disposal.', 'REQ-PROP-001', 6, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-20', '2024-09-15', 1),
    ('REQ-PROP-002', 'The thrusters shall provide minimum thrust of 1N for attitude control', 'Adequate thrust is required for attitude control during various mission phases.', 'REQ-PROP-002', 6, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-22', '2024-09-30', 1),
    ('REQ-PROP-003', 'The propulsion system shall use non-toxic propellants', 'Safety requirements mandate the use of non-toxic propellants for ground handling and launch operations.', 'REQ-PROP-003', 6, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-24', '2024-10-15', 1);

-- Structure Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-STRUCT-001', 'The satellite structure shall survive launch loads of 15g in all axes', 'Structural design must withstand launch vehicle vibration and acceleration environments.', 'REQ-STRUCT-001', 7, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-21', '2024-10-15', 1),
    ('REQ-STRUCT-002', 'The solar array deployment mechanism shall be 99% reliable', 'Solar array deployment is critical for mission success and must have high reliability.', 'REQ-STRUCT-002', 7, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-23', '2024-10-30', 1),
    ('REQ-STRUCT-003', 'The satellite shall fit within 1.2m x 1.2m x 2.0m launch envelope', 'Physical dimensions must comply with launch vehicle fairing constraints.', 'REQ-STRUCT-003', 7, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-25', '2024-11-15', 1);

-- Software Requirements
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date, project_id) VALUES
    ('REQ-SW-001', 'The on-board computer shall execute fault detection and recovery autonomously', 'Autonomous fault management is required for reliable operation without constant ground intervention.', 'REQ-SW-001', 8, 3, 1, 1, 2, 0, '', '2024-01-22', '2024-01-22', '2024-11-15', 1),
    ('REQ-SW-002', 'The software shall support over-the-air updates', 'Software update capability is required for bug fixes and feature enhancements during mission lifetime.', 'REQ-SW-002', 8, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-24', '2024-11-30', 1),
    ('REQ-SW-003', 'The system shall maintain accurate onboard time synchronization', 'Precise timekeeping is required for data correlation and mission operations.', 'REQ-SW-003', 8, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-26', '2024-12-15', 1);

-- =============================================================================
-- SPACE PROJECT TESTS
-- =============================================================================

-- Power System Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-PWR-001', 'Verify solar array generates 500W under AM0 illumination', 7, 'Solar array testing in thermal vacuum chamber', 1),
    ('TEST-PWR-002', 'Verify battery provides 200W for 45 minutes during discharge test', 7, 'Battery cycle testing', 1),
    ('TEST-PWR-003', 'Verify redundant power paths function independently', 7, 'Power distribution system testing', 1),
    ('TEST-PWR-004', 'Verify power system efficiency under various load conditions', 7, 'End-to-end power system testing', 1);

-- Communication System Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-COMM-001', 'Verify S-band communication link performance', 7, 'RF testing in anechoic chamber', 1),
    ('TEST-COMM-002', 'Verify X-band communication link performance', 7, 'RF testing in anechoic chamber', 1),
    ('TEST-COMM-003', 'Verify 10 Mbps data rate capability', 7, 'Data transmission testing', 1),
    ('TEST-COMM-004', 'Verify communication during simulated orbital conditions', 7, 'End-to-end communication testing', 1);

-- Attitude Control Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-ACS-001', 'Verify star tracker pointing accuracy', 7, 'Star tracker calibration testing', 1),
    ('TEST-ACS-002', 'Verify reaction wheel momentum capacity', 7, 'Reaction wheel testing', 1),
    ('TEST-ACS-003', 'Verify attitude control loop performance', 7, 'Control system testing', 1),
    ('TEST-ACS-004', 'Verify autonomous attitude determination', 7, 'System integration testing', 1);

-- Thermal Control Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-THERM-001', 'Verify thermal control system performance in vacuum', 7, 'Thermal vacuum testing', 1),
    ('TEST-THERM-002', 'Verify payload temperature stability', 7, 'Thermal cycling testing', 1),
    ('TEST-THERM-003', 'Verify passive thermal control effectiveness', 7, 'Thermal analysis and testing', 1),
    ('TEST-THERM-004', 'Verify thermal blankets installation and performance', 7, 'Thermal blanket testing', 1);

-- Payload Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-PAY-001', 'Verify optical payload resolution performance', 7, 'Optical testing in clean room', 1),
    ('TEST-PAY-002', 'Verify multi-spectral imaging capability', 7, 'Spectral calibration testing', 1),
    ('TEST-PAY-003', 'Verify payload data storage capacity', 7, 'Data storage testing', 1),
    ('TEST-PAY-004', 'Verify payload pointing accuracy', 7, 'Payload alignment testing', 1);

-- Propulsion Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-PROP-001', 'Verify thruster thrust performance', 7, 'Thruster hot fire testing', 1),
    ('TEST-PROP-002', 'Verify delta-V capability', 7, 'Propulsion system testing', 1),
    ('TEST-PROP-003', 'Verify propellant compatibility', 7, 'Material compatibility testing', 1),
    ('TEST-PROP-004', 'Verify propulsion system safety', 7, 'Safety testing', 1);

-- Structure Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-STRUCT-001', 'Verify structural integrity under launch loads', 7, 'Vibration testing', 1),
    ('TEST-STRUCT-002', 'Verify solar array deployment mechanism', 7, 'Deployment testing', 1),
    ('TEST-STRUCT-003', 'Verify satellite fits launch envelope', 7, 'Dimensional verification', 1),
    ('TEST-STRUCT-004', 'Verify structural thermal performance', 7, 'Thermal structural testing', 1);

-- Software Tests
INSERT INTO tests (test_name, test_description, test_status, test_source, project_id) VALUES
    ('TEST-SW-001', 'Verify fault detection and recovery algorithms', 7, 'Software testing', 1),
    ('TEST-SW-002', 'Verify over-the-air update capability', 7, 'Software update testing', 1),
    ('TEST-SW-003', 'Verify time synchronization accuracy', 7, 'Time synchronization testing', 1),
    ('TEST-SW-004', 'Verify software integration with hardware', 7, 'System integration testing', 1);

-- =============================================================================
-- TRACEABILITY MATRIX (SPACE PROJECT)
-- =============================================================================

-- Power System Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (1, 1), (1, 4),          -- REQ-PWR-001 -> Solar array and system efficiency tests
    (2, 2), (2, 4),          -- REQ-PWR-002 -> Battery and system efficiency tests
    (3, 3), (3, 4);          -- REQ-PWR-003 -> Redundant paths and system efficiency tests

-- Communication System Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (4, 5), (4, 6), (4, 8),  -- REQ-COMM-001 -> S-band, X-band, and end-to-end tests
    (5, 7), (5, 8),          -- REQ-COMM-002 -> Data rate and end-to-end tests
    (6, 5), (6, 6), (6, 8);  -- REQ-COMM-003 -> Dual frequency and end-to-end tests

-- Attitude Control Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (7, 9), (7, 11), (7, 12),  -- REQ-ACS-001 -> Star tracker, control loop, and integration tests
    (8, 9), (8, 12),          -- REQ-ACS-002 -> Star tracker and integration tests
    (9, 10), (9, 11), (9, 12); -- REQ-ACS-003 -> Reaction wheel, control loop, and integration tests

-- Thermal Control Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (10, 13), (10, 15),        -- REQ-THERM-001 -> Thermal vacuum and passive control tests
    (11, 14), (11, 16),        -- REQ-THERM-002 -> Payload temperature and blanket tests
    (12, 15), (12, 16);        -- REQ-THERM-003 -> Passive control and blanket tests

-- Payload Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (13, 17), (13, 20),        -- REQ-PAY-001 -> Resolution and pointing accuracy tests
    (14, 18), (14, 20),        -- REQ-PAY-002 -> Multi-spectral and pointing accuracy tests
    (15, 19), (15, 20);        -- REQ-PAY-003 -> Data storage and pointing accuracy tests

-- Propulsion Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (16, 21), (16, 22),        -- REQ-PROP-001 -> Thrust and delta-V tests
    (17, 21), (17, 23),        -- REQ-PROP-002 -> Thrust and compatibility tests
    (18, 23), (18, 24);        -- REQ-PROP-003 -> Compatibility and safety tests

-- Structure Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (19, 25), (19, 28),        -- REQ-STRUCT-001 -> Vibration and thermal structural tests
    (20, 26), (20, 27),        -- REQ-STRUCT-002 -> Deployment and dimensional tests
    (21, 27), (21, 28);        -- REQ-STRUCT-003 -> Dimensional and thermal structural tests

-- Software Requirements -> Tests
INSERT INTO matrix (matrix_req_id, matrix_test_id, project_id) VALUES
    (22, 29), (22, 32),        -- REQ-SW-001 -> Fault detection and integration tests
    (23, 30), (23, 32),        -- REQ-SW-002 -> Update capability and integration tests
    (24, 31), (24, 32);        -- REQ-SW-003 -> Time synchronization and integration tests

-- =============================================================================
-- SAMPLE LOG ENTRIES
-- =============================================================================

-- Insert sample log entries for audit trail
INSERT INTO logs (user_id, action_type, entity_type, entity_id, project_id, description, created_at) VALUES
    (1, 'CREATE', 'PROJECT', 1, 1, 'Space Project created', NOW() - INTERVAL '1 day'),
    (1, 'CREATE', 'REQUIREMENT', 1, 1, 'Power requirement REQ-PWR-001 created', NOW() - INTERVAL '12 hours'),
    (2, 'UPDATE', 'REQUIREMENT', 1, 1, 'Power requirement REQ-PWR-001 updated', NOW() - INTERVAL '6 hours'),
    (1, 'CREATE', 'TEST', 1, 1, 'Power test TEST-PWR-001 created', NOW() - INTERVAL '4 hours'),
    (3, 'LINK', 'MATRIX', NULL, 1, 'Requirement REQ-PWR-001 linked to test TEST-PWR-001', NOW() - INTERVAL '2 hours');

-- =============================================================================
-- FINAL COMMENTS
-- =============================================================================

-- This script creates a complete ReqMan database with:
-- 1. All tables with proper relationships and constraints
-- 2. Diesel helper functions for automatic timestamp management
-- 3. Performance indexes for optimal query performance
-- 4. Complete Space Project example data including:
--    - 3 projects (Space, ReqMan, Empty)
--    - 5 users with different roles
--    - 8 categories for space systems
--    - 6 applicability options for mission types
--    - 4 verification methods
--    - 24 requirements across 8 system categories
--    - 32 tests covering all requirement areas
--    - Complete traceability matrix linking requirements to tests
--    - Sample audit log entries
--
-- The database is ready for immediate use with the ReqMan application.
