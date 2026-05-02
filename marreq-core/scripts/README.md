# Marreq Database Scripts

This directory contains all scripts needed to set up, migrate, seed, and
maintain the Marreq database.  **Diesel is the single authority for schema
management.**  No script in this directory applies DDL directly, all schema
changes go through `marreq-core/migrations/` (run `diesel` from `marreq-core/`).

---

## Prerequisites

| Requirement | Install command |
|---|---|
| Docker with Compose | https://docs.docker.com/compose/install/ |
| `diesel` CLI | `cargo install diesel_cli --no-default-features --features postgres` |
| `.env` file with `DATABASE_URL` | `cp .env.example .env` (see `.env.example` in the project root) |

> **`.env` is gitignored.** Copy `.env.example` to `.env` and adjust values for
> your local setup. Never commit `.env` directly.

---

## Scripts at a Glance

| Script | Purpose |
|---|---|
| `db_setup.sh` | **Fresh install**, create DB + apply all migrations |
| `db_seed.sh` | Load demo/test data (dev/testing only) |
| `db_migrate.sh` | Apply or revert migrations after a version update |
| `db_backup.sh` | Dump the database to a compressed archive |
| `db_reset.sh` | Drop the database entirely (dev resets only) |
| `lazy_setup.sh` | One-click developer bootstrap (full install) |
| `reindex_project.sh` | Trigger semantic-search re-indexing via API |
| `run_checks.sh` | Dedicated checks runner (fmt, clippy, stylelint, purgecss, npm ci, npm test) |
| `run_tests.sh` | Dedicated backend test runner with summary output |
| `run_ci.sh` | Centralized quality pipeline (`checks`, `tests`, `local-ci`) |
| `init_complete.sql` | Demo/test seed data (used by `db_seed.sh`) |

---

## Workflows

### 1. First-time setup (recommended: Docker)

```bash
# 1. Start the database container, create the DB, apply all migrations
./marreq-core/scripts/db_setup.sh

# 2. (Optional) Load demo/test data
./marreq-core/scripts/db_seed.sh

# 3. Start Marreq
cargo run -p marreq-server
```

`db_setup.sh` will:
- Start the `db` Docker service if it is not running
- Wait for PostgreSQL to be ready
- Create the `marreq` database if it does not exist
- Run `diesel migration run` (applies every migration in `migrations/` in order)

For the full application setup matrix (`marreq-server` / `marreq-cloud`, Docker / local), see [`docs/developer/setup.md`](../../docs/developer/setup.md).

### 2. Combined setup + seed in one command

```bash
./marreq-core/scripts/db_setup.sh --seed
```

### 3. Applying updates after pulling a new version

```bash
git pull
./marreq-core/scripts/db_migrate.sh up
cargo build --release
```

### 4. Rolling back a migration (development)

```bash
# Revert the most recent migration
./marreq-core/scripts/db_migrate.sh down

# Revert 2 migrations
./marreq-core/scripts/db_migrate.sh down 2
```

### 5. Check migration status

```bash
./marreq-core/scripts/db_migrate.sh list
```

Output legend: `[X]` = applied, `[ ]` = pending.

### 6. Backup before an update

```bash
# Save to ./backups/marreq_<timestamp>.sql.gz  (directory auto-created)
./marreq-core/scripts/db_backup.sh

# Custom output path
./marreq-core/scripts/db_backup.sh /var/backups/marreq_prod.sql.gz
```

Restore:
```bash
gunzip -c backups/marreq_<timestamp>.sql.gz | \
  docker exec -i <container> psql -U rust -d marreq
```

### 7. Full reset (development only)

```bash
# ⚠  Destroys all data
./marreq-core/scripts/db_reset.sh             # drops the database
./marreq-core/scripts/db_setup.sh --seed      # recreate and reload demo data
```

---

## Working Without Docker (bare-metal PostgreSQL)

Set `DATABASE_URL` to point at your server and ensure `psql` is in `PATH`.
`db_setup.sh` detects the absence of Docker Compose and switches to a
psql-based flow automatically:

```bash
export DATABASE_URL=postgres://myuser:mypass@myhost:5432/marreq
./marreq-core/scripts/db_setup.sh
```

The same applies to `db_migrate.sh`, `db_seed.sh`, and `db_backup.sh`.

---

## How Migrations Work

Schema is managed exclusively through Diesel migrations in `migrations/`.

| Migration | Description |
|---|---|
| `2026-01-31-000001_baseline_schema` | Full initial schema (all tables, indexes, triggers) |

Diesel tracks applied migrations in the `__diesel_schema_migrations` table.
Running `diesel migration run` (or `db_migrate.sh up`) is idempotent, already
applied migrations are skipped.

To add a new migration:
```bash
diesel migration generate <migration_name>
# Edit migrations/<timestamp>_<name>/up.sql and down.sql
./marreq-core/scripts/db_migrate.sh up
```

---

## Demo Data (`init_complete.sql`)

`db_seed.sh` executes `init_complete.sql`, which inserts:

**Users** (all passwords: `ChangeMe123!`):

| Username | Name | Role | Project |
|----------|------|------|---------|
| `alice` | Alice Johnson | Admin | Marreq Project |
| `dr_smith` | Dr. Sarah Smith | Admin | Space Project |
| `eng_jones` | Engineer Mike Jones | User | Space Project |
| `tech_lee` | Technician Lisa Lee | User | Space Project |
| `qa_wilson` | QA Specialist Tom Wilson | User | Space Project |
| `admin` | System Administrator | Admin | Marreq Project |

**Projects**: Space Project, Marreq Project, Empty Project

The seed file also includes sample requirements, tests, matrix traceability,
custom fields, and logs.  It refuses to run if the schema is not present or
if data already exists.

---

## Utility Scripts

### `reindex_project.sh`

Triggers semantic-search re-indexing for a project via the REST API.
Useful after bulk-importing requirements or if the embedding index is stale.

```bash
./marreq-core/scripts/reindex_project.sh
# Prompts for URL, username, password, and project ID
```
