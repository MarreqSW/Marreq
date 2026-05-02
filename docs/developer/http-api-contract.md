# Marreq HTTP API contract (interchangeable frontends)

This document describes how **any** browser or native client can talk to Marreq when the server runs in **JSON API mode** (`MARREQ_UI_MODE=api_only`). The bundled Vite SPA in `frontend/` is the reference implementation; alternative UIs must follow the same rules.

## Base URL

- **Production (Docker split stack):** same origin as the UI, path prefix `/api` (nginx proxies `/api/*` to Rocket). Example: `https://app.example.com/api/...`.
- **Local Rocket only:** `http://localhost:8000/api/...`.
- **Local Vite dev:** configure `vite.config.ts` to proxy `/api` to Rocket, and use relative URLs such as `/api/auth/me` so the browser origin stays `http://localhost:5173`.

Frontend builds can set `VITE_API_BASE` (empty or `/api`) if you centralize the prefix in code.

## Cookies and CORS

- Session authentication uses **HTTP-only private cookies** set by the server: typically **`session`** on HTTP (e.g. local dev) or **`__Host-session`** when `MARREQ_SECURE_SESSION_COOKIE=1` over HTTPS (see `marreq-core/src/auth/session.rs`). Use `fetch(..., { credentials: 'include' })` for all API calls that need a session.
- **Production:** Prefer a **single origin** (nginx serves SPA + proxies `/api`) so cookies are first-party and CORS is unnecessary.
- **Development (split origins, e.g. Vite `5173` + Rocket `8000`):**
  - Add the dev UI origin to `CORS_ALLOWED_ORIGINS` (defaults include `http://localhost:5173`).
  - Set `CORS_ALLOW_CREDENTIALS=true` so browsers send cookies on cross-origin XHR/fetch.

## CSRF

State-changing requests are protected by the existing CSRF fairing:

- Cookie name: **`csrf`** (Rocket private cookie; not readable from JS).
- Header name: **`X-CSRF-Token`** — must match the value in the `csrf` cookie (same pattern as the HTML `<meta name="csrf-token">` flow).

**Discovery (recommended for SPAs):**

1. `GET /api/auth/csrf` → JSON `{ "csrf_token": "<token>" }` (also ensures the `csrf` cookie is set).
2. For `POST`/`PATCH`/`PUT`/`DELETE`, send header `X-CSRF-Token: <same token>`.

For **`POST /api/auth/login`** and **`POST /api/auth/logout`**, if the browser sends an **`Origin` or `Referer`** that is on the CSRF allowlist (same host as your SPA, e.g. `http://127.0.0.1:8080` in Docker), the request is accepted **even when** `X-CSRF-Token` and the `csrf` cookie disagree (split-stack / proxy edge cases). Other mutating `/api/*` routes still require a matching `X-CSRF-Token` + `csrf` cookie (or Bearer auth). Sending the header from `GET /api/auth/csrf` remains recommended.

## Authentication (session JSON)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/auth/csrf` | Return CSRF token string; refreshes cookie if needed. |
| `POST` | `/api/auth/login` | JSON body: `{ "username", "password" }` (same as `LoginForm`). Sets session + CSRF cookies on success. |
| `POST` | `/api/auth/logout` | Clears session and CSRF. |
| `GET` | `/api/auth/me` | Current user JSON; **401** if not authenticated (JSON body, not HTML). |

### Dashboard (SPA home)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/dashboard` | Authenticated dashboard payload: `user`, decorated `projects` (same shape as the legacy HTML index), `projects_count`, `selected_project_id`, `selected_project_slug`, `csrf_token`. **401** if not logged in. |

Successful login response (200): `{ "status": "ok", "user": { ... } }` (serialized `User` model).

Errors from API handlers use JSON (see below). Failed login typically returns **400** with a message (e.g. invalid credentials).

## Session-scoped project list

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/projects` | Projects visible to the logged-in user (admins: all projects; others: memberships). **401** if not authenticated. |
| `GET` | `/api/project-from-path/<namespace>/<slug>` | Resolve a browser path `/{namespace}/{slug}` to project metadata (`id`, `name`, `slug`, `route_slug`). **401** if not authenticated; **403** if not a member (non-admin); **404** if unknown. (Not under `/api/projects/…` to avoid Rocket route collisions.) |
| `GET` | `/api/projects/{project_id}/verifications` | Verifications (tests) in the project. Session or Bearer; requires `ViewRequirements`. |

Project-scoped CRUD and resources under `/api/projects/{project_id}/...` follow existing routes (Bearer token or session, per handler).

## Error format

Structured errors from `ApiError` responses:

```json
{
  "status": 400,
  "error": "Bad Request",
  "message": "human-readable detail"
}
```

HTTP status matches the error class (400, 401, 403, 404, 409, 422, 500).

## Full route list

Rocket mounts all JSON routes under `/api` in `marreq-core/src/api/mod.rs` (shared routes) and the deployment crate's `src/routes.rs` (deployment-specific routes). The project [README](../../README.md) includes a human-maintained endpoint summary (requirements, tests, matrix, baselines, MCP audit, etc.).

A minimal **OpenAPI 3** sketch for auth and session project listing lives in [`openapi.yaml`](openapi.yaml) (extend or replace with generated spec later).

## Server flags (backend)

| Variable | Purpose |
|----------|---------|
| `MARREQ_UI_MODE=api_only` | Do not mount legacy HTML routes (`/`, `/p/...`, `/user/...`). |
| `MARREQ_SERVE_STATIC=0` | Do not serve `/static` from Rocket (SPA container serves assets). Default is off when `api_only`, on otherwise. |

## Docker (two containers)

- **frontend:** nginx on host port **8080** → SPA + `location /api/` → `marreq-server:8000`.
- **backend:** Rocket on **8000** (internal to compose network unless you publish it).
- **Adminer:** host port **8081** (to avoid clashing with the frontend).

See [`../../docker/README.md`](../../docker/README.md).
