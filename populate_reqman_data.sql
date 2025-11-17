-- Clear existing data
DELETE FROM matrix;
DELETE FROM requirements;
DELETE FROM tests;
DELETE FROM users;
DELETE FROM categories;
DELETE FROM applicability;

-- Reset sequences
ALTER SEQUENCE requirements_req_id_seq RESTART WITH 1;
ALTER SEQUENCE tests_test_id_seq RESTART WITH 1;
ALTER SEQUENCE users_user_id_seq RESTART WITH 1;
ALTER SEQUENCE categories_cat_id_seq RESTART WITH 1;
ALTER SEQUENCE applicability_app_id_seq RESTART WITH 1;

-- Insert users
INSERT INTO users (name, email, user_role) VALUES
('Alice Johnson', 'alice@reqman.com', 'Requirements Engineer'),
('Bob Smith', 'bob@reqman.com', 'Test Engineer');

-- Insert categories
INSERT INTO categories (title, description, tag) VALUES
('Core Features', 'Essential functionality of the ReqMan system', 'CORE'),
('User Interface', 'Web interface and user experience features', 'UI'),
('Database', 'Database design and data management', 'DB'),
('API', 'REST API and external integrations', 'API'),
('Reporting', 'Export and reporting capabilities', 'REPORT'),
('Security', 'Authentication and authorization features', 'SEC'),
('Performance', 'System performance and optimization', 'PERF');

-- Insert applicability
INSERT INTO applicability (title, description, tag) VALUES
('All Products', 'Applies to all product lines', 'ALL'),
('Enterprise', 'Enterprise-level deployments', 'ENT'),
('Small Teams', 'Small team implementations', 'SMB'),
('Cloud', 'Cloud-based deployments', 'CLOUD');

-- Insert requirements for ReqMan project
INSERT INTO requirements (title, description, reference_code, category_id, applicability_id, current_status_id, verification_method_id, author_id, reviewer_id, req_parent_id, req_link, creation_date, update_date, deadline_date) VALUES
-- Core System Requirements
('REQ-SYS-001', 'The system shall provide a web-based interface for managing requirements and tests', 'The ReqMan system must offer a modern, responsive web interface that allows users to create, edit, view, and delete requirements and tests through a browser-based application.', 'REQ-SYS-001', 1, 1, 1, 1, 2, 0, 'https://github.com/reqman/web-interface', '2024-01-15', '2024-01-15', '2024-06-30'),
('REQ-SYS-002', 'The system shall support hierarchical requirements with parent-child relationships', 'Requirements must be organized in a hierarchical structure where child requirements can be linked to parent requirements, allowing for complex requirement decomposition and traceability.', 'REQ-SYS-002', 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-20', '2024-07-15'),
('REQ-SYS-003', 'The system shall provide a traceability matrix linking requirements to tests', 'A visual matrix must be generated showing the relationships between requirements and tests, indicating which tests verify which requirements.', 'REQ-SYS-003', 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-25', '2024-07-30'),

-- User Management Requirements
('REQ-USER-001', 'The system shall support user management with roles and permissions', 'Users must be able to register, login, and be assigned different roles (e.g., Requirements Engineer, Test Engineer, Reviewer) with appropriate permissions.', 'REQ-USER-001', 1, 3, 1, 1, 2, 0, '', '2024-01-16', '2024-01-16', '2024-08-15'),
('REQ-USER-002', 'The system shall allow assignment of authors and reviewers to requirements', 'Each requirement must have an assigned author who created it and a reviewer who validates it, with clear tracking of these assignments.', 'REQ-USER-002', 1, 2, 1, 1, 2, 0, '', '2024-01-16', '2024-01-18', '2024-08-30'),

-- Database Requirements
('REQ-DB-001', 'The system shall use PostgreSQL as the primary database', 'The application must be built on PostgreSQL database with proper schema design, indexes, and data integrity constraints.', 'REQ-DB-001', 1, 2, 1, 1, 2, 0, 'https://www.postgresql.org/', '2024-01-17', '2024-01-17', '2024-06-15'),
('REQ-DB-002', 'The system shall support database migrations for schema evolution', 'Database schema changes must be managed through version-controlled migrations that can be applied and rolled back safely.', 'REQ-DB-002', 1, 2, 1, 1, 2, 0, 'https://diesel.rs/', '2024-01-17', '2024-01-19', '2024-06-30'),

-- API Requirements
('REQ-API-001', 'The system shall provide a RESTful API for programmatic access', 'A comprehensive REST API must be available for all CRUD operations on requirements, tests, users, and other entities.', 'REQ-API-001', 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-18', '2024-07-15'),
('REQ-API-002', 'The API shall return data in JSON format', 'All API responses must be formatted as JSON with consistent structure and proper HTTP status codes.', 'REQ-API-002', 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-20', '2024-07-30'),

-- Category Management Requirements
('REQ-CAT-001', 'The system shall support user-defined categories for requirements', 'Users must be able to create, edit, and delete custom categories to organize requirements according to their needs.', 'REQ-CAT-001', 1, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-19', '2024-08-15'),
('REQ-CAT-002', 'Each requirement shall be assignable to one category', 'Requirements must be associated with exactly one category to maintain clear organization and filtering capabilities.', 'REQ-CAT-002', 1, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-21', '2024-08-30'),

-- Applicability Requirements
('REQ-APP-001', 'The system shall support applicability definitions for requirements', 'Users must be able to define applicability options (e.g., product lines, system types) to indicate which requirements apply to which contexts.', 'REQ-APP-001', 1, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-20', '2024-09-15'),
('REQ-APP-002', 'Each requirement shall have an assigned applicability', 'Requirements must be linked to specific applicability options to ensure proper scoping and filtering.', 'REQ-APP-002', 1, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-22', '2024-09-30'),

-- Export Requirements
('REQ-EXP-001', 'The system shall support Excel export of requirements with all metadata', 'Users must be able to export all requirements to Excel format including all fields such as title, description, category, applicability, status, dates, and relationships.', 'REQ-EXP-001', 1, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-21', '2024-10-15'),
('REQ-EXP-002', 'The system shall support Excel export of the traceability matrix', 'The traceability matrix must be exportable to Excel format showing the relationships between requirements and tests in a tabular format.', 'REQ-EXP-002', 1, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-23', '2024-10-30'),

-- UI Requirements
('REQ-UI-001', 'The system shall provide a modern, responsive web interface', 'The web interface must be built with modern CSS, be responsive across different screen sizes, and provide an intuitive user experience.', 'REQ-UI-001', 1, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-22', '2024-11-15'),
('REQ-UI-002', 'The interface shall use card-based layouts for better organization', 'Information must be presented using card-based layouts with consistent styling, proper spacing, and clear visual hierarchy.', 'REQ-UI-002', 1, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-24', '2024-11-30'),

-- Performance Requirements
('REQ-PERF-001', 'The system shall load pages within 2 seconds', 'All web pages must load and display content within 2 seconds under normal network conditions.', 'REQ-PERF-001', 1, 3, 1, 1, 2, 0, '', '2024-01-23', '2024-01-23', '2024-12-15'),
('REQ-PERF-002', 'The system shall support concurrent access by multiple users', 'The application must handle multiple simultaneous users without performance degradation or data corruption.', 'REQ-PERF-002', 1, 3, 1, 1, 2, 0, '', '2024-01-23', '2024-01-25', '2024-12-30');

-- Insert tests
INSERT INTO tests (test_title, description, status_id, source) VALUES
-- Core System Tests
('TEST-SYS-001', 'Verify web interface loads correctly and displays requirements list', 1, 'Manual testing of web interface'),
('TEST-SYS-002', 'Verify hierarchical requirements can be created and linked', 1, 'Database integration testing'),
('TEST-SYS-003', 'Verify traceability matrix displays correct relationships', 1, 'UI automation testing'),
('TEST-SYS-004', 'Verify user registration and login functionality', 1, 'Security testing'),
('TEST-SYS-005', 'Verify author and reviewer assignments work correctly', 1, 'Database testing'),

-- Database Tests
('TEST-DB-001', 'Verify PostgreSQL connection and basic CRUD operations', 1, 'Database integration testing'),
('TEST-DB-002', 'Verify database migrations can be applied and rolled back', 1, 'Migration testing'),
('TEST-DB-003', 'Verify data integrity constraints are enforced', 1, 'Database constraint testing'),

-- API Tests
('TEST-API-001', 'Verify REST API endpoints return correct JSON responses', 1, 'API testing with Postman'),
('TEST-API-002', 'Verify all CRUD operations work through API', 1, 'API integration testing'),
('TEST-API-003', 'Verify API authentication and authorization', 1, 'Security testing'),

-- Category Tests
('TEST-CAT-001', 'Verify category creation, editing, and deletion', 1, 'UI testing'),
('TEST-CAT-002', 'Verify requirements can be assigned to categories', 1, 'Database testing'),
('TEST-CAT-003', 'Verify category filtering works correctly', 1, 'UI testing'),

-- Applicability Tests
('TEST-APP-001', 'Verify applicability options can be managed', 1, 'UI testing'),
('TEST-APP-002', 'Verify requirements can be assigned applicability', 1, 'Database testing'),
('TEST-APP-003', 'Verify applicability filtering works correctly', 1, 'UI testing'),

-- Export Tests
('TEST-EXP-001', 'Verify requirements export to Excel includes all fields', 1, 'File generation testing'),
('TEST-EXP-002', 'Verify matrix export to Excel is correctly formatted', 1, 'File generation testing'),
('TEST-EXP-003', 'Verify exported files can be opened in Excel', 1, 'File compatibility testing'),

-- UI Tests
('TEST-UI-001', 'Verify responsive design works on different screen sizes', 1, 'Cross-browser testing'),
('TEST-UI-002', 'Verify card-based layouts display correctly', 1, 'UI testing'),
('TEST-UI-003', 'Verify navigation and user flow works smoothly', 1, 'Usability testing'),

-- Performance Tests
('TEST-PERF-001', 'Verify page load times are under 2 seconds', 1, 'Performance testing'),
('TEST-PERF-002', 'Verify system handles concurrent user access', 1, 'Load testing'),
('TEST-PERF-003', 'Verify database queries are optimized', 1, 'Database performance testing');

-- Create traceability matrix links
INSERT INTO matrix (req_id, id) VALUES
-- Core System Requirements -> Tests
(1, 1), (1, 2), (1, 3),  -- REQ-SYS-001 -> UI and hierarchy tests
(2, 2), (2, 3),          -- REQ-SYS-002 -> Hierarchy and matrix tests
(3, 3), (3, 18),         -- REQ-SYS-003 -> Matrix display and export tests

-- User Management Requirements -> Tests
(4, 4), (4, 10),         -- REQ-USER-001 -> User management and API tests
(5, 5), (5, 10),         -- REQ-USER-002 -> Assignment and API tests

-- Database Requirements -> Tests
(6, 6), (6, 7),          -- REQ-DB-001 -> Database connection and migration tests
(7, 7), (7, 8),          -- REQ-DB-002 -> Migration and constraint tests

-- API Requirements -> Tests
(8, 9), (8, 10),         -- REQ-API-001 -> API response and CRUD tests
(9, 9), (9, 10),         -- REQ-API-002 -> API response and CRUD tests

-- Category Requirements -> Tests
(10, 12), (10, 13), (10, 14),  -- REQ-CAT-001 -> Category management tests
(11, 12), (11, 13), (11, 14),  -- REQ-CAT-002 -> Category assignment tests

-- Applicability Requirements -> Tests
(12, 15), (12, 16), (12, 17),  -- REQ-APP-001 -> Applicability management tests
(13, 15), (13, 16), (13, 17),  -- REQ-APP-002 -> Applicability assignment tests

-- Export Requirements -> Tests
(14, 18), (14, 20),      -- REQ-EXP-001 -> Requirements export tests
(15, 19), (15, 20),      -- REQ-EXP-002 -> Matrix export tests

-- UI Requirements -> Tests
(16, 21), (16, 22), (16, 23),  -- REQ-UI-001 -> UI responsive and layout tests
(17, 22), (17, 23),            -- REQ-UI-002 -> Card layout and navigation tests

-- Performance Requirements -> Tests
(18, 24), (18, 25), (18, 26),  -- REQ-PERF-001 -> Performance and load tests
(19, 25), (19, 26);            -- REQ-PERF-002 -> Concurrent access and optimization tests 