# MCP (Model Context Protocol) Setup for Marreq

Marreq can be used from AI assistants (Cursor, Claude, etc.) via an optional **MCP server** that exposes tools mapped to the Marreq REST API. The MCP server does not access the database directly.

## Architecture

- **AI client** (Cursor / Claude) ↔ **Marreq MCP server** (stdio) ↔ **Marreq REST API** (HTTP + Bearer token) ↔ **Database**
- All access is project-scoped and permission-checked. Every tool call is audited (see `postAudit` → `POST /api/mcp/audit`).

The MCP server implements a **subset** of the HTTP API on purpose (smaller attack surface). A full route-by-route matrix is in [API parity (MCP vs REST)](#api-parity-mcp-vs-rest) below.

## Prerequisites

1. **Marreq** running with the API available (e.g. `cargo run --bin marreq`).
2. **API token** for a Marreq user, with optional project scope (see below).
3. **Node.js** 18+ to run the MCP server.

## 1. Create an API token (Marreq)

API tokens are stored in the `user_api_tokens` table. You need to insert a row with a **hashed** token (SHA-256 hex of the raw token). Example using `psql` and a generated secret:

```bash
# Generate a random token (keep this secret)
RAW_TOKEN="your-secret-token-here"

# SHA-256 hex hash (e.g. with openssl)
TOKEN_HASH=$(echo -n "$RAW_TOKEN" | sha256sum | cut -d ' ' -f1)

# Insert token for user 1, optional project_id for scope (NULL = any project)
psql $DATABASE_URL -c "
  INSERT INTO user_api_tokens (user_id, token_hash, name, project_id)
  VALUES (1, '$TOKEN_HASH', 'MCP server', 1);
"
```

Use `MARREQ_API_TOKEN="$RAW_TOKEN"` when starting the MCP server. If `project_id` is set on the token, the server may only access that project (enforced by the API).

## 2. Environment variables for the MCP server

Set these before starting the MCP server (e.g. in a `.env` file or your shell):

| Variable | Required | Description |
|----------|----------|-------------|
| `MARREQ_BASE_URL` | Yes | Marreq API base URL (e.g. `http://localhost:8000`) |
| `MARREQ_API_TOKEN` | Yes | Raw API token (Bearer) |
| `MARREQ_PROJECT_ID` | Yes | Project ID to scope all tools |
| `MARREQ_MODE` | No | `read_only` (default), `read_extended`, or `draft_write` — see [Tool tiers](#tool-tiers) |
| `MARREQ_TRACE_WRITE` | No | If `true` / `1` / `yes`, registers matrix replace and clear-suspect tools (see below) |
| `MARREQ_USER_ID` | No | User ID (for audit display) |
| `MARREQ_ROLE` | No | Role (for future use) |
| `MARREQ_SESSION_ID` | No | Session identifier (for audit correlation) |

### Tool tiers

| Tier | Env | Tools |
|------|-----|--------|
| **Core read** | `MARREQ_MODE=read_only` (default) | Requirements (get/list/versions/diff), trace up/down, coverage report, baseline get + baseline diff |
| **Extended read** | `MARREQ_MODE=read_extended` **or** `draft_write` | Core read **plus**: list/get verifications, list baselines, requirement/verification audit activity, requirement comments (list), verification matrix (read), project catalog (categories, applicability, statuses, methods, custom fields), diff baseline vs current requirement |
| **Draft write** | `MARREQ_MODE=draft_write` | Extended read **plus**: create/patch requirement, set version approval, create baseline, create requirement comment |
| **Trace / matrix write** | `MARREQ_TRACE_WRITE=true` (any mode) | `put_verification_matrix`, `clear_suspect` — still requires matching API permissions (`EditRequirements`, etc.) |

`draft_write` implies extended read tools (same as `read_extended` for read surface).

## 3. Build and run the MCP server

The convenience script `mcp-server/run.sh` reads `MARREQ_API_TOKEN` and
`MARREQ_PROJECT_ID` from the environment — set them in your shell or in a
local `.env` file that you source beforehand:

```bash
# One-time: export your personal values
export MARREQ_API_TOKEN=<raw-token-from-step-1>
export MARREQ_PROJECT_ID=1

# Then run the server
cd mcp-server && ./run.sh
```

Or build and start manually:

```bash
cd mcp-server
npm install
npm run build
npm start
```

Or in one step: `npm run dev` (builds then runs). The server uses **stdio** transport: the AI client typically spawns it and communicates over stdin/stdout.

> **Never commit `MARREQ_API_TOKEN` to git.** `run.sh` intentionally has no
> hardcoded token and will fail with a clear error if the variable is unset.

## 4. Configure your AI client (e.g. Cursor)

Add the Marreq MCP server to your client config. Example for Cursor (in project or user MCP settings):

```json
{
  "mcpServers": {
    "marreq": {
      "command": "node",
      "args": ["/path/to/Marreq/mcp-server/dist/index.js"],
      "env": {
        "MARREQ_BASE_URL": "http://localhost:8000",
        "MARREQ_API_TOKEN": "your-secret-token-here",
        "MARREQ_PROJECT_ID": "1",
        "MARREQ_MODE": "read_extended",
        "MARREQ_TRACE_WRITE": "false"
      }
    }
  }
}
```

Use the absolute path to `mcp-server/dist/index.js` and the same env vars as above.

For Phase 2 requirement/baseline writes, set `MARREQ_MODE=draft_write`. For traceability matrix edits without other draft tools, you can keep `read_only` or `read_extended` and set `MARREQ_TRACE_WRITE=true`.

## 5. Available tools (by name)

### Core read (`read_only` and above)

| Tool | Description |
|------|-------------|
| `get_requirement` | Get a requirement by id (with trace summary: parent, children, linked tests) |
| `list_requirements` | List requirements; optional filter by `approval_state` (draft/reviewed/approved) and `has_tests` (true/false) |
| `get_versions` | Version history for a requirement |
| `compare_versions` | Structured diff between two requirement versions |
| `trace_up` | Parent requirement(s) for a requirement |
| `trace_down` | Child requirements and linked tests |
| `coverage_report` | Requirements without tests, tests without requirements, suspect links |
| `get_baseline` | Baseline metadata, requirements snapshot, and traceability |
| `diff_baselines` | Compare two baselines (requirements and traceability diff) |

### Extended read (`read_extended` or `draft_write`)

| Tool | Description |
|------|-------------|
| `list_verifications` | List verifications (tests) in the project |
| `get_verification` | Get one verification; must belong to `MARREQ_PROJECT_ID` |
| `list_baselines` | List baseline metadata rows for the project |
| `get_requirement_activity` | Audit log entries for a requirement |
| `get_verification_activity` | Audit log entries for a verification |
| `list_requirement_comments` | Comments on a requirement; optional `requirement_version_id` |
| `get_verification_matrix` | Requirement ids linked to a verification |
| `list_project_catalog` | Categories, applicability, statuses, verification methods, custom fields (project-filtered) |
| `diff_baseline_vs_current` | Diff baseline snapshot vs current requirement version |

### Draft write (`draft_write` only)

| Tool | Description |
|------|-------------|
| `create_requirement` | Create a new requirement in the project |
| `patch_requirement` | Update a requirement (creates new version). Changing `status_id` requires project reviewer rules on the API |
| `set_approval` | Set requirement version approval to `reviewed` or `approved` |
| `create_baseline` | Create a new baseline snapshot |
| `create_requirement_comment` | Add a comment on a requirement |

### Trace write (`MARREQ_TRACE_WRITE=true`)

| Tool | Description |
|------|-------------|
| `put_verification_matrix` | Replace all requirement links for a verification |
| `clear_suspect` | Clear suspect flag on a matrix link (`req_id`, `verification_id`) |

All tools are scoped to `MARREQ_PROJECT_ID` where the API provides a project path. Audit entries are written to Marreq (`POST /api/mcp/audit`).

## 6. API parity (MCP vs REST)

Reference: shared route list in `marreq-core/src/api/mod.rs` (plus deployment-specific routes in `marreq-server/src/routes.rs` / `marreq-cloud/src/routes.rs`). This table states whether an MCP tool exists for that capability.

| REST area | MCP coverage |
|-----------|----------------|
| Auth (login, logout, CSRF, me) | **No** — use Bearer token |
| Dashboard / session projects | **No** |
| Requirements: list/get/versions/diff/patch/create/delete (global paths) | **Partial** — project-scoped get/list/versions/diff/patch/create; **no** delete via MCP |
| Requirements: impacted tests | **No** |
| Activity (`.../requirements/:id/activity`, `.../verifications/:id/activity`) | **Yes** (extended read) |
| Comments list/create | **Yes** (extended / draft_write) |
| Version parent links CRUD | **No** |
| Verifications: list/get | **Yes** (extended); create/update/delete | **No** |
| Matrix get/put | **Yes** (extended read; put with `MARREQ_TRACE_WRITE`) |
| Trace up/down, coverage | **Yes** (core read) |
| `clear_suspect` | **Yes** (`MARREQ_TRACE_WRITE`) |
| Baselines: list/get/bundle/diff/diff_vs_current | **Yes** list/get bundle/diff/diff_vs_current; create via draft_write |
| Categories, applicability, statuses, methods, custom fields | **Read** via `list_project_catalog`; **CRUD** | **No** |
| Members, reviewers, permissions | **No** |
| Users, groups, projects (admin) | **No** |
| Semantic search / reindex | **No** |
| Cache admin | **No** |
| MCP audit endpoint | **Internal** (called after each tool) |

## 7. Security notes

- **Token**: Store `MARREQ_API_TOKEN` securely; never commit it. Use env or a secrets manager.
- **Base URL**: For production, use HTTPS and a URL the MCP server can reach.
- **Project scope**: Prefer creating tokens with `project_id` set so a compromised token only exposes one project.
- **Modes**: Default `read_only` limits tools. Use `read_extended` only when assistants need verifications, audit trails, or catalog. Use `draft_write` and `MARREQ_TRACE_WRITE` only for trusted automation.

## 8. Troubleshooting

- **Unauthorized (401)**: Check that `MARREQ_API_TOKEN` matches a token in `user_api_tokens` (compare SHA-256 hash of the raw token).
- **Forbidden (403)**: The token’s project scope (if set) must match `MARREQ_PROJECT_ID`; or the user must be a member of the project with permission for the action.
- **Connection refused**: Ensure Marreq is running and `MARREQ_BASE_URL` is correct (e.g. `http://localhost:8000`).
- **Tool not found**: Confirm `MARREQ_MODE` and `MARREQ_TRACE_WRITE` — extended and trace tools are only registered when those settings enable them.
