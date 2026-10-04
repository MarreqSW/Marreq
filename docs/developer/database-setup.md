# Marreq Database Setup Guide

For the full binary-level setup flows (**`marreq-server`** / **`marreq-cloud`**, Docker / local), start with the [setup guide](setup.md). This page stays focused on database initialization, seeding, migration, reset, and verification details.

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

From the SPA (self-hosted, administrators): **Admin → Backup** → **Download backup** downloads `marreq-backup_<timestamp>.tar.gz` (issue #341). The server runs `pg_dump` against `DATABASE_URL` (the Docker image ships `postgresql-client`; set `MARREQ_PG_DUMP` to use another binary) and packs the dump with the attachment files:

```text
marreq-backup_<timestamp>/database.sql          plain SQL from pg_dump
marreq-backup_<timestamp>/attachments/ab/cd/…   the attachment store (MARREQ_ATTACHMENTS_DIR)
marreq-backup_<timestamp>/manifest.json         format, time, file count and size
```

Untick **Include attachment files** for a database-only archive
(`POST /api/admin/backup?attachments=false`). The files are copied right after
the dump: a file uploaded meanwhile may be extra (harmless), one deleted
meanwhile is left out. While the archive is built the server needs free space
in its temp directory for about the dump plus the compressed archive.

From a shell (database only; back up the attachments volume as shown below):
```bash
./marreq-core/scripts/db_backup.sh
# Saves to ./backups/marreq_<timestamp>.sql.gz
```

### Restore Backup
Restore into an **empty** database (the backend runs migrations on start, so
restore before starting it, or into a freshly created database).

An **Admin → Backup** archive:
```bash
tar xzf marreq-backup_<timestamp>.tar.gz
docker compose -f docker/docker-compose.yml exec -T db \
   psql -U rust -d marreq -v ON_ERROR_STOP=1 < marreq-backup_<timestamp>/database.sql
```
then put `marreq-backup_<timestamp>/attachments/` back into the attachments
volume ([Attachment files](#attachment-files) below).

A `db_backup.sh` dump:
```bash
gunzip -c backups/marreq_<timestamp>.sql.gz | \
   docker compose -f docker/docker-compose.yml exec -T db \
   psql -U rust -d marreq -v ON_ERROR_STOP=1
```
Use `psql` from PostgreSQL 17 or newer: dumps written by `pg_dump` 17 (the
Docker image and **Admin → Backup**) can contain commands older clients reject.

### Attachment files

An **Admin → Backup** archive contains the attachment files unless they were
left out; a `db_backup.sh` dump never does (issue #241). The server stores them under `MARREQ_ATTACHMENTS_DIR` (`/var/lib/marreq/attachments` in
the Docker image), which the compose files mount from the `marreq_attachments`
volume (`marreq_attachments_cloud` for `marreq-cloud`). Compose prefixes volume
names with the project name, so with the default project it is
`docker_marreq_attachments`; check with `docker volume ls | grep attachments`.

Without an Admin → Backup archive, back up the volume right after the database
dump so both describe the same moment:

```bash
docker run --rm \
  -v docker_marreq_attachments:/data:ro \
  -v "$PWD/backups":/backup \
  alpine tar czf /backup/attachments_<timestamp>.tar.gz -C /data .
```

Restore it into the (empty) volume before starting the backend:

```bash
docker run --rm \
  -v docker_marreq_attachments:/data \
  -v "$PWD/backups":/backup \
  alpine sh -c 'tar xzf /backup/attachments_<timestamp>.tar.gz -C /data && chown -R 10001:10001 /data'
```

From an unpacked Admin → Backup archive:

```bash
docker run --rm \
  -v docker_marreq_attachments:/data \
  -v "$PWD/marreq-backup_<timestamp>/attachments":/backup:ro \
  alpine sh -c 'cp -a /backup/. /data/ && chown -R 10001:10001 /data'
```

Files are named by their SHA-256 (`ab/cd/<hash>`), so a restored directory
can be checked with `sha256sum`. Files that no database row references are
harmless. A row whose file is missing gives a 404 on download.

## Login rate-limit table

`login_rate_limits` (migration `2026-10-06-000001_login_rate_limits`) holds
the shared failed-login counters and lockouts (issue #286; see *Login rate
limiting* in [authentication.md](authentication.md)). It needs no maintenance:
rows are deleted on a successful login and swept after
`MARREQ_RATE_LIMIT_RETENTION_HOURS`. To lift every lockout at once, for example
after a misconfigured client locked out an account:

```sql
DELETE FROM login_rate_limits;                                   -- everything
DELETE FROM login_rate_limits WHERE scope_key = 'alice';         -- one username (canonical, lower case)
```

## Upgrading from PostgreSQL 15

The Compose stack uses PostgreSQL 17 (`pgvector/pgvector:pg17-trixie`) on a new
volume, `pgdata17`. PostgreSQL cannot open a data directory from an older major
version, so an existing installation moves its data with a dump and restore. The
old PostgreSQL 15 volume (`pgdata`) is not touched, which keeps rollback simple.

1. **Before updating**, with the old stack still running, take a backup:
   **Admin → Backup → Download backup** (a `.tar.gz`; the files can be left
   out, the attachments volume is not affected by this upgrade), or
   ```bash
   ./marreq-core/scripts/db_backup.sh    # -> backups/marreq_<timestamp>.sql.gz
   ```
   Note a few numbers to compare later (for example the requirement count on
   the dashboard of each project).
2. Stop the stack and update to this release (`git pull`, or pull the new images):
   ```bash
   docker compose -f docker/docker-compose.yml down
   ```
3. Start **only** the new database. This creates the empty `pgdata17` volume:
   ```bash
   docker compose -f docker/docker-compose.yml up -d db
   docker compose -f docker/docker-compose.yml exec db pg_isready -U rust -d marreq
   ```
4. Restore the backup **before** the backend starts (it would otherwise create
   the tables first and the restore would fail):
   ```bash
   # Admin → Backup archive:
   tar xzf marreq-backup_<timestamp>.tar.gz
   docker compose -f docker/docker-compose.yml exec -T db \
      psql -U rust -d marreq -v ON_ERROR_STOP=1 < marreq-backup_<timestamp>/database.sql
   # or a db_backup.sh dump:
   gunzip -c backups/marreq_<timestamp>.sql.gz | \
      docker compose -f docker/docker-compose.yml exec -T db \
      psql -U rust -d marreq -v ON_ERROR_STOP=1
   ```
5. Start the rest of the stack and check the data (log in, compare the numbers
   from step 1):
   ```bash
   docker compose -f docker/docker-compose.yml up -d
   ```

Use the same `-f` files you normally run with (for example add
`-f docker/docker-compose.prod.yml`, or use `docker-compose.light.yml`).

**Rollback:** check out (or pull) the previous release and start it again. It
still uses the untouched PostgreSQL 15 volume.

**Clean-up:** once the upgrade is confirmed, remove the old volume. Its name is
prefixed with the Compose project name; `docker volume ls` shows it (for
example `docker_pgdata`):
```bash
docker volume rm docker_pgdata
```

**Managed PostgreSQL** (not Compose): upgrade the server with your provider's
major-version upgrade (or dump and restore as above), and run the backend image
from this release so its `pg_dump` (17) matches the server.
