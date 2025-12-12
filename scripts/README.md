# ReqMan Scripts

This directory contains utility scripts for managing the ReqMan application database and testing components.

## Scripts Overview

### 1. `setup_database.sh`

**Purpose**: Initialize or reset the ReqMan database with schema and sample data.

**What it does**:
- Detects and uses either `docker compose` (v2) or `docker-compose` (legacy)
- Verifies Docker daemon is running
- Checks that the PostgreSQL database container (`db` service) is running
- Waits for PostgreSQL to be ready to accept connections (up to 30 retries)
- Creates the `reqman` database if it doesn't exist
- Cleans existing tables (drops all ReqMan-related tables)
- Initializes the database with complete schema and sample data from `init_complete.sql`
- Verifies the setup by showing:
  - Created tables
  - Sample users (all with password: 'password')
  - Created projects
  - Project memberships
  - Data counts for each entity type

**Usage**:
```bash
./scripts/setup_database.sh
```

**Prerequisites**:
- Docker must be running
- Database service must be started: `docker compose up -d`
- `init_complete.sql` file must exist in the project root (or set `INIT_SQL` environment variable)

#### 👥 Pre-configured Users

All users have password: `password`

| Username | Name | Role | Project |
|----------|------|------|---------|
| `alice` | Alice Johnson | Admin | ReqMan Project |
| `dr_smith` | Dr. Sarah Smith | Admin | Space Project |
| `eng_jones` | Engineer Mike Jones | User | Space Project |
| `tech_lee` | Technician Lisa Lee | User | Space Project |
| `qa_wilson` | QA Specialist Tom Wilson | User | Space Project |
| `admin` | System Administrator | Admin | ReqMan Project |


---

### 2. `clear_database.sh`

**Purpose**: Completely remove the ReqMan database and clean up all associated objects.

**What it does**:
- Detects and uses either `docker compose` (v2) or `docker-compose` (legacy)
- Verifies Docker daemon is running
- Checks that the PostgreSQL database container (`db` service) is running
- Drops the `reqman` database forcefully (terminates all connections)
- Cleans up any leftover objects owned by the `rust` user
- Verifies that the database has been successfully removed

**Usage**:
```bash
./scripts/clear_database.sh
```

**Prerequisites**:
- Docker must be running
- Database service must be started: `docker compose up -d`

**Warning**: This script permanently deletes all data in the `reqman` database. Use with caution!

---

### 3. `test_excel_parser.sh`

**Purpose**: Test the Excel parser component by downloading exports from ReqMan and parsing them.

**What it does**:
- Checks if the ReqMan server is running on `http://127.0.0.1:8000`
- Creates a `test_exports/` directory
- Downloads Excel exports from the running server:
  - `requirements.xls` - All requirements
  - `tests.xls` - All tests
  - `matrix.xls` - Requirements-tests matrix
- Tests parsing of requirements and tests files (dry run mode)
- Generates JSON files from the Excel exports:
  - `requirements.json`
  - `tests.json`
- Lists all generated files

**Usage**:
```bash
./scripts/test_excel_parser.sh
```

**Prerequisites**:
- ReqMan server must be running: `cargo run --bin req_man`
- Excel parser must be built: `cd excel_parser && cargo build --release`

**Output**: Generated files are saved in the `test_exports/` directory.

---

## Common Workflow

### Initial Setup
1. Start Docker services: `docker compose up -d`
2. Initialize the database: `./scripts/setup_database.sh`
3. Start the application: `cargo run --bin req_man`

### Reset Database
1. Clear existing database: `./scripts/clear_database.sh`
2. Reinitialize: `./scripts/setup_database.sh`

### Test Excel Parser
1. Ensure ReqMan is running with data
2. Run the test script: `./scripts/test_excel_parser.sh`
3. Check generated files in `test_exports/`

---

## Error Handling

All scripts include:
- Exit on error with `set -euo pipefail`
- Clear error messages with ❌ indicators
- Success confirmations with ✅ indicators
- Prerequisite checks before executing main logic

## Environment Variables

### `setup_database.sh`
- `INIT_SQL`: Path to the SQL initialization file (default: `init_complete.sql`)

## Notes

- All scripts detect Docker Compose version automatically (v2 or legacy)
- Scripts use color-coded emoji indicators for better readability
- Database scripts require the `db` service to be running before execution
- The test script requires a running ReqMan server instance
