# ReqMan Database Setup Guide

## Overview

This guide provides complete instructions for setting up the ReqMan database with all necessary tables, constraints, indexes, and sample data including users with working passwords.

## Quick Start

### Option 1: Automated Setup (Recommended)

1. **Ensure Docker is running**
   ```bash
   docker --version
   ```

2. **Start the database container**
   ```bash
   docker-compose up -d
   ```

3. **Run the automated setup script**
   ```bash
   ./scripts/setup_database.sh
   ```

4. **Start the application**
   ```bash
   cargo run --bin req_man
   ```

5. **Access the application**
   - URL: http://localhost:8000
   - Login with any user using password: `password`

### Option 2: Manual Setup

1. **Create the database**
   ```bash
   docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"
   ```

2. **Run the initialization script**
   ```bash
   docker exec -i reqman_db_1 psql -U rust -d reqman < scripts/init_complete.sql
   ```

## Database Schema

### Core Tables

| Table | Description |
|-------|-------------|
| `projects` | Multi-project support with project metadata |
| `users` | User accounts with authentication and permissions |
| `requirements` | Core requirement data with metadata |
| `tests` | Test cases with status and source information |
| `matrix` | Traceability links between requirements and tests |
| `categories` | User-defined requirement categories |
| `applicability` | Requirement applicability definitions |
| `verification` | Verification methods for requirements |
| `status` | Status definitions for requirements and tests |
| `logs` | Audit trail for all system activities |

### Key Features

- **Multi-project support**: All entities are project-scoped
- **Hierarchical requirements**: Support for parent-child relationships
- **Traceability matrix**: Link requirements to tests
- **Audit logging**: Complete activity tracking
- **User authentication**: Secure password-based login
- **Role-based access**: Admin and regular user roles

## Sample Data

### Projects
- **Space Project**: Space exploration satellite requirements
- **ReqMan Project**: Requirements management system development
- **Empty Project**: Testing and demonstration project

### Users (All with password: `password`)
| Username | Name | Role | Project |
|----------|------|------|---------|
| `alice` | Alice Johnson | Admin | ReqMan Project |
| `dr_smith` | Dr. Sarah Smith | Admin | Space Project |
| `eng_jones` | Engineer Mike Jones | User | Space Project |
| `tech_lee` | Technician Lisa Lee | User | Space Project |
| `qa_wilson` | QA Specialist Tom Wilson | User | Space Project |
| `admin` | System Administrator | Admin | ReqMan Project |

### Space Project Sample Data
- **8 Categories**: Power, Communication, Attitude Control, Thermal, etc.
- **6 Applicability**: All Missions, Earth Observation, Communication, etc.
- **4 Verification Methods**: Inspection, Analysis, Demonstration, Test
- **5 Requirements**: Power, communication, and thermal requirements
- **5 Tests**: Corresponding test cases for each requirement
- **5 Matrix Links**: Complete traceability mapping

## Database Files

### `scripts/init_complete.sql`
Complete database initialization script containing:
- All table definitions
- Foreign key constraints
- Performance indexes
- Semantic search schema (pgvector + full-text search)
- Sample data
- Working user passwords

### `scripts/setup_database.sh`
Automated setup script that:
- Checks Docker status
- Creates database
- Cleans existing data
- Runs initialization
- Verifies setup

## Verification

After setup, verify the database is working:

```bash
# Check tables
docker exec reqman_db_1 psql -U rust -d reqman -c "\dt"

# Check users
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT username, name, is_admin FROM users;"

# Check sample data
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT COUNT(*) as requirements FROM requirements;"
```

## Troubleshooting

### Common Issues

1. **Docker not running**
   ```bash
   docker info
   # Start Docker if needed
   ```

2. **Database container not running**
   ```bash
   docker-compose up -d
   ```

3. **Port already in use**
   ```bash
   lsof -i :8000
   kill <PID>
   ```

4. **Database connection issues**
   ```bash
   docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT 1;"
   ```

5. **"relation already exists" when running `diesel migration run`**
   This happens when the database was created with `scripts/init_complete.sql` (or `setup_database.sh`) but Diesel’s migration history table is missing those runs. Mark the baseline and seed migrations as applied, then run migrations:
   ```bash
   # With Docker (replace reqman_db_1 if your container name differs):
   docker exec -i reqman_db_1 psql -U rust -d reqman < scripts/backfill_diesel_migrations.sql
   diesel migration run
   ```
   If you connect with `DATABASE_URL` (e.g. local PostgreSQL), run the backfill against that database first:
   ```bash
   psql "$DATABASE_URL" -f scripts/backfill_diesel_migrations.sql
   diesel migration run
   ```

### Reset Database

To completely reset the database:

```bash
# Drop and recreate database
docker exec reqman_db_1 psql -U rust -d postgres -c "DROP DATABASE IF EXISTS reqman;"
docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"

# Re-run initialization
./scripts/setup_database.sh
```

## Security Notes

- All users have the same password (`password`) for demonstration
- In production, use strong, unique passwords
- Consider implementing password policies
- Review and adjust user permissions as needed

## Performance

The database includes optimized indexes for:
- User authentication
- Project-based queries
- Requirement filtering
- Test status queries
- Audit log searches

## Backup and Restore

### Create Backup
```bash
docker exec reqman_db_1 pg_dump -U rust reqman > backup.sql
```

### Restore Backup
```bash
docker exec -i reqman_db_1 psql -U rust -d reqman < backup.sql
```

## Support

For issues or questions:
1. Check the troubleshooting section
2. Review application logs
3. Verify database connectivity
4. Check Docker container status

---

**Note**: This setup provides a complete, working ReqMan database with sample data. The application is ready to use immediately after setup.
