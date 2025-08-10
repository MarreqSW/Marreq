# ReqMan Database Setup Guide

## Project Overview

**ReqMan** is a comprehensive web-based requirements and test management system built with Rust, Rocket, and PostgreSQL. It provides a complete solution for managing hierarchical requirements, tests, traceability matrices, and generating reports.

## Database Architecture

The system uses a multi-project architecture with the following key features:

### Core Entities
- **Projects**: Multi-project support with project metadata
- **Requirements**: Core requirement data with metadata and project association
- **Tests**: Test cases with status and source information, project association
- **Matrix**: Traceability links between requirements and tests, project-scoped
- **Categories**: User-defined requirement categories, project-specific
- **Applicability**: User-defined applicability options, project-specific
- **Users**: System users (authors, reviewers) with authentication
- **Status**: Requirement and test status definitions
- **Verification**: Verification method definitions
- **Logs**: Audit trail for all operations

### Key Features
- **Multi-Project Support**: Complete data isolation between projects
- **Hierarchical Requirements**: Parent-child relationships for complex requirement decomposition
- **Traceability Matrix**: Visual mapping between requirements and tests
- **User Management**: Role-based access control with authentication
- **Audit Logging**: Complete audit trail for compliance and debugging
- **Performance Optimized**: Indexed queries for fast data retrieval

## Consolidated Database Setup

### What's Included

The `init_consolidated.sql` file contains:

1. **Complete Database Schema**: All tables with proper relationships and constraints
2. **Diesel Helper Functions**: Automatic timestamp management functions
3. **Performance Indexes**: Optimized database indexes for fast queries
4. **Space Project Example Data**: Comprehensive example dataset including:
   - 3 projects (Space, ReqMan, Empty)
   - 5 users with different roles and permissions
   - 8 categories for space systems (Power, Communication, Attitude Control, etc.)
   - 6 applicability options for mission types (All Missions, Earth Observation, etc.)
   - 4 verification methods (Inspection, Analysis, Demonstration, Test)
   - 24 requirements across 8 system categories
   - 32 tests covering all requirement areas
   - Complete traceability matrix linking requirements to tests
   - Sample audit log entries

### Quick Setup

1. **Start PostgreSQL** (using Docker):
   ```bash
   docker-compose up -d
   ```

2. **Initialize Database**:
   ```bash
   # Connect to PostgreSQL
   psql -h localhost -U rust -d reqman -f init_consolidated.sql
   ```

3. **Start the Application**:
   ```bash
   cargo run
   ```

4. **Access the Application**:
   - Web Interface: http://localhost:8000
   - API Base URL: http://localhost:8000/api/v1

### Database Connection Details

From `Rocket.toml`:
```toml
[default.databases.my_db]
url = "postgres://rust:rust@127.0.0.1:5432/reqman"
```

## Space Project Example Data

The consolidated script includes a comprehensive Space Project dataset that demonstrates:

### System Categories
1. **Power System** (PWR): Solar panels, batteries, power distribution
2. **Communication** (COMM): Antennas, transponders, data links
3. **Attitude Control** (ACS): Gyroscopes, reaction wheels, star trackers
4. **Thermal Control** (THERM): Heat pipes, radiators, thermal blankets
5. **Payload** (PAY): Scientific instruments and mission equipment
6. **Propulsion** (PROP): Thrusters and fuel systems
7. **Structure** (STRUCT): Mechanical structure and deployment mechanisms
8. **Software** (SW): On-board computer systems and algorithms

### Mission Applicability
- **All Missions**: Applies to all satellite missions
- **Earth Observation**: Low Earth orbit observation satellites
- **Communication**: Geostationary communication satellites
- **Navigation**: GPS and navigation satellites
- **Deep Space**: Interplanetary and deep space missions
- **CubeSat**: Small satellite missions

### Sample Requirements
- Power generation requirements (500W solar, 200W battery)
- Communication requirements (90% coverage, 10 Mbps data rate)
- Attitude control requirements (±0.1° pointing accuracy)
- Thermal control requirements (-20°C to +60°C operating range)
- Payload requirements (1-meter resolution, multi-spectral imaging)
- Propulsion requirements (500 m/s delta-V, non-toxic propellants)
- Structural requirements (15g launch loads, 99% deployment reliability)
- Software requirements (autonomous fault detection, OTA updates)

### Sample Tests
- Power system testing (solar array, battery, distribution)
- Communication testing (S-band, X-band, data rate)
- Attitude control testing (star tracker, reaction wheels, control loops)
- Thermal testing (vacuum, cycling, passive control)
- Payload testing (resolution, spectral, storage, pointing)
- Propulsion testing (thrust, delta-V, compatibility, safety)
- Structural testing (vibration, deployment, dimensional, thermal)
- Software testing (fault detection, updates, time sync, integration)

## Migration History

The consolidated script merges the following diesel migrations:

1. `00000000000000_diesel_initial_setup` - Diesel helper functions
2. `2022-11-07-135533_requirements` - Core tables (requirements, users, status, categories, verification, tests, matrix)
3. `2025-08-03-090750_add_applicability` - Applicability table and requirements relationship
4. `2025-08-03-093841_add_justification_to_requirements` - Justification field for requirements
5. `2025-08-03-102340_add_password_to_users` - Password authentication for users
6. `2025-08-06-094732_add_admin_to_users` - Admin role for users
7. `2025-08-06-154739_create_projects_table` - Multi-project support
8. `2025-08-06-160126_add_project_id_to_verification` - Project-scoped verification methods
9. `2025-08-06-190646_create_logs_table` - Audit logging system
10. `2025-08-06-195902_fix_logs_jsonb_to_text` - Fix for logs data type

## Usage Examples

### Web Interface
- **Requirements**: `/requirements` - View and manage requirements
- **Tests**: `/tests` - Manage test cases
- **Matrix**: `/matrix` - View traceability matrix
- **Categories**: `/categories` - Manage requirement categories
- **Applicability**: `/applicability` - Manage applicability options
- **Projects**: `/projects` - Manage projects

### API Endpoints
- **Requirements**: `GET/POST/DELETE /api/v1/requirements`
- **Tests**: `GET/POST/DELETE /api/v1/tests`
- **Matrix**: `GET /api/v1/matrix`
- **Categories**: `GET/POST/PUT/DELETE /api/v1/categories`
- **Applicability**: `GET/POST/PUT/DELETE /api/v1/applicability`
- **Users**: `GET /api/v1/users`
- **Status**: `GET/POST /api/v1/status`

### Export Features
- **Requirements Export**: Excel export with all metadata
- **Matrix Export**: Traceability matrix in Excel format
- **File Format**: `.xls` files with comprehensive data

## Development Notes

### Database Optimization
- All foreign key relationships are properly indexed
- Query performance optimized with strategic indexes
- Audit logging with minimal performance impact
- Project-scoped queries for data isolation

### Security Features
- Password hashing with bcrypt
- Role-based access control
- Admin user management
- Audit trail for compliance

### Multi-Project Architecture
- Complete data isolation between projects
- Project-specific categories and applicability
- User association with projects
- Cross-project audit logging

## Troubleshooting

### Common Issues
1. **Connection Refused**: Ensure PostgreSQL is running with `docker-compose up -d`
2. **Authentication Failed**: Default password is 'password123' for all users
3. **Migration Errors**: Use the consolidated script instead of individual migrations
4. **Performance Issues**: Check that indexes are created properly

### Reset Database
To completely reset the database:
```bash
# Drop and recreate database
dropdb -h localhost -U rust reqman
createdb -h localhost -U rust reqman

# Run consolidated initialization
psql -h localhost -U rust -d reqman -f init_consolidated.sql
```

## Next Steps

1. **Customize Data**: Modify the example data to match your project needs
2. **Add Users**: Create additional users with appropriate roles
3. **Configure Categories**: Define project-specific categories
4. **Set Applicability**: Configure applicability options for your domain
5. **Import Data**: Use the Excel parser to import existing requirements and tests

The consolidated database is now ready for production use with the ReqMan application!
