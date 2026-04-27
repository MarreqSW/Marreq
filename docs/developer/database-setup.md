# Marreq Database Setup Guide

## Overview

Marreq now uses a single database strategy:
- **Schema creation/evolution**: `marreq-core/migrations/*/up.sql` (Diesel migrations)
- **Sample/demo data**: `marreq-core/scripts/init_complete.sql` (seed data only)

`marreq-core/scripts/init_complete.sql` does **not** create tables, indexes, triggers, or extensions.

## Environment variables

All configuration is driven by environment variables. The application loads them
from a `.env` file in the project root (via `dotenvy`) if the file exists.

**`.env` is gitignored and must never be committed.** Copy the template to get
started:

```bash
cp .env.example .env
```

### Key variables

| Variable | Required | Default in `.env.example` | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | `postgres://rust:rust@127.0.0.1:5433/marreq` | PostgreSQL connection string (host port from `docker-compose.yml`) |
| `ROCKET_SECRET_KEY` | Production | _(auto-generated in dev)_ | 256-bit base64 key for cookie signing. Generate with `openssl rand -base64 32`. |
| `EMBEDDINGS_ENABLED` | No | `false` | Enable pgvector semantic search |
| `EMBEDDING_PROVIDER` | No | `ollama` | `ollama` or `openai` |
| `EMBEDDING_MODEL` | No | `nomic-embed-text` | Embedding model name |
| `OLLAMA_URL` | No | `http://localhost:11434` | Ollama API base URL |
| `RAG_ENABLED` | No | `false` | Enable LLM-assisted search |
| `RAG_MODEL` | No | `llama3.2` | LLM model for RAG |

> In **development**, `ROCKET_SECRET_KEY` can be omitted — Rocket auto-generates
> an ephemeral key (sessions expire on restart, which is fine locally). In
> **production**, always set a stable key so sessions survive restarts.



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
3. Create a `.env` file in the project root from the provided template:
   ```bash
   cp .env.example .env
   ```
   The default values work for a local Docker setup. Edit the file if your
   database URL or optional services (Ollama, embeddings) differ.
   **`.env` is gitignored — never commit it.**
4. Run the setup script:
   ```bash
   ./marreq-core/scripts/db_setup.sh
   ```
5. (Optional) Load demo/test data:
   ```bash
   ./marreq-core/scripts/db_seed.sh
   ```
6. Start the application:
   ```bash
   cargo run -p marreq-server
   ```
7. Access the app at `http://localhost:8000` and log in with seeded users (password: `ChangeMe123!`).

### Option 2: Manual Setup

1. Create the database:
   ```bash
   docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d postgres -c "CREATE DATABASE marreq;"
   ```
2. Apply migrations (schema):
   ```bash
   diesel migration run
   ```
3. Seed sample data:
   ```bash
   docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq < marreq-core/scripts/init_complete.sql
   ```

> For the full scripts reference, see [marreq-core/scripts/README.md](../../marreq-core/scripts/README.md).

## Database Schema

### Core Tables

| Table | Description |
|-------|-------------|
| `projects` | Multi-project support with project metadata |
| `users` | User accounts with authentication and permissions |
| `project_members` | Membership and role per project |
| `project_reviewers` | Users allowed to change requirement/verification **status** and version **approval** for that project |
| `requirements` | Requirement containers with immutable versions in `requirement_versions` |
| `verifications` | Verification items (name, reference, status, author/reviewer, optional method) |
| `matrix` | Traceability links between requirements and verifications |
| `categories` | User-defined requirement categories |
| `applicability` | Requirement applicability definitions |
| `verification_methods` | Methods attachable to requirement versions and optionally to verifications |
| `verification_status` | Per-project statuses for verifications |
| `requirement_status` | Requirement status definitions |
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
- 5 verifications
- 5 matrix links

Seeds also populate **`project_reviewers`** (from Admin/Reviewer roles) so approval and status gates work for demo users; adjust via **Project settings → Reviewers** or `PUT /api/projects/<id>/reviewers`.

## Database Files

### `migrations/*/up.sql`
Schema source of truth:
- Tables and constraints
- Indexes
- Triggers/functions
- Extensions (including `vector`)

### `marreq-core/scripts/init_complete.sql`
Seed data only:
- Sample projects/users
- Statuses/categories/applicability/verification
- Sample requirements/tests/matrix links
- Sample logs and custom-field data

### `marreq-core/scripts/db_setup.sh`
Fresh install script that:
- Starts the Docker `db` service if not running
- Creates the `marreq` database if absent
- Runs `diesel migration run` (schema source of truth)

Use `--seed` flag to also load demo data in one step.

### `marreq-core/scripts/db_seed.sh`
Loads demo/test data from `init_complete.sql`. **Not for production.**

### `marreq-core/scripts/db_migrate.sh`
Wrapper for `diesel migration run / revert / list`.  Use after pulling updates:
```bash
./marreq-core/scripts/db_migrate.sh up     # apply pending migrations
./marreq-core/scripts/db_migrate.sh list   # check status
```

### `marreq-core/scripts/db_backup.sh`
Runs `pg_dump` and saves a compressed archive to `./backups/`.

### `marreq-core/scripts/db_reset.sh`
Drops the `marreq` database entirely.  Development use only.

## Verification

```bash
# Check tables
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "\dt"

# Check users
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT username, name, is_admin FROM users ORDER BY id;"

# Check sample data
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT COUNT(*) AS requirements FROM requirements;"
```

## Troubleshooting

### Common Issues

1. Docker not running:
   ```bash
   docker info
   ```
2. DB container not running:
   ```bash
   docker compose -f docker/docker-compose.yml up -d db
   ```
3. `diesel: command not found`:
   - Install the Diesel CLI:
     ```bash
     cargo install diesel_cli --no-default-features --features postgres
     ```
4. Seed script fails with `projects table is not empty`:
   - `marreq-core/scripts/init_complete.sql` is seed-only for fresh DBs.
   - Reset with:
     ```bash
     ./marreq-core/scripts/db_reset.sh
     ./marreq-core/scripts/db_setup.sh --seed
     ```

## Reset Database

```bash
./marreq-core/scripts/db_reset.sh
./marreq-core/scripts/db_setup.sh --seed
```

## Security Notes

- Seeded users share the demo password `ChangeMe123!`.
- Change passwords outside local/demo environments.

## Backup and Restore

### Create Backup
```bash
./marreq-core/scripts/db_backup.sh
# Saves to ./backups/marreq_<timestamp>.sql.gz
```

### Restore Backup
```bash
gunzip -c backups/marreq_<timestamp>.sql.gz | \
   docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq
```
