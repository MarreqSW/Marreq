# Marreq Database Setup Guide

## Overview

Marreq now uses a single database strategy:
- **Schema creation/evolution**: `migrations/*/up.sql` (Diesel migrations)
- **Sample/demo data**: `scripts/init_complete.sql` (seed data only)

`scripts/init_complete.sql` does **not** create tables, indexes, triggers, or extensions.

## Quick Start

### Option 1: Automated Setup (Recommended)

1. Ensure Docker is running:
   ```bash
   docker --version
   ```
2. Install the Diesel CLI (once per machine):
   ```bash
   cargo install diesel_cli --no-default-features --features postgres
   ```
3. Create a `.env` file in the project root:
   ```
   DATABASE_URL=postgres://rust:rust@127.0.0.1:5432/marreq
   ```
4. Run the setup script:
   ```bash
   ./scripts/db_setup.sh
   ```
5. (Optional) Load demo/test data:
   ```bash
   ./scripts/db_seed.sh
   ```
6. Start the application:
   ```bash
   cargo run --bin marreq
   ```
7. Access the app at `http://localhost:8000` and log in with seeded users (password: `ChangeMe123!`).

### Option 2: Manual Setup

1. Create the database:
   ```bash
   docker compose exec -T db psql -U rust -d postgres -c "CREATE DATABASE marreq;"
   ```
2. Apply migrations (schema):
   ```bash
   diesel migration run
   ```
3. Seed sample data:
   ```bash
   docker compose exec -T db psql -U rust -d marreq < scripts/init_complete.sql
   ```

> For the full scripts reference, see [scripts/README.md](scripts/README.md).

## Database Schema

### Core Tables

| Table | Description |
|-------|-------------|
| `projects` | Multi-project support with project metadata |
| `users` | User accounts with authentication and permissions |
| `requirements` | Requirement containers with immutable versions in `requirement_versions` |
| `tests` | Test cases with status and source information |
| `matrix` | Traceability links between requirements and tests |
| `categories` | User-defined requirement categories |
| `applicability` | Requirement applicability definitions |
| `verification` | Verification methods for requirements |
| `requirement_status` | Requirement status definitions |
| `test_status` | Test status definitions |
| `logs` | Audit trail for system activities |

### Key Features

- Multi-project scoping
- Immutable requirement versions
- Traceability matrix
- Immutable baselines
- Semantic-search tables (`pgvector` + queue)
- User authentication and role-aware access

## Sample Data

### Projects
- **Space Project**: Space exploration satellite requirements
- **Marreq Project**: Requirements management system development
- **Empty Project**: Empty sandbox project

### Users (all seeded users use `ChangeMe123!`)

| Username | Name | Role | Project |
|----------|------|------|---------|
| `alice` | Alice Johnson | Admin | Marreq Project |
| `dr_smith` | Dr. Sarah Smith | Admin | Space Project |
| `eng_jones` | Engineer Mike Jones | User | Space Project |
| `tech_lee` | Technician Lisa Lee | User | Space Project |
| `qa_wilson` | QA Specialist Tom Wilson | User | Space Project |
| `admin` | System Administrator | Admin | Marreq Project |

### Space Project Sample Data
- 8 categories
- 6 applicability definitions
- 4 verification methods
- 5 requirements (with initial versions)
- 5 tests
- 5 matrix links

## Database Files

### `migrations/*/up.sql`
Schema source of truth:
- Tables and constraints
- Indexes
- Triggers/functions
- Extensions (including `vector`)

### `scripts/init_complete.sql`
Seed data only:
- Sample projects/users
- Statuses/categories/applicability/verification
- Sample requirements/tests/matrix links
- Sample logs and custom-field data

### `scripts/db_setup.sh`
Fresh install script that:
- Starts the Docker `db` service if not running
- Creates the `marreq` database if absent
- Runs `diesel migration run` (schema source of truth)

Use `--seed` flag to also load demo data in one step.

### `scripts/db_seed.sh`
Loads demo/test data from `init_complete.sql`. **Not for production.**

### `scripts/db_migrate.sh`
Wrapper for `diesel migration run / revert / list`.  Use after pulling updates:
```bash
./scripts/db_migrate.sh up     # apply pending migrations
./scripts/db_migrate.sh list   # check status
```

### `scripts/db_backup.sh`
Runs `pg_dump` and saves a compressed archive to `./backups/`.

### `scripts/db_reset.sh`
Drops the `marreq` database entirely.  Development use only.

## Verification

```bash
# Check tables
docker compose exec -T db psql -U rust -d marreq -c "\dt"

# Check users
docker compose exec -T db psql -U rust -d marreq -c "SELECT username, name, is_admin FROM users ORDER BY id;"

# Check sample data
docker compose exec -T db psql -U rust -d marreq -c "SELECT COUNT(*) AS requirements FROM requirements;"
```

## Troubleshooting

### Common Issues

1. Docker not running:
   ```bash
   docker info
   ```
2. DB container not running:
   ```bash
   docker compose up -d db
   ```
3. `diesel: command not found`:
   - Install the Diesel CLI:
     ```bash
     cargo install diesel_cli --no-default-features --features postgres
     ```
4. Seed script fails with `projects table is not empty`:
   - `scripts/init_complete.sql` is seed-only for fresh DBs.
   - Reset with:
     ```bash
     ./scripts/db_reset.sh
     ./scripts/db_setup.sh --seed
     ```

## Reset Database

```bash
./scripts/db_reset.sh
./scripts/db_setup.sh --seed
```

## Security Notes

- Seeded users share the demo password `ChangeMe123!`.
- Change passwords outside local/demo environments.

## Backup and Restore

### Create Backup
```bash
./scripts/db_backup.sh
# Saves to ./backups/marreq_<timestamp>.sql.gz
```

### Restore Backup
```bash
gunzip -c backups/marreq_<timestamp>.sql.gz | \
  docker compose exec -T db psql -U rust -d marreq
```
