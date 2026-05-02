# Requirement Manager (Marreq)

[![Codacy Badge](https://app.codacy.com/project/badge/Grade/972f03dc70864d4e807afd7d2adcd1b0)](https://app.codacy.com?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)

A comprehensive web-based requirements and test management system built with **Rust**, **Rocket**, and **PostgreSQL**. The **primary UI** is a **React 19 + Vite + TypeScript** SPA (Tailwind CSS; React Flow for traceability), talking to a **JSON API** under `/api`. Legacy client assets remain under `frontend/static/` for gradual migration (`@static` in Vite). This software supports hierarchical requirements, tests (verifications), traceability matrices, baselines, reports, and exports.

Documentation index (by audience): [docs/README.md](docs/README.md)

## ✨ Features

### 📋 Core Management
- **Multi-Project Support**: Manage multiple projects with isolated data
- **Requirements Management**: Create, edit, and organize hierarchical requirements
- **Test Management**: Manage tests with status tracking and source documentation
- **Traceability Matrix**: Visual mapping between requirements and tests; requirement detail page lists **all** linked tests per requirement (“Verified by” section with links to test pages)
- **User Management**: Assign authors and reviewers to requirements with authentication
- **Project reviewers**: Per-project list of members who may change **requirement status**, **verification status**, and **version approval** (draft / reviewed / approved); configured in the SPA **Settings** or via `GET`/`PUT /api/projects/<id>/reviewers`

### 🏷️ Advanced Features
- **Requirement comments**: Comment threads on requirements and optional requirement versions; author, timestamp, optional version reference; chronological list; immutable after creation. UI panel on requirement/version detail pages; approved versions can be locked from new comments (`LOCK_APPROVED_VERSION_COMMENTS`). Comments in audit logs and in Excel/ReqIF exports.
- **Requirement version diff**: Compare two requirement versions or a baseline snapshot vs current; API returns structured diff (metadata, verification, text) with optional **labels** (e.g. Status “Draft”, verification method titles) in addition to IDs; UI diff modal shows these labels for easier reading
- **Categories**: User-defined categories for organizing requirements (project-specific)
- **Applicability**: Define product lines, system types, or project scopes (project-specific)
- **Status Tracking**: Track requirement status (Draft, Accepted, Rejected, etc.)
- **Verification Methods**: Specify verification types (Test, Analysis, Review, etc.)
- **Authentication**: Secure login system with password management
- **Project Isolation**: Data separation between different projects

### 📊 Reporting & Export
- **Excel Export**: Export requirements with all fields to Excel format; includes a **Comments** sheet (requirement_id, version_id, author, created_at, body)
- **Matrix Export**: Export traceability matrix to Excel
- **ReqIF 1.2**: Import and export requirements as ReqIF XML; export current project or an immutable baseline; comments included as **Remarks** attribute per requirement when present
- **Comprehensive Data**: All metadata included in exports (categories, applicability, dates, comments, etc.)

### 📸 Immutable Baselines
- **Project Baselines**: Create point-in-time snapshots of all requirement versions and traceability
- **Immutable**: Baselines and their contents cannot be updated or deleted (enforced at DB and API level)
- **Version Snapshot**: Each baseline stores which `requirement_version` was current per requirement, plus the traceability matrix at creation time
- **Export from Baseline**: ReqIF and UI support exporting a specific baseline for audits or releases
- **API & UI**: Create/list/view baselines via REST API and web UI (project Baselines section, nav, requirements export dropdown)
- **Baseline detail page**: View baseline metadata, requirements table, and full traceability list; requirement and test **references** (e.g. REQ-PWR-001, TEST-PWR-001) are shown instead of raw IDs, with one row per (requirement, test) link
- **Diff vs current**: From a baseline view, compare a requirement’s snapshot to the current version in a **diff modal**; the “Diff vs current” action is hidden when the requirement is unchanged (same version as current)

### 🎨 Web UI (React SPA)
- **Stack**: React 19, Vite 6, TypeScript, Tailwind CSS; light/dark theming and shared design tokens
- **Routing**: Login, project-scoped requirements and verification views, requirement editor, traceability graph — see [frontend/README.md](frontend/README.md)
- **Responsive**: Layout tuned for desktop and smaller viewports
- **Same-origin API**: Session cookies and CSRF work when the SPA and `/api` share an origin (Docker nginx or Vite dev proxy); details in [docs/developer/http-api-contract.md](docs/developer/http-api-contract.md)

### 🔌 API Access
- **RESTful API**: Complete programmatic access to all data
- **JSON Format**: Standard JSON responses for integration
- **CRUD Operations**: Full Create, Read, Update, Delete support
- **Project-Scoped**: All API operations respect project boundaries; project-scoped routes support both session and Bearer token auth (e.g. for MCP)
- **API tokens**: Bearer token auth for headless clients (e.g. MCP); tokens can be scoped to a project (see [MCP Setup](docs/developer/mcp-setup.md))

### 🤖 MCP (Model Context Protocol)
- **MCP server**: Optional TypeScript MCP server in `mcp-server/` that exposes a **subset** of the REST API as MCP tools for AI assistants (Cursor, Claude, etc.). Bearer token; project-scoped. Full parity matrix: [MCP Setup](docs/developer/mcp-setup.md).
- **Core read (default)**: `MARREQ_MODE=read_only` — requirements, trace, coverage, baselines (get + diff).
- **Extended read**: `MARREQ_MODE=read_extended` (or `draft_write`) — verifications, baseline list, audit activity, comments, matrix read, catalog, baseline-vs-current diff.
- **Draft write**: `MARREQ_MODE=draft_write` — create/patch requirement, approvals, baselines, requirement comments.
- **Trace write**: `MARREQ_TRACE_WRITE=true` — replace verification matrix links, clear suspect flags.
- **Audit**: Every tool call is logged to Marreq (`POST /api/mcp/audit`).

### ✅ Requirement approval workflow (UI)
- **Detail page**: Approval badge (draft / reviewed / approved), metadata (approved by, date), and contextual actions: *Mark as Reviewed* and *Approve Requirement* for users in the **project reviewers** list (or administrators). Confirmation modals before each transition.
- **Edit when approved**: Clicking *Edit* on an approved requirement shows a warning that editing creates a new Draft version; user can cancel or proceed.
- **Version history**: Each version shows its approval state; list and detail show approval consistently.
- **List view**: Approval column and filters (*Approved only* / *Not approved*). Approval state is read-only once set; transitions are explicit and audit-friendly.

## ToDo List
+ [X] Hierarchy for
  + [X] Requirements
  + [X] Tests
+ [X] Better webpage 
  + [X] Use templates (based on hbs)
  + [X] Modern CSS design system
  + [X] Responsive layout
+ [X] Reports generator
  + [X] Excel export for requirements
  + [X] Excel export for traceability matrix
  + [ ] Latex template
  + [ ] PDF document
+ [X] Categories management
  + [X] CRUD operations
  + [X] API endpoints
+ [X] Applicability management
  + [X] CRUD operations
  + [X] API endpoints
+ [X] REST API (comprehensive)
  + [X] Requirements endpoints
  + [X] Tests endpoints
  + [X] Categories endpoints
  + [X] Applicability endpoints
  + [X] Matrix endpoints
+ [x] Operations logging
+ [X] Parsers for requirements
  + [ ] Latex files (Write a command)
  + [ ] Word files (Write a macro)
  + [X] Excel files
+ [ ] Parsers for tests
  + [ ] Doxygen documentation
  + [ ] ...
+ [X] Multiple projects
+ [X] Optimize DB access
  + [X] Reduce SQL queries
  + [X] DB pool
+ [X] Security
  + [ ] Use https
  + [X] users/admin
+ [X] Snapshots
  + [X] Immutable project baselines (requirement versions + traceability)
  + [ ] Configuration management
+ [X] Better error management
  + [X] Remove unwrap/expect in production paths (guards, routes, DB init, Excel export)
  + [X] Result-based DB pool init; clear startup failure message
  + [X] try_repo_read/try_repo_write for non-panicking lock in request path

## 🚀 Quick Start

### Prerequisites

+ **PostgreSQL**: Database backend (provided via Docker)
+ **Docker & Docker Compose**: For database containerization
+ **Rust**: Programming language
+ **clang** /  **libclang-dev**: Required by `xlsxwriter`

### Installation

#### Quick Start (Recommended)

For a fully initialized database with pre-configured users and sample data, use the helper scripts described in the [scripts README](marreq-core/scripts/README.md), in particular [`db_setup.sh`](marreq-core/scripts/db_setup.sh) (optionally followed by [`db_seed.sh`](marreq-core/scripts/db_seed.sh)).

Typical flow:
- Start database: `docker compose -f docker/docker-compose.yml up -d db`
- Initialize DB schema: `./marreq-core/scripts/db_setup.sh`
- Load sample data (optional): `./marreq-core/scripts/db_seed.sh`
- Start API (self-hosted mode): `cargo run -p marreq-server` — serves **http://127.0.0.1:8000** with JSON under **`/api`**.

For the **full browser UI** locally, run the SPA against that API:

```bash
cd frontend
npm install
npm run dev   # http://localhost:5173 — proxies /api → http://127.0.0.1:8000
```

Demo admin user **`alice`** / **`ChangeMe123!`** (change after first login).

For the full setup matrix (**`marreq-server`** vs **`marreq-cloud`**, Docker vs local), see the [setup guide](docs/developer/setup.md). For database-only details (automated, manual, reset, verification), see the [database setup guide](docs/developer/database-setup.md).

### Docker: API backend + SPA frontend

The default [docker/docker-compose.yml](docker/docker-compose.yml) stack runs **db**, **marreq-server** (Rocket **JSON API** on `127.0.0.1:8000`), **frontend** (nginx serving the **production Vite build** on **http://localhost:8080** with **`/api/`** proxied to `marreq-server`), and **adminer** (**http://localhost:8081**). The `cloud` compose profile adds **`marreq-cloud`** on **`127.0.0.1:8001`** plus **`frontend-cloud`** on **http://localhost:8082**. See the [setup guide](docs/developer/setup.md), [docker/README.md](docker/README.md), and the [HTTP API contract](docs/developer/http-api-contract.md).

Workspace layout, deployment modes, and build commands: [docs/developer/workspace-layout.md](docs/developer/workspace-layout.md). SPA scripts, routes, and API mapping: [frontend/README.md](frontend/README.md).

## 📖 Usage

### Web interface

Use the SPA (**Docker** `http://localhost:8080` or **`npm run dev`** in `frontend/`). Sign in, pick a project, then use requirements, verifications, traceability, and related flows from the in-app navigation.

1. **Requirements**: View and manage requirements (versions, comments where exposed in the UI). Comments on requirement/version views follow project rules (e.g. locked on approved versions when configured).
2. **Verifications (tests)**: Manage verification records per project
3. **Traceability**: Matrix / graph views linking requirements and verifications
4. **Baselines**: Create and inspect immutable snapshots; export ReqIF where the UI exposes it; diff vs current when available
5. **Categories & applicability**: Managed per project where the UI provides entry points (additional admin flows may be API-only)

### Export Features

- **Requirements Export**: Click "Export Excel" on the requirements page or homepage
- **Matrix Export**: Click "Export Excel" on the matrix page
- **ReqIF Export**: Use "Export → ReqIF (current)" for live project, or "ReqIF (from baseline…)" to pick a baseline and download its snapshot as ReqIF 1.2 XML
- **File Format**: Excel downloads as `.xls`; ReqIF as XML

### Import Features

- **Excel Import (Web UI)**: Upload `.xlsx`/`.csv` files in the project import flow with column mapping
- **ReqIF 1.2 Import**: Import requirements from ReqIF XML into a project (project ReqIF/Import page)
- **Data Import**: Import requirements and related metadata from Excel or ReqIF
- **Indexing integration**: Imported requirements are queued for semantic index refresh when embeddings are enabled

## 🔌 API Reference

**Interchangeable clients:** session auth, CSRF, and Docker/nginx notes are documented in [docs/developer/http-api-contract.md](docs/developer/http-api-contract.md). A partial [OpenAPI spec](docs/developer/openapi.yaml) covers auth and session-scoped project listing.

### Base URL
```
http://localhost:8000/api
```

Behind the Docker frontend (or Vite dev), use the **same origin** as the SPA (e.g. `http://localhost:8080/api/...`). JSON routes are mounted under `/api` from `marreq-core/src/api/mod.rs` (shared routes) and the deployment crate's `src/routes.rs` (deployment-specific routes). When adding or changing API endpoints, update this section so the list stays in sync.

### Endpoints

#### Auth & session (JSON / SPA)
- `GET /auth/csrf` — JSON `{ "csrf_token" }` for mutating requests (`X-CSRF-Token` header)
- `POST /auth/login` — JSON body `username`, `password`; sets session + CSRF cookies
- `POST /auth/logout` — clears session
- `GET /auth/me` — current user or **401** JSON (not HTML login page)
- `GET /projects` — projects for logged-in user (admin: all; others: memberships)
- `GET /project-from-path/{namespace}/{slug}` — resolve `/{namespace}/{slug}` to project id (SPA deep links; **403** if not a member)
- `GET /projects/{project_id}/verifications` — list verifications (tests) in the project (`ViewRequirements`)

#### Requirements
- `GET /requirements` - List all requirements
- `GET /requirements/{id}` - Get specific requirement
- `GET /requirements/{id}/versions` - List versions for a requirement (newest first)
- `GET /requirements/{req_id}/versions/{version_id}` - Get a specific requirement version
- `GET /requirements/{req_id}/versions/{v1}/diff/{v2}` - Diff two requirement versions (structured JSON: text and metadata added/removed/unchanged; includes optional labels for status, category, applicability, verification)
- `PUT /requirements/{req_id}/versions/{version_id}/approval` - Set approval state (body: `state`: "reviewed" | "approved"; project owners/managers only)
- `GET /requirements/{id}/comments` - List comments for a requirement (query: optional `version_id`; chronological order)
- `POST /requirements/{id}/comments` - Add a comment (body: `body`, optional `requirement_version_id`; approved versions rejected when `LOCK_APPROVED_VERSION_COMMENTS=true`)
- `POST /requirements` - Create new requirement
- `PATCH /requirements/{id}` - Partially update supported requirement fields
- `DELETE /requirements/{id}` - Delete requirement

**Project-scoped (session or Bearer token):**
- `GET /projects/{project_id}/requirements` - List requirements; query: `approval_state`, `has_tests`
- `GET /projects/{project_id}/requirements/{id}` - Get requirement with trace summary (parent, children, linked tests)
- `GET /projects/{project_id}/requirements/{req_id}/versions/{v1}/diff/{v2}` - Diff two versions (requirement must belong to project)
- `POST /projects/{project_id}/requirements` - Create requirement (body must include `project_id` matching route)
- `PATCH /projects/{project_id}/requirements/{id}` - Partially update requirement
- `PUT /projects/{project_id}/requirements/{req_id}/versions/{version_id}/approval` - Set approval state (body: `state`: "reviewed" | "approved")

#### Tests
- `GET /tests` - List all tests
- `GET /tests/{id}` - Get specific test
- `POST /tests` - Create new test
- `POST /tests/{id}/field` - Partially update a test field (body: `field`, `value`; supported fields: name, description, source, status_id, reference_code, parent_id)
- `DELETE /tests/{id}` - Delete test

#### Categories
- `GET /categories` - List all categories
- `GET /categories/{id}` - Get specific category
- `POST /categories` - Create new category
- `PUT /categories/{id}` - Update category
- `DELETE /categories/{id}` - Delete category

#### Applicability
- `GET /applicability` - List all applicability options
- `GET /applicability/{id}` - Get specific applicability
- `POST /applicability` - Create new applicability
- `PUT /applicability/{id}` - Update applicability
- `DELETE /applicability/{id}` - Delete applicability

#### Matrix
- `GET /matrix` - Get traceability matrix data
- `GET /projects/{project_id}/matrix` - Get traceability matrix for a project (session or Bearer)

#### Baselines (immutable snapshots)
- `GET /projects/{project_id}/baselines` - List baselines for a project
- `GET /projects/{project_id}/baselines/{baseline_id}` - Get baseline metadata
- `POST /projects/{project_id}/baselines` - Create baseline (body: `name`, `description`; captures current requirement versions and traceability)
- `GET /projects/{project_id}/baselines/{baseline_id}/requirements` - Get requirements as stored in the baseline
- `GET /projects/{project_id}/baselines/{baseline_id}/requirements/{req_id}/diff/current` - Diff requirement in baseline vs current version (structured JSON with optional labels for status, category, applicability, verification)
- `GET /projects/{project_id}/baselines/{baseline_id}/traceability` - Get traceability snapshot for the baseline

#### Users
- `GET /users` - List all users
- `GET /users/{id}` - Get specific user
- `POST /users` - Create new user
- `DELETE /users/{id}` - Delete user

#### Status
- `GET /status` - List all status options
- `GET /status/{id}` - Get specific status
- `POST /status` - Create new status

#### Traceability
- `GET /projects/{project_id}/requirements/{id}/trace_up` - Get parent requirement(s) (session or Bearer)
- `GET /projects/{project_id}/requirements/{id}/trace_down` - Get child requirements and linked tests (session or Bearer)
- `GET /projects/{project_id}/coverage_report` - Requirements without tests, tests without requirements, suspect links (session or Bearer)
- `POST /traceability/clear_suspect` - Clear suspect flag for a traceability link (body: `req_id`, `test_id`; records current user and timestamp)

#### Semantic search (project-scoped; requires embeddings/RAG when enabled)
- `GET /projects/{project_id}/requirements/semantic_search` - Search requirements by semantic similarity (query params: `q`, optional `k`, filters)
- `POST /projects/{project_id}/requirements/ask` - RAG answer over project requirements (body: `query`, optional `k`, filters)
- `POST /projects/{project_id}/requirements/reindex` - Reindex all requirements for the project (admin only)
- `GET /projects/{project_id}/requirements/index_status` - Get indexing status for the project
- `GET /projects/{project_id}/requirements/semantic_search/status` - Check if semantic search is enabled

#### Cache (internal/operational)
- `GET /cache/stats` - Cache statistics
- `POST /cache/clear` - Clear cache
- `POST /cache/cleanup` - Remove expired entries
- `GET /cache/performance` - Cache performance metrics
- `GET /cache/recommendations` - Cache tuning recommendations
- `POST /cache/reset-counters` - Reset performance counters
- `GET /cache/health` - Cache health check

### Example API Usage

```bash
# Get all requirements
curl http://localhost:8000/api/requirements

# Create a new category
curl -X POST http://localhost:8000/api/categories \
  -H "Content-Type: application/json" \
  -d '{"title": "API", "description": "API requirements", "tag": "API"}'

# Export requirements to Excel
curl -O http://localhost:8000/requirements.xls
```

## 🗄️ Database

### Schema
The application uses PostgreSQL with the following main entities:
- **Projects**: Multi-project support with project metadata
- **Requirements**: Logical requirement containers; current content lives in **requirement_versions** (immutable version history)
- **Requirement versions**: Immutable snapshots of requirement content (title, description, status, category, applicability, etc.)
- **Requirement comments**: Immutable comments attached to a requirement (general) or a specific version; author, body, created_at; optional requirement_version_id
- **Tests**: Test cases with status and source information, project association
- **Matrix**: Traceability links between requirements and tests (live), project-scoped
- **Baselines**: Immutable project snapshots; **baseline_requirements** stores which requirement_version was current per requirement, **baseline_traceability** stores the matrix at baseline time
- **Categories**, **Applicability**, **Requirement status**, **Test status**, **Verification**: Project-scoped lookup/config tables
- **Users**: System users (authors, reviewers) with authentication
- **User API tokens**: Bearer tokens for headless/MCP access; optional project scope; hashed storage, last_used_at tracking
- **Logs**: Audit trail for all system activities

For a full entity-relationship diagram see [docs/architecture/database-schema.md](docs/architecture/database-schema.md) (Mermaid).

### Database Initialization System

A comprehensive database initialization system is provided, including SQL files, helper scripts, pre-configured users, and rich sample data.

- For end-to-end database setup and reset via scripts, see the [scripts README](marreq-core/scripts/README.md) (`db_setup.sh`, `db_seed.sh`, `db_reset.sh`).
- For a full description of the schema, sample projects/users, and manual initialization commands, see the [database setup guide](docs/developer/database-setup.md).

### Migrations
Database schema changes are managed through Diesel migrations (`diesel.toml` lives in **`marreq-core/`**; run CLI commands from there):
```bash
cd marreq-core

# Create new migration
diesel migration generate migration_name

# Run migrations
diesel migration run

# Revert migrations
diesel migration redo
```

**Note**: Migrations are the single source of truth for schema creation/evolution. `marreq-core/scripts/init_complete.sql` is seed data only (sample projects/users/requirements) and should be run after migrations.

## 🛠️ Development

### Project structure
```
Marreq/
├── Cargo.toml              # Virtual workspace (marreq-core, marreq-server, marreq-cloud)
├── Cargo.lock              # Single lock-file for the whole workspace
├── Makefile                # make server / make cloud / make test / …
├── marreq-core/            # shared lib (domain, persistence, Rocket primitives)
│   ├── Cargo.toml
│   ├── src/                # Library source (api/, auth/, services/, models/, …)
│   ├── migrations/         # Diesel migrations (schema source of truth)
│   ├── diesel.toml
│   └── scripts/            # Dev tooling & DB helpers
│       ├── db_setup.sh
│       ├── db_seed.sh
│       ├── db_migrate.sh
│       ├── db_reset.sh
│       ├── db_backup.sh
│       ├── run_checks.sh   # fmt, clippy, stylelint, purgecss, npm test
│       ├── run_tests.sh
│       └── init_complete.sql
├── marreq-server/          # self-hosted binary (admin-managed users)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Rocket launch for server mode
│       ├── deployment.rs   # impl DeploymentMode for Server
│       ├── api/            # Server-only REST handlers
│       └── routes.rs       # pub fn routes() for server-only routes
├── marreq-cloud/           # hosted/SaaS binary (self-registration)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Rocket launch for cloud mode
│       ├── deployment.rs   # impl DeploymentMode for Cloud
│       ├── api/            # Cloud-only REST handlers (register, verify-email, …)
│       ├── services/       # registration_service.rs
│       ├── fairings/       # cloud_admin_bootstrap.rs
│       └── routes.rs       # pub fn routes() for cloud-only routes
├── frontend/               # React + Vite SPA + legacy static/
│   ├── src/                # React app (main.tsx, routes, components)
│   ├── static/             # Legacy JS/CSS (optional @static alias)
│   └── package.json
├── docs/                   # Documentation (developers/architects/users)
│   ├── README.md           # Documentation index
│   └── ReqIF/              # ReqIF standards and reference docs
├── mcp-server/             # Optional MCP server (Node/TypeScript) for AI assistants
├── docker/                 # Container files (compose, Dockerfile, entrypoint, CI override)
│   ├── docker-compose.yml  # Main Docker Compose stack
│   ├── docker-compose.ci.yml
│   ├── Dockerfile          # Backend image (MARREQ_BIN=marreq-server|marreq-cloud)
│   ├── frontend/           # Frontend image + nginx config
│   ├── docker-entrypoint.sh
│   └── README.md
```

### Key technologies
- **Backend**: Rust, Rocket, Diesel, PostgreSQL
- **Frontend**: React 19, TypeScript, Vite, Tailwind CSS (see [frontend/README.md](frontend/README.md))
- **Legacy assets**: `frontend/static/` (not loaded by default in the React shell)
- **Reports**: Excel generation with xlsxwriter
- **Containerization**: Docker Compose (db, backend, frontend, optional services)

### Building
```bash
# Development build (all crates)
cargo build --workspace

# Release build
cargo build --workspace --release

# Run tests (all crates)
cargo test --workspace

# Run tests for a specific crate
cargo test -p marreq-core

# Run all checks (fmt, clippy, stylelint, purgecss, npm ci, npm test)
bash marreq-core/scripts/run_checks.sh

# Run backend test suite with summary output
bash marreq-core/scripts/run_tests.sh

# Run local CI flow (supports --jobs)
bash marreq-core/scripts/run_ci.sh local-ci --jobs 2
```

## 📝 License

This project is open source. See LICENSE file for details.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 🔧 Troubleshooting

### Common Issues

#### Application Startup Issues
If the app exits immediately with **"Database setup failed"**, set `DATABASE_URL` (e.g. in `.env`) and ensure the database is reachable. The app uses Result-based pool initialization and will not start without a valid pool.

```bash
# Check if port 8000 is in use
lsof -i :8000

# Kill existing process
kill <PID>

# Start application (self-hosted mode)
cargo run -p marreq-server
```

#### Login Issues
- **Default credentials**: Seeded admin `alice` has password `ChangeMe123!` (change after first login).
- **Available users**: alice, dr_smith, eng_jones, tech_lee, qa_wilson, admin
- **Reset passwords**: Update database directly or re-run setup script

### Performance Issues

- **Database indexes**: The initialization script includes optimized indexes
- **Connection pooling**: Application uses connection pooling for better performance
- **Query optimization**: Consider adding indexes for custom queries

## 📞 Support

For issues and questions, please open an issue on the project repository.

### Getting Help

1. **Check troubleshooting section** above
2. **Review application logs** for error messages
3. **Run Docker checks/reset** from [docker/README.md](docker/README.md)
4. **Check Docker container status**
5. **Open an issue** with detailed error information
