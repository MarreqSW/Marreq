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

-- Insert users (space project team)
INSERT INTO users (user_username, user_name, user_email, user_level) VALUES
('dr_smith', 'Dr. Sarah Smith', 'sarah.smith@spacecorp.com', 1),
('eng_jones', 'Engineer Mike Jones', 'mike.jones@spacecorp.com', 1),
('tech_lee', 'Technician Lisa Lee', 'lisa.lee@spacecorp.com', 1),
('qa_wilson', 'QA Specialist Tom Wilson', 'tom.wilson@spacecorp.com', 1);

-- Insert categories (space system components)
INSERT INTO categories (cat_title, cat_description, cat_tag) VALUES
('Power System', 'Solar panels, batteries, and power distribution', 'PWR'),
('Communication', 'Antennas, transponders, and data links', 'COMM'),
('Attitude Control', 'Gyroscopes, reaction wheels, and star trackers', 'ACS'),
('Thermal Control', 'Heat pipes, radiators, and thermal blankets', 'THERM'),
('Payload', 'Scientific instruments and mission equipment', 'PAY'),
('Propulsion', 'Thrusters and fuel systems', 'PROP'),
('Structure', 'Mechanical structure and deployment mechanisms', 'STRUCT'),
('Software', 'On-board computer systems and algorithms', 'SW');

-- Insert applicability (mission types)
INSERT INTO applicability (app_title, app_description, app_tag) VALUES
('All Missions', 'Applies to all satellite missions', 'ALL'),
('Earth Observation', 'Low Earth orbit observation satellites', 'EO'),
('Communication', 'Geostationary communication satellites', 'COMM'),
('Navigation', 'GPS and navigation satellites', 'NAV'),
('Deep Space', 'Interplanetary and deep space missions', 'DEEP'),
('CubeSat', 'Small satellite missions', 'CUBE');

-- Insert requirements for Space Project
INSERT INTO requirements (req_title, req_description, req_reference, req_category, req_applicability, req_current_status, req_verification, req_author, req_reviewer, req_parent, req_link, req_creation_date, req_update_date, req_deadline_date) VALUES
-- Power System Requirements
('REQ-PWR-001', 'The satellite shall generate minimum 500W of electrical power during daylight operations', 'The solar array system must provide sufficient power to operate all subsystems including payload, communication, and attitude control systems during sunlit portions of the orbit.', 1, 1, 1, 1, 1, 2, 0, 'https://spacecorp.com/power-specs', '2024-01-15', '2024-01-15', '2024-06-30'),
('REQ-PWR-002', 'The battery system shall provide 200W continuous power for 45 minutes during eclipse', 'During orbital eclipse periods, the battery system must maintain critical systems operation without degradation for the maximum expected eclipse duration.', 1, 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-20', '2024-07-15'),
('REQ-PWR-003', 'The power distribution system shall provide redundant power paths to all critical subsystems', 'Each critical subsystem must have at least two independent power paths to ensure system reliability and fault tolerance.', 1, 1, 2, 1, 1, 2, 0, '', '2024-01-15', '2024-01-25', '2024-07-30'),

-- Communication System Requirements
('REQ-COMM-001', 'The satellite shall maintain continuous communication with ground stations during 90% of each orbit', 'The communication system must provide reliable uplink and downlink capabilities with ground stations, accounting for orbital geometry and atmospheric conditions.', 2, 1, 3, 1, 1, 2, 0, '', '2024-01-16', '2024-01-16', '2024-08-15'),
('REQ-COMM-002', 'The downlink data rate shall be minimum 10 Mbps for science data transmission', 'The communication system must support high-speed data transmission to accommodate payload data volume requirements.', 2, 1, 2, 1, 1, 2, 0, '', '2024-01-16', '2024-01-18', '2024-08-30'),
('REQ-COMM-003', 'The system shall support S-band and X-band communication frequencies', 'Dual-frequency communication capability is required for redundancy and different mission phases.', 2, 1, 2, 1, 1, 2, 0, '', '2024-01-16', '2024-01-19', '2024-09-15'),

-- Attitude Control Requirements
('REQ-ACS-001', 'The satellite shall maintain pointing accuracy of ±0.1 degrees in all axes', 'The attitude control system must provide precise pointing for payload operations and communication antenna alignment.', 3, 1, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-17', '2024-06-15'),
('REQ-ACS-002', 'The system shall perform autonomous attitude determination using star trackers', 'Star tracker-based attitude determination must provide continuous, accurate orientation data without ground intervention.', 3, 1, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-19', '2024-06-30'),
('REQ-ACS-003', 'The reaction wheels shall provide momentum storage capacity of 20 Nms', 'Sufficient momentum storage is required for attitude control during various mission phases and disturbance conditions.', 3, 1, 2, 1, 1, 2, 0, '', '2024-01-17', '2024-01-20', '2024-07-15'),

-- Thermal Control Requirements
('REQ-THERM-001', 'All electronic components shall operate within -20°C to +60°C temperature range', 'Thermal control system must maintain component temperatures within specified limits for reliable operation.', 4, 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-18', '2024-07-15'),
('REQ-THERM-002', 'The payload compartment shall maintain temperature stability of ±2°C', 'Payload instruments require stable thermal environment for accurate measurements and calibration.', 4, 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-20', '2024-07-30'),
('REQ-THERM-003', 'The thermal control system shall operate passively during normal operations', 'Passive thermal control using radiators and thermal blankets is preferred to minimize power consumption.', 4, 1, 2, 1, 1, 2, 0, '', '2024-01-18', '2024-01-22', '2024-08-15'),

-- Payload Requirements
('REQ-PAY-001', 'The optical payload shall provide 1-meter ground resolution at 500km altitude', 'Earth observation payload must meet specified resolution requirements for mission objectives.', 5, 2, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-19', '2024-08-15'),
('REQ-PAY-002', 'The payload shall operate in visible and near-infrared spectral bands', 'Multi-spectral imaging capability is required for comprehensive Earth observation data collection.', 5, 2, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-21', '2024-08-30'),
('REQ-PAY-003', 'The payload data storage shall accommodate 24 hours of continuous imaging', 'On-board storage must support extended imaging operations without data loss.', 5, 2, 2, 1, 1, 2, 0, '', '2024-01-19', '2024-01-23', '2024-09-15'),

-- Propulsion Requirements
('REQ-PROP-001', 'The propulsion system shall provide delta-V capability of 500 m/s', 'Sufficient delta-V is required for orbit maintenance, collision avoidance, and end-of-life disposal.', 6, 1, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-20', '2024-09-15'),
('REQ-PROP-002', 'Adequate thrust is required for attitude control during various mission phases.', 'The thrusters shall provide minimum thrust of 1N for attitude control', 6, 1, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-22', '2024-09-30'),
('REQ-PROP-003', 'The propulsion system shall use non-toxic propellants', 'Safety requirements mandate the use of non-toxic propellants for ground handling and launch operations.', 6, 1, 2, 1, 1, 2, 0, '', '2024-01-20', '2024-01-24', '2024-10-15'),

-- Structure Requirements
('REQ-STRUCT-001', 'The satellite structure shall survive launch loads of 15g in all axes', 'Structural design must withstand launch vehicle vibration and acceleration environments.', 7, 1, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-21', '2024-10-15'),
('REQ-STRUCT-002', 'The solar array deployment mechanism shall be 99% reliable', 'Solar array deployment is critical for mission success and must have high reliability.', 7, 1, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-23', '2024-10-30'),
('REQ-STRUCT-003', 'The satellite shall fit within 1.2m x 1.2m x 2.0m launch envelope', 'Physical dimensions must comply with launch vehicle fairing constraints.', 7, 1, 2, 1, 1, 2, 0, '', '2024-01-21', '2024-01-25', '2024-11-15'),

-- Software Requirements
('REQ-SW-001', 'The on-board computer shall execute fault detection and recovery autonomously', 'Autonomous fault management is required for reliable operation without constant ground intervention.', 8, 1, 3, 1, 1, 2, 0, '', '2024-01-22', '2024-01-22', '2024-11-15'),
('REQ-SW-002', 'The software shall support over-the-air updates', 'Software update capability is required for bug fixes and feature enhancements during mission lifetime.', 8, 1, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-24', '2024-11-30'),
('REQ-SW-003', 'The system shall maintain accurate onboard time synchronization', 'Precise timekeeping is required for data correlation and mission operations.', 8, 1, 2, 1, 1, 2, 0, '', '2024-01-22', '2024-01-26', '2024-12-15');

-- Insert tests for Space Project
INSERT INTO tests (test_name, test_description, test_status, test_source) VALUES
-- Power System Tests
('TEST-PWR-001', 'Verify solar array generates 500W under AM0 illumination', 1, 'Solar array testing in thermal vacuum chamber'),
('TEST-PWR-002', 'Verify battery provides 200W for 45 minutes during discharge test', 1, 'Battery cycle testing'),
('TEST-PWR-003', 'Verify redundant power paths function independently', 1, 'Power distribution system testing'),
('TEST-PWR-004', 'Verify power system efficiency under various load conditions', 1, 'End-to-end power system testing'),

-- Communication System Tests
('TEST-COMM-001', 'Verify S-band communication link performance', 1, 'RF testing in anechoic chamber'),
('TEST-COMM-002', 'Verify X-band communication link performance', 1, 'RF testing in anechoic chamber'),
('TEST-COMM-003', 'Verify 10 Mbps data rate capability', 1, 'Data transmission testing'),
('TEST-COMM-004', 'Verify communication during simulated orbital conditions', 1, 'End-to-end communication testing'),

-- Attitude Control Tests
('TEST-ACS-001', 'Verify star tracker pointing accuracy', 1, 'Star tracker calibration testing'),
('TEST-ACS-002', 'Verify reaction wheel momentum capacity', 1, 'Reaction wheel testing'),
('TEST-ACS-003', 'Verify attitude control loop performance', 1, 'Control system testing'),
('TEST-ACS-004', 'Verify autonomous attitude determination', 1, 'System integration testing'),

-- Thermal Control Tests
('TEST-THERM-001', 'Verify thermal control system performance in vacuum', 1, 'Thermal vacuum testing'),
('TEST-THERM-002', 'Verify payload temperature stability', 1, 'Thermal cycling testing'),
('TEST-THERM-003', 'Verify passive thermal control effectiveness', 1, 'Thermal analysis and testing'),
('TEST-THERM-004', 'Verify thermal blankets installation and performance', 1, 'Thermal blanket testing'),

-- Payload Tests
('TEST-PAY-001', 'Verify optical payload resolution performance', 1, 'Optical testing in clean room'),
('TEST-PAY-002', 'Verify multi-spectral imaging capability', 1, 'Spectral calibration testing'),
('TEST-PAY-003', 'Verify payload data storage capacity', 1, 'Data storage testing'),
('TEST-PAY-004', 'Verify payload pointing accuracy', 1, 'Payload alignment testing'),

-- Propulsion Tests
('TEST-PROP-001', 'Verify thruster thrust performance', 1, 'Thruster hot fire testing'),
('TEST-PROP-002', 'Verify delta-V capability', 1, 'Propulsion system testing'),
('TEST-PROP-003', 'Verify propellant compatibility', 1, 'Material compatibility testing'),
('TEST-PROP-004', 'Verify propulsion system safety', 1, 'Safety testing'),

-- Structure Tests
('TEST-STRUCT-001', 'Verify structural integrity under launch loads', 1, 'Vibration testing'),
('TEST-STRUCT-002', 'Verify solar array deployment mechanism', 1, 'Deployment testing'),
('TEST-STRUCT-003', 'Verify satellite fits launch envelope', 1, 'Dimensional verification'),
('TEST-STRUCT-004', 'Verify structural thermal performance', 1, 'Thermal structural testing'),

-- Software Tests
('TEST-SW-001', 'Verify fault detection and recovery algorithms', 1, 'Software testing'),
('TEST-SW-002', 'Verify over-the-air update capability', 1, 'Software update testing'),
('TEST-SW-003', 'Verify time synchronization accuracy', 1, 'Time synchronization testing'),
('TEST-SW-004', 'Verify software integration with hardware', 1, 'System integration testing');

-- Create traceability matrix links
INSERT INTO matrix (matrix_req_id, matrix_test_id) VALUES
-- Power System Requirements -> Tests
(1, 1), (1, 4),          -- REQ-PWR-001 -> Solar array and system efficiency tests
(2, 2), (2, 4),          -- REQ-PWR-002 -> Battery and system efficiency tests
(3, 3), (3, 4),          -- REQ-PWR-003 -> Redundant paths and system efficiency tests

-- Communication System Requirements -> Tests
(4, 1), (4, 2), (4, 4),  -- REQ-COMM-001 -> S-band, X-band, and end-to-end tests
(5, 3), (5, 4),          -- REQ-COMM-002 -> Data rate and end-to-end tests
(6, 1), (6, 2), (6, 4),  -- REQ-COMM-003 -> Dual frequency and end-to-end tests

-- Attitude Control Requirements -> Tests
(7, 1), (7, 3), (7, 4),  -- REQ-ACS-001 -> Star tracker, control loop, and integration tests
(8, 1), (8, 4),          -- REQ-ACS-002 -> Star tracker and integration tests
(9, 2), (9, 3), (9, 4),  -- REQ-ACS-003 -> Reaction wheel, control loop, and integration tests

-- Thermal Control Requirements -> Tests
(10, 1), (10, 3),        -- REQ-THERM-001 -> Thermal vacuum and passive control tests
(11, 2), (11, 4),        -- REQ-THERM-002 -> Payload temperature and blanket tests
(12, 3), (12, 4),        -- REQ-THERM-003 -> Passive control and blanket tests

-- Payload Requirements -> Tests
(13, 1), (13, 4),        -- REQ-PAY-001 -> Resolution and pointing accuracy tests
(14, 2), (14, 4),        -- REQ-PAY-002 -> Multi-spectral and pointing accuracy tests
(15, 3), (15, 4),        -- REQ-PAY-003 -> Data storage and pointing accuracy tests

-- Propulsion Requirements -> Tests
(16, 1), (16, 2),        -- REQ-PROP-001 -> Thrust and delta-V tests
(17, 1), (17, 3),        -- REQ-PROP-002 -> Thrust and compatibility tests
(18, 3), (18, 4),        -- REQ-PROP-003 -> Compatibility and safety tests

-- Structure Requirements -> Tests
(19, 1), (19, 4),        -- REQ-STRUCT-001 -> Vibration and thermal structural tests
(20, 2), (20, 3),        -- REQ-STRUCT-002 -> Deployment and dimensional tests
(21, 3), (21, 4),        -- REQ-STRUCT-003 -> Dimensional and thermal structural tests

-- Software Requirements -> Tests
(22, 1), (22, 4),        -- REQ-SW-001 -> Fault detection and integration tests
(23, 2), (23, 4),        -- REQ-SW-002 -> Update capability and integration tests
(24, 3), (24, 4);        -- REQ-SW-003 -> Time synchronization and integration tests 