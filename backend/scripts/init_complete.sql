-- =============================================================================
-- Marreq Sample Data Seed Script
-- =============================================================================
-- Purpose:
--   Populate a migrated Marreq database with rich demo data.
--
-- Important:
--   - This script does NOT create schema objects.
--   - Run migrations first (diesel or migrations/*/up.sql).
--   - Intended for an empty database (no projects yet).
--
-- Usage:
--   diesel migration run
--   psql "$DATABASE_URL" -f scripts/init_complete.sql
-- =============================================================================

-- =============================================================================
-- PREFLIGHT CHECKS
-- =============================================================================

DO $$
DECLARE
    _missing_table TEXT;
BEGIN
    -- Ensure migrations were applied before seeding.
    FOREACH _missing_table IN ARRAY ARRAY[
        'projects',
        'users',
        'project_members',
        'requirement_status',
        'verification_status',
        'categories',
        'applicability',
        'verification_methods',
        'requirements',
        'requirement_versions',
        'verifications',
        'matrix',
        'logs',
        'custom_field_definitions',
        'custom_field_values',
        'requirement_version_verification_methods',
        'requirement_version_links',
        'baselines',
        'baseline_requirements',
        'baseline_traceability',
        'baseline_verifications',
        'requirement_embeddings',
        'embedding_index_queue'
    ]
    LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = _missing_table
        ) THEN
            RAISE EXCEPTION
                'Missing table "%". Run migrations before executing scripts/init_complete.sql.',
                _missing_table;
        END IF;
    END LOOP;

    -- Protect from accidental duplicate/demo-data overlay.
    IF EXISTS (SELECT 1 FROM projects LIMIT 1) THEN
        RAISE EXCEPTION 'Seed aborted: projects table is not empty. Use this script on a fresh migrated DB only.';
    END IF;
END
$$;

-- =============================================================================
-- INITIAL DATA
-- =============================================================================

-- Projects
INSERT INTO projects (name, slug, description, creation_date, status) VALUES
    ('Space Project', 'space-project', 'Space exploration satellite requirements and test management system for advanced satellite missions', NOW(), 'active'),
    ('Marreq Project', 'marreq-project', 'Requirements management system development and testing', NOW(), 'active'),
    ('Empty Project', 'empty-project', 'Empty project for testing and demonstration purposes', NOW(), 'active');

-- Requirement status definitions (is_system = true: default set, not editable/deletable)
-- tag_color: #RRGGBB for SPA/classic UIs (must match frontend TagColorPicker / StatusBadge)
INSERT INTO requirement_status (title, description, tag, project_id, is_system, tag_color) VALUES
    ('Draft', 'The requirement is still being edited and developed', 'Drf', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#64748b'),
    ('Proposal', 'The requirement is proposed and awaiting approval', 'Pro', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#7c3aed'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#15803d'),
    ('Rejected', 'The requirement is not accepted and needs revision', 'Rej', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#b91c1c'),
    ('Cancelled', 'The requirement is cancelled and will not be implemented', 'Can', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#57534e'),
    ('Finished', 'The requirement is finished and completed', 'Fsh', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#0e7490'),
    ('Draft', 'The requirement is still being edited and developed', 'Drf', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#64748b'),
    ('Proposal', 'The requirement is proposed and awaiting approval', 'Pro', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#7c3aed'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#15803d'),
    ('Rejected', 'The requirement is not accepted and needs revision', 'Rej', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#b91c1c'),
    ('Cancelled', 'The requirement is cancelled and will not be implemented', 'Can', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#57534e'),
    ('Finished', 'The requirement is finished and completed', 'Fsh', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#0e7490');

-- Verification status definitions (is_system = true: default set, not editable/deletable)
INSERT INTO verification_status (title, description, tag, project_id, is_system, tag_color) VALUES
    ('Passed', 'The test has passed all criteria', 'Pass', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#15803d'),
    ('Failed', 'The test has failed one or more criteria', 'Fail', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#b91c1c'),
    ('Pending', 'The test is pending execution', 'Pend', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#b45309'),
    ('In Progress', 'The test is currently being executed', 'Prog', (SELECT id FROM projects WHERE name = 'Space Project'), true, '#1d4ed8'),
    ('Passed', 'The test has passed all criteria', 'Pass', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#15803d'),
    ('Failed', 'The test has failed one or more criteria', 'Fail', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#b91c1c'),
    ('Pending', 'The test is pending execution', 'Pend', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#b45309'),
    ('In Progress', 'The test is currently being executed', 'Prog', (SELECT id FROM projects WHERE name = 'Marreq Project'), true, '#1d4ed8');

-- Users with working passwords (all users have password: ChangeMe123!)
-- Password hash (Argon2id):
-- $argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0
INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'alice', 'Alice Johnson', 'alice@marreq.com', true,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'alice');

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'dr_smith', 'Dr. Sarah Smith', 'sarah.smith@spacecorp.com', true,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'dr_smith');

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'eng_jones', 'Engineer Mike Jones', 'mike.jones@spacecorp.com', false,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'eng_jones');

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'tech_lee', 'Technician Lisa Lee', 'lisa.lee@spacecorp.com', false,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'tech_lee');

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'qa_wilson', 'QA Specialist Tom Wilson', 'tom.wilson@spacecorp.com', false,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'qa_wilson');

INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'sysadmin', 'System Administrator', 'admin@marreq.com', true,
       '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users WHERE username = 'sysadmin');

-- Project ownership for namespace-based project URLs
UPDATE projects
SET owner_id = (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1)
WHERE slug = 'space-project';

UPDATE projects
SET owner_id = (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1)
WHERE slug = 'marreq-project';

UPDATE projects
SET owner_id = (SELECT id FROM users WHERE username = 'sysadmin' ORDER BY id LIMIT 1)
WHERE slug = 'empty-project';

-- Project membership assignments (role: 1=Owner, 2=Manager, 3=Contributor, 4=Viewer)
INSERT INTO project_members (project_id, user_id, role) VALUES
    ((SELECT id FROM projects WHERE name = 'Space Project'), (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1), 1),
    ((SELECT id FROM projects WHERE name = 'Space Project'), (SELECT id FROM users WHERE username = 'eng_jones' ORDER BY id LIMIT 1), 3),
    ((SELECT id FROM projects WHERE name = 'Space Project'), (SELECT id FROM users WHERE username = 'tech_lee' ORDER BY id LIMIT 1), 3),
    ((SELECT id FROM projects WHERE name = 'Space Project'), (SELECT id FROM users WHERE username = 'qa_wilson' ORDER BY id LIMIT 1), 4),
    ((SELECT id FROM projects WHERE name = 'Marreq Project'), (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1), 1),
    ((SELECT id FROM projects WHERE name = 'Marreq Project'), (SELECT id FROM users WHERE username = 'sysadmin' ORDER BY id LIMIT 1), 2),
    ((SELECT id FROM projects WHERE name = 'Marreq Project'), (SELECT id FROM users WHERE username = 'qa_wilson' ORDER BY id LIMIT 1), 3),
    ((SELECT id FROM projects WHERE name = 'Empty Project'), (SELECT id FROM users WHERE username = 'sysadmin' ORDER BY id LIMIT 1), 1),
    ((SELECT id FROM projects WHERE name = 'Empty Project'), (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1), 2);

-- Categories for Space Project
INSERT INTO categories (title, description, tag, project_id) VALUES
    ('Power System', 'Solar panels, batteries, and power distribution systems', 'PWR', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Communication', 'Antennas, transponders, and data communication links', 'COMM', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Attitude Control', 'Gyroscopes, reaction wheels, and star trackers for orientation', 'ACS', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Thermal Control', 'Heat pipes, radiators, and thermal blankets for temperature management', 'THERM', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Payload', 'Scientific instruments and mission-specific equipment', 'PAY', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Propulsion', 'Thrusters and fuel systems for orbital maneuvers', 'PROP', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Structure', 'Mechanical structure and deployment mechanisms', 'STRUCT', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Software', 'On-board computer systems and control algorithms', 'SW', (SELECT id FROM projects WHERE name = 'Space Project'));

-- Categories for Marreq Project
INSERT INTO categories (title, description, tag, project_id) VALUES
    ('User Interface', 'User interface components and functionality', 'UI', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Backend', 'Server-side logic and API endpoints', 'BE', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Database', 'Database schema and data management', 'DB', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Authentication', 'User authentication and authorization', 'AUTH', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Documentation', 'Technical and user documentation', 'DOC', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Testing', 'Test infrastructure and test cases', 'TEST', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Performance', 'System performance and optimization', 'PERF', (SELECT id FROM projects WHERE name = 'Marreq Project'));

-- Applicability definitions for Space Project
INSERT INTO applicability (title, description, tag, project_id) VALUES
    ('All Missions', 'Applies to all satellite missions regardless of type', 'ALL', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Earth Observation', 'Low Earth orbit observation and imaging satellites', 'EO', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Communication', 'Geostationary and medium Earth orbit communication satellites', 'COMM', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Navigation', 'GPS, GLONASS, and other navigation satellite systems', 'NAV', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Deep Space', 'Interplanetary and deep space exploration missions', 'DEEP', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('CubeSat', 'Small satellite missions and CubeSat platforms', 'CUBE', (SELECT id FROM projects WHERE name = 'Space Project'));

-- Applicability definitions for Marreq Project
INSERT INTO applicability (title, description, tag, project_id) VALUES
    ('All Users', 'Applies to all user types', 'ALL', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Administrators', 'Applies to system administrators only', 'ADMIN', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Project Managers', 'Applies to project managers and owners', 'MGR', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Contributors', 'Applies to regular contributors', 'CONT', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Viewers', 'Applies to read-only viewers', 'VIEW', (SELECT id FROM projects WHERE name = 'Marreq Project'));

-- Verification methods for Space Project
INSERT INTO verification_methods (title, description, tag, project_id) VALUES
    ('Inspection', 'Nondestructive examination of a system or component', 'INSP', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Analysis', 'Verification using mathematical models and calculations', 'ANALYSIS', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Demonstration', 'Manipulation of the product as intended in its operational environment', 'DEMO', (SELECT id FROM projects WHERE name = 'Space Project')),
    ('Test', 'Controlled verification with predefined inputs and expected outputs', 'TEST', (SELECT id FROM projects WHERE name = 'Space Project'));

-- Verification methods for Marreq Project
INSERT INTO verification_methods (title, description, tag, project_id) VALUES
    ('Code Review', 'Review of source code by peers', 'REVIEW', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Unit Test', 'Automated unit testing', 'UNIT', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Integration Test', 'Testing of integrated components', 'INTEG', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('System Test', 'End-to-end system testing', 'SYS', (SELECT id FROM projects WHERE name = 'Marreq Project')),
    ('Manual Test', 'Manual testing by QA team', 'MANUAL', (SELECT id FROM projects WHERE name = 'Marreq Project'));

-- Requirements for Space Project (containers only; content in requirement_versions)
INSERT INTO requirements (project_id, stable_code) VALUES
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'REQ-PWR-001'),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'REQ-PWR-002'),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'REQ-COMM-001'),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'REQ-ACS-001'),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'REQ-THERM-001');

-- Initial versions (v1) for each requirement
INSERT INTO requirement_versions (
    requirement_id,
    title,
    description,
    category_id,
    applicability_id,
    status_id,
    author_id,
    reviewer_id,
    created_at,
    deadline_date
) VALUES
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-001' ORDER BY id LIMIT 1),
        'REQ-PWR-001',
        'The satellite shall generate minimum 500W of electrical power during daylight operations under AM0 illumination conditions',
        (SELECT id FROM categories WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'PWR' ORDER BY id LIMIT 1),
        (SELECT id FROM applicability WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ALL' ORDER BY id LIMIT 1),
        (SELECT id FROM requirement_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Drf' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        '2024-01-15',
        '2024-06-30'
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-002' ORDER BY id LIMIT 1),
        'REQ-PWR-002',
        'The battery system shall provide 200W continuous power for 45 minutes during eclipse periods',
        (SELECT id FROM categories WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'PWR' ORDER BY id LIMIT 1),
        (SELECT id FROM applicability WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ALL' ORDER BY id LIMIT 1),
        (SELECT id FROM requirement_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pro' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        '2024-01-15',
        '2024-07-15'
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-COMM-001' ORDER BY id LIMIT 1),
        'REQ-COMM-001',
        'The satellite shall maintain continuous communication with ground stations during 90% of each orbit period',
        (SELECT id FROM categories WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'COMM' ORDER BY id LIMIT 1),
        (SELECT id FROM applicability WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ALL' ORDER BY id LIMIT 1),
        (SELECT id FROM requirement_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Drf' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        '2024-01-16',
        '2024-08-15'
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-ACS-001' ORDER BY id LIMIT 1),
        'REQ-ACS-001',
        'The satellite shall maintain pointing accuracy of +/-0.1 degrees in all three axes during normal operations',
        (SELECT id FROM categories WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ACS' ORDER BY id LIMIT 1),
        (SELECT id FROM applicability WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ALL' ORDER BY id LIMIT 1),
        (SELECT id FROM requirement_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Drf' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        '2024-01-17',
        '2024-06-15'
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-THERM-001' ORDER BY id LIMIT 1),
        'REQ-THERM-001',
        'All electronic components shall operate within -20C to +60C temperature range throughout the mission',
        (SELECT id FROM categories WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'THERM' ORDER BY id LIMIT 1),
        (SELECT id FROM applicability WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ALL' ORDER BY id LIMIT 1),
        (SELECT id FROM requirement_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Drf' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        '2024-01-18',
        '2024-07-15'
    );

-- Point each requirement container to its current (first) version and set first_created_at
UPDATE requirements r
SET
    current_version_id = (
        SELECT rv.id
        FROM requirement_versions rv
        WHERE rv.requirement_id = r.id
        ORDER BY rv.id ASC
        LIMIT 1
    ),
    first_created_at = (
        SELECT rv.created_at
        FROM requirement_versions rv
        WHERE rv.requirement_id = r.id
        ORDER BY rv.id ASC
        LIMIT 1
    )
WHERE r.current_version_id IS NULL;

-- Verifications (test cases) for Space Project
INSERT INTO verifications (reference_code, name, description, status_id, source, project_id) VALUES
    (
        'TEST-PWR-001',
        'Solar Array Power Output Test',
        'Verify solar array generates 500W under AM0 illumination in thermal vacuum chamber',
        (SELECT id FROM verification_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pass' ORDER BY id LIMIT 1),
        'Solar array testing in thermal vacuum chamber',
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        'TEST-PWR-002',
        'Battery Endurance Discharge Test',
        'Verify battery provides 200W for 45 minutes during discharge test cycle',
        (SELECT id FROM verification_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pass' ORDER BY id LIMIT 1),
        'Battery cycle testing and capacity verification',
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        'TEST-COMM-001',
        'S-Band Communication Performance Test',
        'Verify S-band communication link performance and data rate capabilities',
        (SELECT id FROM verification_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pass' ORDER BY id LIMIT 1),
        'RF testing in anechoic chamber',
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        'TEST-ACS-001',
        'Star Tracker Pointing Accuracy Test',
        'Verify star tracker pointing accuracy and attitude determination',
        (SELECT id FROM verification_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pass' ORDER BY id LIMIT 1),
        'Star tracker calibration and pointing accuracy testing',
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        'TEST-THERM-001',
        'Thermal Vacuum Performance Test',
        'Verify thermal control system performance in vacuum environment',
        (SELECT id FROM verification_status WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'Pass' ORDER BY id LIMIT 1),
        'Thermal vacuum testing and temperature cycling',
        (SELECT id FROM projects WHERE name = 'Space Project')
    );

-- Traceability matrix (requirements to verifications mapping)
INSERT INTO matrix (req_id, verification_id, project_id) VALUES
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-001' ORDER BY id LIMIT 1),
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-PWR-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-002' ORDER BY id LIMIT 1),
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-PWR-002' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-COMM-001' ORDER BY id LIMIT 1),
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-COMM-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-ACS-001' ORDER BY id LIMIT 1),
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-ACS-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project')
    ),
    (
        (SELECT id FROM requirements WHERE stable_code = 'REQ-THERM-001' ORDER BY id LIMIT 1),
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-THERM-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project')
    );

-- Requirement version-verification links
INSERT INTO requirement_version_verification_methods (requirement_version_id, verification_method_id) VALUES
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM verification_methods WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'INSP' ORDER BY id LIMIT 1)
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-002' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM verification_methods WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ANALYSIS' ORDER BY id LIMIT 1)
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-COMM-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM verification_methods WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'INSP' ORDER BY id LIMIT 1)
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-ACS-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM verification_methods WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'ANALYSIS' ORDER BY id LIMIT 1)
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-THERM-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM verification_methods WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND tag = 'TEST' ORDER BY id LIMIT 1)
    );

-- Custom field definitions (Space Project)
INSERT INTO custom_field_definitions (project_id, label, field_type, enum_values, sort_order) VALUES
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'Component', 'text', NULL, 0),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'Risk', 'enum', '["Low", "Medium", "High"]'::jsonb, 1),
    ((SELECT id FROM projects WHERE name = 'Space Project'), 'Priority', 'number', NULL, 2);

-- Sample custom field values for first three requirement versions
INSERT INTO custom_field_values (requirement_version_id, custom_field_definition_id, value) VALUES
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Component' ORDER BY id LIMIT 1),
        'Power System'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Risk' ORDER BY id LIMIT 1),
        'Medium'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Priority' ORDER BY id LIMIT 1),
        '1'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-002' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Component' ORDER BY id LIMIT 1),
        'Power System'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-002' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Risk' ORDER BY id LIMIT 1),
        'Low'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-PWR-002' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Priority' ORDER BY id LIMIT 1),
        '2'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-COMM-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Component' ORDER BY id LIMIT 1),
        'Communication'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-COMM-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Risk' ORDER BY id LIMIT 1),
        'High'
    ),
    (
        (SELECT rv.id FROM requirement_versions rv JOIN requirements r ON r.id = rv.requirement_id WHERE r.stable_code = 'REQ-COMM-001' ORDER BY rv.id LIMIT 1),
        (SELECT id FROM custom_field_definitions WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND label = 'Priority' ORDER BY id LIMIT 1),
        '1'
    );

-- Sample audit logs
INSERT INTO logs (user_id, action_type, entity_type, entity_id, project_id, description, created_at) VALUES
    (
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        'CREATE',
        'PROJECT',
        (SELECT id FROM projects WHERE name = 'Space Project'),
        (SELECT id FROM projects WHERE name = 'Space Project'),
        'Space Project created by system administrator',
        NOW() - INTERVAL '1 day'
    ),
    (
        (SELECT id FROM users WHERE username = 'alice' ORDER BY id LIMIT 1),
        'CREATE',
        'REQUIREMENT',
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project'),
        'Power requirement REQ-PWR-001 created by Dr. Smith',
        NOW() - INTERVAL '12 hours'
    ),
    (
        (SELECT id FROM users WHERE username = 'dr_smith' ORDER BY id LIMIT 1),
        'UPDATE',
        'REQUIREMENT',
        (SELECT id FROM requirements WHERE stable_code = 'REQ-PWR-002' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project'),
        'Power requirement REQ-PWR-002 status updated to Proposal',
        NOW() - INTERVAL '6 hours'
    ),
    (
        (SELECT id FROM users WHERE username = 'eng_jones' ORDER BY id LIMIT 1),
        'CREATE',
        'TEST',
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-PWR-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project'),
        'Test TEST-PWR-001 created by Engineer Jones',
        NOW() - INTERVAL '4 hours'
    ),
    (
        (SELECT id FROM users WHERE username = 'tech_lee' ORDER BY id LIMIT 1),
        'UPDATE',
        'TEST',
        (SELECT id FROM verifications WHERE project_id = (SELECT id FROM projects WHERE name = 'Space Project') AND reference_code = 'TEST-PWR-001' ORDER BY id LIMIT 1),
        (SELECT id FROM projects WHERE name = 'Space Project'),
        'Test TEST-PWR-001 status updated to Passed by Technician Lee',
        NOW() - INTERVAL '2 hours'
    );

-- =============================================================================
-- COMPLETION MESSAGE
-- =============================================================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Marreq Sample Data Seed Complete';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Seeded Data:';
    RAISE NOTICE '- 3 Projects created';
    RAISE NOTICE '- 6 Users available (password: ChangeMe123!)';
    RAISE NOTICE '- 12 Requirement status definitions';
    RAISE NOTICE '- 8 Test status definitions';
    RAISE NOTICE '- 15 Categories total';
    RAISE NOTICE '- 11 Applicability definitions';
    RAISE NOTICE '- 9 Verification methods';
    RAISE NOTICE '- 5 Requirements (with initial versions) for Space Project';
    RAISE NOTICE '- 5 Requirement version-verification links';
    RAISE NOTICE '- 5 Tests for Space Project';
    RAISE NOTICE '- 5 Traceability matrix entries';
    RAISE NOTICE '- 3 Custom field definitions';
    RAISE NOTICE '- 9 Custom field value samples';
    RAISE NOTICE '- 9 Project membership assignments';
    RAISE NOTICE '- 5 Sample audit logs';
    RAISE NOTICE '';
    RAISE NOTICE 'Login Credentials:';
    RAISE NOTICE '- Username: alice, Password: ChangeMe123! (Admin)';
    RAISE NOTICE '- Username: dr_smith, Password: ChangeMe123! (Admin)';
    RAISE NOTICE '- Username: eng_jones, Password: ChangeMe123!';
    RAISE NOTICE '- Username: tech_lee, Password: ChangeMe123!';
    RAISE NOTICE '- Username: qa_wilson, Password: ChangeMe123!';
    RAISE NOTICE '- Username: sysadmin, Password: ChangeMe123! (Admin)';
    RAISE NOTICE '';
    RAISE NOTICE 'The database is ready for use!';
    RAISE NOTICE '========================================';
END
$$;
