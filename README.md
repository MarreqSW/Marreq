# Requirement Manager (Marreq)

[![Codacy Badge](https://app.codacy.com/project/badge/Grade/972f03dc70864d4e807afd7d2adcd1b0)](https://app.codacy.com?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)

A comprehensive web-based requirements and test management system built with Rust, Rocket, and PostgreSQL. This software provides a complete solution for managing hierarchical requirements, tests, traceability matrices, and generating reports.

Documentation index (by audience): [docs/README.md](docs/README.md)

## ✨ Features

### 📋 Core Management
- **Multi-Project Support**: Manage multiple projects with isolated data
- **Requirements Management**: Create, edit, and organize hierarchical requirements
- **Test Management**: Manage tests with status tracking and source documentation
- **Traceability Matrix**: Visual mapping between requirements and tests; requirement detail page lists **all** linked tests per requirement (“Verified by” section with links to test pages)
- **User Management**: Assign authors and reviewers to requirements with authentication

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

### 🎨 Modern UI
- **Responsive Design**: Works on desktop and mobile devices
- **Modern Interface**: Clean, card-based layout with consistent styling
- **Intuitive Navigation**: Easy-to-use interface with clear visual hierarchy
- **Professional Styling**: Consistent color scheme and typography

### 🔌 API Access
- **RESTful API**: Complete programmatic access to all data
- **JSON Format**: Standard JSON responses for integration
- **CRUD Operations**: Full Create, Read, Update, Delete support
- **Project-Scoped**: All API operations respect project boundaries; project-scoped routes support both session and Bearer token auth (e.g. for MCP)
- **API tokens**: Bearer token auth for headless clients (e.g. MCP); tokens can be scoped to a project (see [MCP Setup](docs/developer/mcp-setup.md))

### 🤖 MCP (Model Context Protocol)
- **MCP server**: Optional TypeScript MCP server in `mcp-server/` that exposes Marreq data as MCP tools for AI assistants (Cursor, Claude, etc.). Talks to the Marreq REST API with Bearer token; all tools are project-scoped.
- **Read-only (default)**: get_requirement, list_requirements, get_versions, compare_versions, trace_up, trace_down, coverage_report, get_baseline, diff_baselines.
- **Phase 2 (draft_write)**: When `MARREQ_MODE=draft_write`, additional tools: create_requirement, patch_requirement, set_approval, create_baseline.
- **Audit**: Every tool call is logged to Marreq (POST /api/mcp/audit) for compliance.
- See [MCP Setup](docs/developer/mcp-setup.md) for environment variables, tool list, and how to run the server.

### ✅ Requirement approval workflow (UI)
- **Detail page**: Approval badge (draft / reviewed / approved), metadata (approved by, date), and contextual actions: *Mark as Reviewed* and *Approve Requirement* (for project owners/managers). Confirmation modals before each transition.
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

For a fully initialized database with pre-configured users and sample data, use the helper script described in the [scripts README](scripts/README.md), in particular [`setup_database.sh`](scripts/setup_database.sh).

Typical flow:
- Start database: `docker compose up -d db`
- Initialize DB with sample data: `./scripts/setup_database.sh`
- Start app: `cargo run --bin marreq`

Then open **http://localhost:8000** in your browser (demo admin user `alice` uses password `ChangeMe123!`).

For detailed database setup options (automated, manual, reset, verification) see the [database setup guide](docs/developer/database-setup.md).

## 📖 Usage

### Web Interface

1. **Requirements**: Navigate to project requirements to view and manage requirements (with version history). On requirement and version detail pages, use the **Comments** section to read and add comments (add form hidden when the version is approved and comments are locked).
2. **Tests**: Go to project tests to manage test cases
3. **Matrix**: Visit project matrix to view the traceability matrix
4. **Baselines**: From project dashboard or nav, open **Baselines** to create immutable snapshots, **view** a baseline (detail page with requirements and traceability), compare requirements to current via **Diff vs current** (opens diff modal; hidden when unchanged), or export a baseline as ReqIF
5. **Categories**: Access project categories to manage requirement categories
6. **Applicability**: Visit project applicability to manage applicability options

### Export Features

- **Requirements Export**: Click "Export Excel" on the requirements page or homepage
- **Matrix Export**: Click "Export Excel" on the matrix page
- **ReqIF Export**: Use "Export → ReqIF (current)" for live project, or "ReqIF (from baseline…)" to pick a baseline and download its snapshot as ReqIF 1.2 XML
- **File Format**: Excel downloads as `.xls`; ReqIF as XML

### Import Features

- **Excel Parser**: Standalone application to parse exported Excel files and import data via API
- **ReqIF 1.2 Import**: Import requirements from ReqIF XML into a project (project ReqIF/Import page)
- **Data Import**: Import requirements, tests, and traceability data from Excel or ReqIF
- **API Integration**: Seamless integration with the main application's REST API

## 🔌 API Reference

### Base URL
```
http://localhost:8000/api
```

All API routes are mounted at `/api` in [src/app.rs](src/app.rs). When adding or changing API endpoints, update this section so the list stays in sync.

### Endpoints

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

- For end-to-end database setup and reset via scripts, see the [scripts README](scripts/README.md) (section `setup_database.sh`).
- For a full description of the schema, sample projects/users, and manual initialization commands, see the [database setup guide](docs/developer/database-setup.md).

### Migrations
Database schema changes are managed through Diesel migrations:
```bash
# Create new migration
diesel migration generate migration_name

# Run migrations
diesel migration run

# Revert migrations
diesel migration redo
```

**Note**: Migrations are the single source of truth for schema creation/evolution. `scripts/init_complete.sql` is seed data only (sample projects/users/requirements) and should be run after migrations.

## 🛠️ Development

### Project Structure
```
Marreq/
├── src/
│   ├── main.rs              # Application entry point
│   ├── models.rs            # Data models
│   ├── schema.rs            # Database schema (auto-generated)
│   ├── helper_functions.rs  # Database operations
│   ├── routes/              # Route handlers
│   ├── generators/          # Report generators
│   └── html/               # Static assets
├── templates/              # Handlebars templates
├── migrations/             # Database migrations
├── docs/                  # Documentation (developers/architects/users)
│   ├── README.md          # Documentation index
├── mcp-server/            # Optional MCP server (Node/TypeScript) for AI assistants
├── scripts/               # Dev tooling & DB setup
│   ├── init_complete.sql  # Sample data seed (schema must already exist)
│   └── setup_database.sh  # Automated database setup
└── docker-compose.yml     # Docker database configuration
```

### Key Technologies
- **Backend**: Rust with Rocket web framework
- **Database**: PostgreSQL with Diesel ORM
- **Frontend**: Handlebars templates with custom CSS
- **Reports**: Excel generation with xlsxwriter
- **Containerization**: Docker for database

### Building
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run all checks (fmt, clippy, stylelint, purgecss, npm ci, npm test)
./run_checks.sh
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

#### Database Connection Issues
```bash
# Check if database container is running
docker ps | grep Marreq_db_1

# Check database connectivity
docker exec Marreq_db_1 psql -U rust -d marreq -c "SELECT 1;"

# Restart database container
docker-compose restart
```

#### Application Startup Issues
If the app exits immediately with **"Database setup failed"**, set `DATABASE_URL` (e.g. in `.env`) and ensure the database is reachable. The app uses Result-based pool initialization and will not start without a valid pool.

```bash
# Check if port 8000 is in use
lsof -i :8000

# Kill existing process
kill <PID>

# Start application with specific binary
cargo run --bin marreq
```

#### Login Issues
- **Default credentials**: Seeded admin `alice` has password `ChangeMe123!` (change after first login).
- **Available users**: alice, dr_smith, eng_jones, tech_lee, qa_wilson, admin
- **Reset passwords**: Update database directly or re-run setup script

#### Database Reset
```bash
# Complete database reset
docker exec Marreq_db_1 psql -U rust -d postgres -c "DROP DATABASE IF EXISTS marreq;"
./scripts/setup_database.sh
```

### Verification Commands

```bash
# Verify database setup
docker exec Marreq_db_1 psql -U rust -d marreq -c "\dt"

# Check user creation
docker exec Marreq_db_1 psql -U rust -d marreq -c "SELECT username, name, is_admin FROM users;"

# Verify sample data
docker exec Marreq_db_1 psql -U rust -d marreq -c "SELECT COUNT(*) as requirements FROM requirements;"
```

### Performance Issues

- **Database indexes**: The initialization script includes optimized indexes
- **Connection pooling**: Application uses connection pooling for better performance
- **Query optimization**: Consider adding indexes for custom queries

## 📞 Support

For issues and questions, please open an issue on the project repository.

### Getting Help

1. **Check troubleshooting section** above
2. **Review application logs** for error messages
3. **Verify database setup** using verification commands
4. **Check Docker container status**
5. **Open an issue** with detailed error information
