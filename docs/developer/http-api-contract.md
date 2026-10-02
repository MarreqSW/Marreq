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
| `POST` | `/api/auth/change-password` | JSON body: `{ "current_password", "new_password", "confirm_password" }`. Requires a session. On success, all sessions are revoked and cookies cleared. |
| `GET` | `/api/auth/me` | Current user JSON; **401** if not authenticated (JSON body, not HTML). |
| `PUT` | `/api/auth/me` | Update your own profile. JSON `{ "name", "email", "current_password"? }`; returns the updated user. Username and admin flag are never changed here (extra fields are ignored). `current_password` is required when the email changes and the account has a password. **400** for validation errors, a missing/incorrect password, or an email change where emails must be verified (`requires_email_verification`, cloud); **409** when the email is already used; **401** without a session. |
| `GET` | `/api/auth/providers` | Enabled password/external methods; never includes client credentials. |
| `GET` | `/api/auth/external/{provider}/start` | Begin Authorization Code + PKCE login. |
| `GET` | `/api/auth/external/{provider}/callback` | Single-use provider callback. |
| `GET` | `/api/auth/identities` | List connected identities for the current user. |
| `POST` | `/api/auth/external/{provider}/link` | Begin an explicit, CSRF-protected account link. |
| `DELETE` | `/api/auth/identities/{id}` | Disconnect an identity if another login method remains. |
| `GET` | `/api/oauth/grants` | List delegated connected applications for the current user. |
| `DELETE` | `/api/oauth/grants/{id}` | Revoke an owned delegated application grant. |

Remote MCP OAuth discovery and protocol endpoints are mounted at the origin
root rather than under `/api`: `/.well-known/oauth-authorization-server`,
`/.well-known/oauth-protected-resource`, `/oauth/register`, `/oauth/authorize`,
and `/oauth/token`. See [authentication.md](authentication.md#delegated-applications).

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
| `PATCH` | `/api/projects/{project_id}` | Edit project properties. Body (all optional; absent fields stay unchanged): `name` (2–100 chars), `description` (≤ 1000; `null`/empty clears), `status` (`Active`, `Completed`, `OnHold`, `Cancelled`), `owner_id` (must be a project member), `group_id` (`null` = personal namespace). The slug is not editable (unknown fields → **422**). Requires `ManageProjectConfiguration` (project Admin) or instance admin; a group move also needs `ManageProjects` in both the current and the target group. Session or Bearer; send `X-CSRF-Token` with a session. Returns the updated project plus `project_base_path`; the change is audit-logged with old and new values. **400** validation, **403** permission, **404** unknown target group, **422** non-member owner or unknown status. |
| `DELETE` | `/api/projects/{project_id}` | Permanently delete the project and everything in it: requirements (versions, links, comments, custom values), verifications, matrix links, baselines and their snapshots, saved views (locked ones included), attachments and their files (a file shared with another project is kept), catalog, custom field definitions, storage quota, members, reviewers, project-scoped API tokens and notifications. Body: `{"confirm_slug": "<project slug>"}` (unknown fields → **422**). Only the project **owner** or an **instance admin**; session only (no Bearer tokens), send `X-CSRF-Token`. The project's audit history is kept with `project_id` cleared, and a `DELETE`/`PROJECT` entry records the name, slug and counts. **204** on success; **400** if `confirm_slug` does not match; **403** for anyone else; **404** unknown project. |
| `GET` | `/api/project-from-path/<slug>` | Resolve a browser path `/{slug}` to project metadata (`id`, `name`, `slug`, `route_slug`). **401** if not authenticated; **403** if not a member (non-admin); **404** if unknown. (Not under `/api/projects/…` to avoid Rocket route collisions.) |
| `GET` | `/api/projects/{project_id}/verifications` | Verifications (tests) in the project. Session or Bearer; requires `ViewRequirements`. |
| `POST` | `/api/projects/{project_id}/imports/excel/preview` | Multipart field `file` (`.xlsx` or `.csv`). Requires `EditRequirements`. Returns guessed `import_type`, `columns`, `sample_rows`, `row_count`, `available_fields`, and unique values per source column. |
| `POST` | `/api/projects/{project_id}/imports/excel` | Multipart `file`, `import_type` (`requirements`, `tests`, or `matrix`), `column_mappings` JSON, and optional `value_mappings` JSON (`target_field`, `source_value`, `target_id`). Unknown catalog/user names fall back to project defaults; supplied target IDs are validated against the project. Creates records; row errors are returned without failing the whole request. **`matrix`**: map `requirement_reference_code` and `verification_reference_code` to existing codes in this project; missing codes are reported and no records are invented; existing links are skipped. Send `X-CSRF-Token` and do not set `Content-Type` (browser sets the multipart boundary). |
| `POST` | `/api/projects/{project_id}/imports/reqif` | Multipart field `file`: a `.reqif`/`.xml` document (up to 20 MiB) or a ReqIFZ archive (detected by its ZIP signature, up to `MARREQ_REQIFZ_MAX_MB`, default 100; **413** above). Requires `EditRequirements`. Creates requirements using project catalog defaults (first status, category, applicability, verification method) and the current user as author/reviewer. Every `.reqif` in an archive is imported, and files referenced by XHTML `object` elements become attachments (type, size and quota checks as for uploads; failures are warnings). Returns `imported_count`, `created_link_count`, `imported_attachment_count`, `documents`, `errors`, and `warnings`. **400** for an unsafe archive (absolute or `..` paths, symlinks, duplicates, zip bombs) or a `.reqifz` that is not a ZIP. Send `X-CSRF-Token`. |
| `GET` | `/api/projects/{project_id}/exports/requirements.xlsx` | Whole-project requirements workbook (custom field columns plus a `Comments` sheet). Requires `ViewRequirements`. Responds with the xlsx media type and `Content-Disposition: attachment`. |
| `GET` | `/api/projects/{project_id}/exports/verifications.xlsx` | Whole-project verifications workbook. Requires `ViewRequirements`. Responds with the xlsx media type and `Content-Disposition: attachment`. |
| `GET` | `/api/projects/{project_id}/exports/matrix.xlsx` | Traceability matrix workbook: one row per requirement, one column per verification, `Yes` where a link exists. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/dsm` | Dependency structure matrix. Query: `link_types` (comma-separated; default all but `RELATES_TO`; empty = none), `order` (`hierarchy` default, or `partition`), `category_id`, `root_id` (subtree). Returns `requirements` (ordered), sparse `cells` (`row`, `col`, `link_types`, `link_ids`, `upstream_changed`, `in_loop`), `groups`, `loops` (`requirement_ids`, cycle `path`) and `stats`. Session or Bearer with `requirements:read` + `traceability:read`; requires `ViewRequirements`. **400** for an unknown link type or order; **404** if `root_id` is not in the project. |
| `GET` | `/api/projects/{project_id}/exports/dsm.xlsx` | The dependency structure matrix as a workbook (sheets `DSM`, `Loops`, `Legend`); same query parameters as `/dsm`. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/matrix-links.xlsx` | Two-column workbook (`requirement_code`, `verification_code`), one row per existing link. Same layout as Excel/CSV **Matrix links** import. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/requirements.pdf` | Requirements table as PDF (id, title, reference, status plus one column per custom field). Requires `ViewRequirements`. Responds with `application/pdf` and `Content-Disposition: attachment`. |
| `GET` | `/api/projects/{project_id}/exports/report.pdf` | Project summary report as PDF (totals, coverage, requirement and verification status breakdowns). Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/requirements.reqif` | Current project requirements as ReqIF 1.2 XML. Requires `ViewRequirements`. Responds with `application/xml` and `Content-Disposition: attachment`. |
| `GET` | `/api/projects/{project_id}/exports/baselines/{baseline_id}.reqif` | Immutable baseline snapshot as ReqIF 1.2 XML. Requires `ViewRequirements`. **404** if the baseline is missing or belongs to another project. |
| `GET` | `/api/projects/{project_id}/exports/requirements.reqifz` | ReqIFZ archive (`application/zip`): `requirements-project-{id}.reqif` plus each requirement attachment at `files/{attachment_id}/{filename}`, linked from the statement with `<xhtml:object class="marreq-attachment" data=… type=…>`. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/baselines/{baseline_id}.reqifz` | The baseline as a ReqIFZ archive with the files it recorded, including ones deleted since. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/bundle.json` | Versioned JSON project snapshot (`marreq.project-bundle.v1`): catalog, current requirements, verifications, matrix links, comments, and members by username. Requires `ViewRequirements`. |
| `POST` | `/api/projects/imports/bundle` | Create a **new** project from a `marreq.project-bundle.v1` JSON file (multipart `file`, optional `group_id`). Same auth as `POST /api/projects`. Maps users by username/email; missing users become warnings. Does not import into an existing project. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions` | List requirement versions (newest first). Each item is a `RequirementVersion` row (title, statement, catalog FKs, justification, approval). Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions/{version_id}` | One version snapshot. Same fields as the version row plus `custom_fields` (`field_id`, `label`, `value`) and `verification_method_ids`. **404** if the version is missing, belongs to another requirement, or the requirement is not in the project. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions/{old_version_id}/diff/{new_version_id}` | Structured, read-only requirement version diff. Returns added/removed/unchanged title, statement, and justification lines plus status, category, applicability, verification-method, and custom-field comparisons. Requires `ViewRequirements`; both versions must belong to the requirement. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/requirements/{requirement_id}/diff/current` | Compare the requirement version frozen in a baseline with its current version. Same structured requirement-diff payload. |
| `GET` | `/api/projects/{project_id}/verifications/{verification_id}/snapshots` | Chronological verification snapshots reconstructed from the audit log (`CREATE` / `UPDATE` / `STATUS_CHANGE`). Snapshot `id` is the producing log id, except `0` which is the state before the first recorded update when no create log exists. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/verifications/{verification_id}/snapshots/{old_snapshot_id}/diff/{new_snapshot_id}` | Structured, read-only verification snapshot diff. Returns added/removed/unchanged name, description, source, and reference lines plus status, verification type, and parent comparisons. Requires `ViewRequirements`; both snapshot ids must belong to the verification. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/verifications/{verification_id}/diff/current` | Compare the verification definition and status frozen in a baseline with the current verification. Returns the structured verification diff payload. |

Project-scoped CRUD and resources under `/api/projects/{project_id}/...` follow existing routes (Bearer token or session, per handler).

## Attachments and project storage

Files attached to requirements and verifications (issue #241). Session or Bearer token (project-scoped tokens are limited to their project). Reads need `ViewRequirements`, changes need `EditRequirements`. Errors use the usual JSON body (`status`, `error`, `message`).

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/projects/{project_id}/attachments?entity_type=&entity_id=` | Live attachments of one `requirement` or `verification`, oldest first: `id`, `entity_type`, `entity_id`, `filename`, `content_type`, `size_bytes`, `uploaded_by`, `uploaded_by_name`, `created_at`, `deleted` (always `false` here). **400** for another `entity_type`; **404** if the entity is not in the project. |
| `POST` | `/api/projects/{project_id}/attachments` | Multipart `file`, `entity_type`, `entity_id`; CSRF header for sessions. **201** with the attachment. **413** when the file is over `MARREQ_ATTACHMENT_MAX_MB` (default 10) or would take the project over its quota (the message gives the usage); **415** when the extension is not allowed or the content does not match it (checked against magic bytes; text types must be UTF-8); **400** for an empty file or missing name. Identical content is stored once and counts once per project. Audited as `CREATE ATTACHMENT`. |
| `GET` | `/api/projects/{project_id}/attachments/{attachment_id}/download` | The file, streamed with its stored `Content-Type`, `Content-Disposition: attachment; filename="<ascii>"; filename*=UTF-8''<name>`, `X-Content-Type-Options: nosniff` and `Content-Security-Policy: default-src 'none'; sandbox`. **404** for deleted attachments. |
| `DELETE` | `/api/projects/{project_id}/attachments/{attachment_id}` | **204**. Soft delete: the row and file are removed unless a baseline keeps them. Audited as `DELETE ATTACHMENT`. |
| `GET` | `/api/projects/{project_id}/storage` | `used_bytes` (distinct files, including ones only baselines keep), `retained_by_baselines_bytes`, `quota_bytes`, `quota_is_default`, `default_quota_bytes`, `max_file_bytes`, `allowed_extensions`. |
| `PUT` | `/api/projects/{project_id}/storage/quota` | **Instance administrators only** (**403** otherwise). JSON `{ "quota_mb": n }` (whole MB, at least 1) or `{ "quota_mb": null }` to use `MARREQ_PROJECT_STORAGE_QUOTA_MB` (default 500). Returns the storage object. A quota below the current usage only blocks new uploads. Audited as a project `UPDATE`. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/attachments` | Attachments recorded when the baseline was taken (live attachments of its requirements and of all verifications); `deleted: true` for files deleted since. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/attachments/{attachment_id}/download` | Like the download above, also for files deleted after the baseline. **404** if the attachment is not in that baseline. |

Uploads larger than Rocket's form limits never reach the handler; a JSON **413** catcher answers them. The limits are raised at startup when the per-file maximum needs it, and nginx allows 100 MB on `/api/`.

## Error format

Structured errors from `ApiError` responses:

```json
{
  "status": 400,
  "error": "Bad Request",
  "message": "human-readable detail"
}
```

HTTP status matches the error class (400, 401, 403, 404, 409, 410, 422, 500).

## Admin audit logs

Instance-wide (not project-scoped). All four routes require a **global administrator**; other authenticated users receive **403**.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/admin/logs` | JSON `{ items, total, limit, offset }`. Query: `entity_type`, `entity_id`, `user_id`, `action_type`, `project_id`, `since`, `until` (RFC 3339 or `YYYY-MM-DD[THH:MM]`), `limit` (default 50, max 100), `offset`. Each item matches entity activity (`log_id`, `user_id`, `username`, `action_type`, `summary`, `description`, `created_at`, `changes`) plus `entity_type`, `entity_id`, `project_id`. Newest first. |
| `GET` | `/api/admin/logs/export.json` | Same filters; `limit` default/max 10000. JSON attachment `audit-logs.json`. Records an `EXPORT` audit row. |
| `GET` | `/api/admin/logs/stats` | Activity summary. Query: `since`, `until` (same formats as the list; default: the last 30 UTC days ending now; at most 366 days, **400** otherwise or when `since > until`), `entity_type`, `action_type`, `user_id`, `project_id`, `top` (default 10, max 50). Response `{ since, until, total, active_users, by_day: [{ day, count }], by_action: [{ action_type, count }], by_user: [{ user_id, username, count }] }`. `by_day` has one zero-filled entry per UTC day; `by_action` / `by_user` are sorted by count and capped at `top`; `active_users` is not capped. |
| `POST` | `/api/admin/logs/cleanup` | JSON `{ "days": n }` with `n >= 1`. Response `{ "deleted": <count> }`. Removes rows older than `n` days. |

## Admin database backup

Self-hosted only; requires a **global administrator**.

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/admin/backup` | No body; needs the CSRF header. Runs `pg_dump --no-owner --no-privileges --compress=6` against `DATABASE_URL` and returns **200** with `Content-Type: application/gzip` and `Content-Disposition: attachment; filename="marreq-backup_<YYYYMMDD>_<HHMMSS>.sql.gz"` (gzipped plain SQL). **403** for non-admins; **410** when the deployment mode disallows backups (`allows_database_backup: false` in `GET /api/meta/deployment`, i.e. cloud); **500** with the `pg_dump` error summary if the dump fails. Success and failure are recorded as `EXPORT` audit rows. The `pg_dump` binary can be overridden with `MARREQ_PG_DUMP`. The Compose nginx allows up to 15 minutes for this route. |

## User management

Instance-wide. All routes require a **global administrator** (`users.is_admin`); other authenticated users receive **403**. Mutating routes need the CSRF header like any other SPA write.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/users` | JSON array of users (`id`, `username`, `name`, `email`, `creation_date`, `last_login`, `is_admin`, `email_verified`). Password hashes are never serialized. |
| `GET` | `/api/users/{id}` | One user; **404** if unknown. |
| `POST` | `/api/users` | JSON `{ username, name, email, password, is_admin }` (all required). Response `{ "status": "ok", "id": n }`. **400** for validation, password-policy, or (cloud) admin-promotion errors; **409** when the username/namespace is taken; **410** when the deployment only allows self-registration. |
| `PUT` | `/api/users/{id}` | JSON `{ username, name, email, is_admin }`. Returns the updated user. Username and email are trimmed and lowercased. **400** when changing `is_admin` is disabled in this deployment mode or when an admin removes their own admin flag; **409** when it would leave no administrator or the username is taken. |
| `PUT` | `/api/users/{id}/password` | JSON `{ new_password, confirm_password }`. **204** on success; the target user's sessions are revoked (unless it is the caller). **400** when the passwords differ or violate the password policy (message explains why). Recorded in the audit log without the password. |
| `DELETE` | `/api/users/{id}` | **204** on success. **400** when deleting your own account; **409** for the last administrator or when other records (groups, baselines, saved views, requirements, …) still reference the user. |

## Requirement statement format

`description` on requirements and verifications (create, update, field updates, responses, exports) is **Marreq statement Markdown**: paragraphs separated by a blank line (a single newline is a line break), `- ` / `* ` bulleted and `1. ` numbered lists, `**bold**`, `*italic*` / `_italic_`, `` `code` ``, and `[label](url)` links with `http:`, `https:`, or `mailto:` URLs (others are kept as literal text); `\` escapes a marker character. It is stored and returned as source text; the SPA renders it without HTML (`frontend/src/utils/statementMarkdown.ts`), and ReqIF export converts it to XHTML (`marreq-core/src/rich_text.rs`). The 2000-byte limit counts the Markdown source.

## Full route list

Rocket mounts all JSON routes under `/api` in `marreq-core/src/api/mod.rs` (shared routes) and the deployment crate's `src/routes.rs` (deployment-specific routes). The project [README](../../README.md) includes a human-maintained endpoint summary (requirements, tests, matrix, baselines, MCP audit, etc.).

A minimal **OpenAPI 3** sketch for auth and session project listing lives in [`openapi.yaml`](openapi.yaml) (extend or replace with generated spec later).

## Server flags (backend)

| Variable | Purpose |
|----------|---------|
| `MARREQ_UI_MODE=api_only` | Do not mount legacy HTML routes (`/`, `/p/...`, `/user/...`). |
| `MARREQ_SERVE_STATIC=0` | Do not serve `/static` from Rocket (SPA container serves assets). Default is off when `api_only`, on otherwise. |

## Docker (Compose frontends)

- **frontend:** nginx on host port **8080** → SPA + `location /api/` → `marreq-server:8000`.
- **frontend-cloud:** nginx on host port **8082** → SPA + `location /api/` → `marreq-cloud:8001` (cloud profile).
- **backend:** Rocket on **8000** or **8001** depending on deployment mode and compose profile.
- **Adminer:** host port **8081** (to avoid clashing with the frontend).

See [`../../docker/README.md`](../../docker/README.md).
