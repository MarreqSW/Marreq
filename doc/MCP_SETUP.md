# MCP (Model Context Protocol) Setup for Marreq

Marreq can be used from AI assistants (Cursor, Claude, etc.) via an optional **MCP server** that exposes read-only tools. The MCP server talks to the Marreq REST API; it does not access the database directly.

## Architecture

- **AI client** (Cursor / Claude) ↔ **Marreq MCP server** (stdio) ↔ **Marreq REST API** (HTTP + Bearer token) ↔ **Database**
- All access is project-scoped and permission-checked. Every tool call is audited.

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
TOKEN_HASH=$(echo -n "$RAW_TOKEN" | sha256sum | cut -d' ' -f1)

# Insert token for user 1, optional project_id for scope (NULL = any project)
psql $DATABASE_URL -c "
  INSERT INTO user_api_tokens (user_id, token_hash, name, project_id)
  VALUES (1, '$TOKEN_HASH', 'MCP server', 1);
"
```

Use `MARREQ_API_TOKEN="$RAW_TOKEN"` when starting the MCP server. If `project_id` is set on the token, the server may only access that project (enforced by the API).

## 2. Environment variables for the MCP server

Environment variable names use the `MARREQ_*` prefix (formerly `REQMAN_*`). Update your config if you used the old names.

Set these before starting the MCP server (e.g. in a `.env` file or your shell):

| Variable | Required | Description |
|----------|----------|-------------|
| `MARREQ_BASE_URL` | Yes | Marreq API base URL (e.g. `http://localhost:8000`) |
| `MARREQ_API_TOKEN` | Yes | Raw API token (Bearer) |
| `MARREQ_PROJECT_ID` | Yes | Project ID to scope all tools |
| `MARREQ_MODE` | No | `read_only` (default) or `draft_write` (Phase 2) |
| `MARREQ_USER_ID` | No | User ID (for audit display) |
| `MARREQ_ROLE` | No | Role (for future use) |
| `MARREQ_SESSION_ID` | No | Session identifier (for audit correlation) |

## 3. Build and run the MCP server

```bash
cd mcp-server
npm install
npm run build
npm start
```

Or in one step: `npm run dev` (builds then runs). The server uses **stdio** transport: the AI client typically spawns it and communicates over stdin/stdout.

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
        "MARREQ_PROJECT_ID": "1"
      }
    }
  }
}
```

Use the absolute path to `mcp-server/dist/index.js` and the same env vars as above.

To enable Phase 2 write tools, set `MARREQ_MODE=draft_write` in the MCP server env (same config block). The server will then register `create_requirement`, `patch_requirement`, `set_approval`, and `create_baseline` in addition to the read-only tools.

## 5. Available tools

### Read-only (Phase 1, always available)

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

### Draft write (Phase 2, only when `MARREQ_MODE=draft_write`)

| Tool | Description |
|------|-------------|
| `create_requirement` | Create a new requirement in the project (draft). Parameters: title, description, reference_code, author_id, reviewer_id, category_id, status_id, applicability_id, verification_method_ids; optional parent_id, justification, custom_fields. |
| `patch_requirement` | Update a requirement (creates new version). Parameters: requirement_id, patch (title, description, status_id, etc. as needed). |
| `set_approval` | Set requirement version approval to `reviewed` or `approved`. Requires project owner/manager role. Parameters: requirement_id, version_id, state. |
| `create_baseline` | Create a new baseline snapshot for the project. Parameters: name, optional description. |

All tools are project-scoped to `MARREQ_PROJECT_ID`. Audit entries are written to Marreq (entity_type `MCP`, queryable in logs). Write tools are only registered when the server is started with `MARREQ_MODE=draft_write`.

## 6. Security notes

- **Token**: Store `MARREQ_API_TOKEN` securely; never commit it. Use env or a secrets manager.
- **Base URL**: For production, use HTTPS and a URL the MCP server can reach.
- **Project scope**: Prefer creating tokens with `project_id` set so a compromised token only exposes one project.
- **Read-only vs draft_write**: Default `MARREQ_MODE=read_only` exposes only read tools. Set `MARREQ_MODE=draft_write` to enable create/patch requirement, set approval, and create baseline (Phase 2).

## 7. Troubleshooting

- **Unauthorized (401)**: Check that `MARREQ_API_TOKEN` matches a token in `user_api_tokens` (compare SHA-256 hash of the raw token).
- **Forbidden (403)**: The token’s project scope (if set) must match `MARREQ_PROJECT_ID`; or the user must be a member of the project.
- **Connection refused**: Ensure Marreq is running and `MARREQ_BASE_URL` is correct (e.g. `http://localhost:8000`).
