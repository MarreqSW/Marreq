# MCP (Model Context Protocol) Setup for Marreq

Marreq can be used from AI assistants (Cursor, Claude, etc.) via an optional **MCP server** that exposes tools mapped to the Marreq REST API. The MCP server does not access the database directly.

## Architecture

- **Local AI client** ↔ **Marreq MCP server** (`stdio`) ↔ **Marreq REST API** (HTTP + Bearer token) ↔ **Database**
- **Remote AI client** ↔ **Marreq MCP server** (Streamable HTTP) ↔ **Marreq REST API** (HTTP + the request Bearer token) ↔ **Database**
- All access is project-scoped and permission-checked. Tool calls emit trusted, best-effort audit events through the internal-only `POST /api/mcp/audit` path.

`stdio` remains the default and keeps the existing environment contract. Remote
mode creates an independent MCP server/transport for every initialized session;
request identity is never stored in a process-global variable. Until delegated
OAuth is configured, remote mode accepts an existing Marreq API token as its
Bearer credential. The credential is resolved to a stable user/client/grant
principal. OAuth access-token rotation therefore does not break an established
session, while a token from another grant cannot assume it. The bearer from the
current MCP request is always the one forwarded to Rocket.
Sessions expire after 30 minutes idle or eight hours absolute and the process
admits at most 1,000 concurrent sessions. Closing the transport removes its
state; credentials are still sent and checked on every downstream REST call.

Remote deployments require `MARREQ_MCP_AUDIT_SECRET` (at least 32 random
characters) in both Rocket and MCP. It authenticates the internal audit call;
an end-user session, API token, or OAuth token alone cannot manufacture an MCP
audit event. Generate it with `openssl rand -hex 32` and never expose it to a
browser or external client.

## Remote Streamable HTTP

```dotenv
MARREQ_MCP_TRANSPORT=http
MARREQ_MCP_HOST=127.0.0.1
MARREQ_MCP_PORT=3000
MARREQ_MCP_PATH=/mcp
MARREQ_BASE_URL=http://127.0.0.1:8000
MARREQ_MCP_PUBLIC_URL=http://127.0.0.1:3000/mcp
```

Connect to `http://127.0.0.1:3000/mcp` and send `Authorization: Bearer
<Marreq API token or delegated OAuth access token>`. `MARREQ_API_TOKEN` and
`MARREQ_PROJECT_ID` are not used in HTTP mode because identity and project are
request/tool scoped. Remote mode registers the complete bounded tool surface;
OAuth scopes and normal Marreq permissions authorize each REST call rather than
server-wide `MARREQ_MODE` or `MARREQ_TRACE_WRITE` flags. Start with
`list_projects`, then pass the selected `project_id` to every project-scoped
tool. `MARREQ_MCP_ALLOWED_HOSTS` is an optional
comma-separated Host allowlist and should be set when listening on a non-loopback
interface.

`MARREQ_BASE_URL` is the private REST origin used by the Node process.
`MARREQ_MCP_PUBLIC_URL` is the canonical externally reachable OAuth resource
and is used in authentication challenges. In production it must be HTTPS and
must exactly match the same setting on Marreq Core; never expose an internal
container hostname in `MARREQ_MCP_PUBLIC_URL`.

Production deployments must terminate HTTPS at a trusted reverse proxy and
forward `/mcp` without logging `Authorization`, cookies, MCP bodies, or query
strings containing credentials. Preserve the `Mcp-Session-Id` and
`Last-Event-ID` headers and disable proxy buffering for event streams. Bind the
Node process to a private interface; do not expose plaintext HTTP publicly.

The tool operation is authoritative. The separate audit write is best-effort:
if audit persistence is unavailable, MCP logs a sanitized server-side error
without credentials and returns the already-committed domain result. This
avoids turning a successful mutation into an ambiguous failure and retry.

### Hosted clients and ChatGPT

Expose the HTTPS resource URL `https://your-marreq-origin.example/mcp`. A modern
MCP client discovers authorization from the protected-resource and
authorization-server metadata on that same origin, dynamically registers its
exact callback URI, and opens Marreq's consent page. The user signs in through
the normal Marreq password/federated flow before approving scopes. No API token
or upstream identity-provider credential is embedded in client configuration.

In ChatGPT or another hosted MCP client, add a custom remote MCP connection and
enter the `/mcp` URL. The client should perform OAuth discovery automatically.
If it asks for a client ID, register the client with `POST /oauth/register`
using the exact HTTPS callback URI supplied by the client. Self-hosted instances
must set `MARREQ_PUBLIC_BASE_URL` to the externally reachable HTTPS origin so
issuer and resource validation agree at every hop.
An unauthenticated request receives a `WWW-Authenticate: Bearer` challenge with
the `resource_metadata` URL for `/.well-known/oauth-protected-resource/mcp`.

Reusable requirements-engineering guidance is provided in
[`mcp-server/MARREQ_SKILL.md`](../../mcp-server/MARREQ_SKILL.md). It is plain
behavioral guidance and is not an authorization mechanism. No client-specific
manifest is committed: the integration uses standard remote MCP and OAuth
discovery rather than a fabricated or vendor-locked packaging format.

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
| `semantic_search_requirements` | Semantic requirement search (when embeddings are configured) |
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
| `create_requirement` | Create a requirement, including structured parent links; requires a persistent idempotency key in remote mode |
| `create_verification` | Create a project-scoped verification with persistent idempotency |
| `update_verification` | Update a project-scoped verification (status changes retain reviewer rules) |
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
| Requirement hierarchy | **Create with parent links**; standalone link/unlink is not exposed |
| Verifications | **List/get/create/update**; deletion is intentionally not exposed |
| Matrix get/put | **Yes** (extended read; put with `MARREQ_TRACE_WRITE`) |
| Trace up/down, coverage | **Yes** (core read) |
| `clear_suspect` | **Yes** (`MARREQ_TRACE_WRITE`) |
| Baselines: list/get/bundle/diff/diff_vs_current | **Yes** list/get bundle/diff/diff_vs_current; create via draft_write |
| Categories, applicability, statuses, methods, custom fields | **Read** via `list_project_catalog`; **CRUD** | **No** |
| Members, reviewers, permissions | **No** |
| Users, groups, projects (admin) | **No** |
| Semantic search | **Yes** |
| RAG ask / semantic reindex | **No** |
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
