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
| `GET` | `/api/home/attention` | What needs the user's attention across the projects listed for them (the start screen, issue #387), newest first, at most 50 `items`. Each item has `kind`, `project_id`, `title`, `at`, and, depending on the kind, `requirement_id`, `reference_code`, `count`, `notification_id`, `notification_type`. Kinds: **`approval`** (one per requirement whose current version is *reviewed*, in projects where the user may change review gates); **`review`** (one per such project, `count` = draft requirements); **`suspect`** (one per project where the user has `EditRequirements`, `count` = suspect matrix links); **`notification`** (unread `review_assigned`, `comment_added` and `mentioned` notifications; `mentioned` is created when a new requirement comment mentions a project member as `@username`, issue #389). Archived projects are skipped, except for notifications. Also returns the totals `approvals`, `reviews`, `suspect_links`, `notifications`. Session only (no Bearer tokens). **401** if not logged in. |
| `GET` | `/api/search?q=&limit=` | Requirements whose reference code or title contains `q` (case-insensitive, at least 2 characters; shorter returns `[]`), across the projects the user can view. Ordered exact reference code, code prefix, code substring, then title match; `limit` defaults to 20, at most 50. Returns `[{project_id, requirement_id, reference_code, title}]`. Session only (no Bearer tokens). **401** if not logged in. |

Successful login response (200): `{ "status": "ok", "user": { ... } }` (serialized `User` model).

Errors from API handlers use JSON (see below). Failed login typically returns **400** with a message (e.g. invalid credentials).

## Session-scoped project list

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/projects` | Projects visible to the logged-in user (admins: all projects; others: memberships). **401** if not authenticated. |
| `PATCH` | `/api/projects/{project_id}` | Edit project properties. Body (all optional; absent fields stay unchanged): `name` (2–100 chars), `description` (≤ 1000; `null`/empty clears), `status` (`Active`, `Completed`, `OnHold`, `Cancelled`), `owner_id` (must be a project member), `group_id` (`null` = personal namespace). The slug is not editable (unknown fields → **422**). Requires `ManageProjectConfiguration` (project Admin) or instance admin; a group move also needs `ManageProjects` in both the current and the target group. Session or Bearer; send `X-CSRF-Token` with a session. Returns the updated project plus `project_base_path`; the change is audit-logged with old and new values. **400** validation, **403** permission, **404** unknown target group, **422** non-member owner or unknown status. |
| `DELETE` | `/api/projects/{project_id}` | Permanently delete the project and everything in it: requirements (versions, links, comments, custom values), verifications, matrix links, baselines and their snapshots, saved views (locked ones included), attachments and their files (a file shared with another project is kept), catalog, custom field definitions, storage quota, members, reviewers, project-scoped API tokens and notifications. Body: `{"confirm_slug": "<project slug>"}` (unknown fields → **422**). Only the project **owner** or an **instance admin**; session only (no Bearer tokens), send `X-CSRF-Token`. The project's audit history is kept with `project_id` cleared, and a `DELETE`/`PROJECT` entry records the name, slug and counts. **204** on success; **400** if `confirm_slug` does not match; **403** for anyone else; **404** unknown project. |
| `POST` | `/api/projects/{project_id}/archive` | Archive the project: it becomes **read-only for everyone**, instance admins included, until unarchived. Reads and exports (including bundles, ReqIF and reports) keep working, and the project stays in `/api/projects` and `/api/dashboard` (the dashboard entries carry `archived: true` and `archived_at`). Every change to it, through the web API, Bearer tokens or MCP, is refused with **403** `{"message": "This project is archived; unarchive it to make changes."}`; members who cannot view it get the usual **403**/**404**. Deleting it is still allowed. No body. Only the project **owner** or an **instance admin**; session only, send `X-CSRF-Token`. Returns the project with `archived_at` and `archived_by` set; audit-logged as `ARCHIVE`. **403** for anyone else; **409** if already archived. |
| `POST` | `/api/projects/{project_id}/unarchive` | Make an archived project editable again, unchanged. Same rules as archiving; audit-logged as `UNARCHIVE`. **409** if not archived. |
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
| `GET` | `/api/projects/{project_id}/exports/report.pdf` | The built-in **Traceability & coverage** report document as PDF (same as `POST …/reports/coverage.pdf` without a body; issue #354). Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/reports/types` | Report document types (`vcd`, `coverage`) with `title`, `description`, `formats` (`pdf`, `odt`), their `sections` (`key`, `title`, `description`, `default_enabled`, `options` with `kind` `select`/`multi_select`/`text`, `choices`, `default`) and a `default_definition` filled for this project and the caller. Requires `ViewRequirements`. See [reports.md](reports.md). |
| `GET` | `/api/projects/{project_id}/report_templates` | Saved report templates the caller can see: their own private ones and every shared one (instance admins see all). Each has `id`, `name`, `report_type`, `visibility`, `owner_id`, `owner_name`, `definition` (normalised: sections added since it was saved are listed, disabled), `can_edit`, timestamps. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/report_templates/{template_id}` | One template; **404** if it is in another project or private to someone else. |
| `POST` | `/api/projects/{project_id}/report_templates` | Body `{name, visibility?: "private"\|"shared", definition}`. **201** with the template. Name 1–120 chars, unique per owner ignoring case (**409**); invalid definition **400**. Requires `ViewRequirements`; audit-logged (`REPORT_TEMPLATE`). |
| `PATCH` | `/api/projects/{project_id}/report_templates/{template_id}` | Change `name`, `visibility` and/or `definition` (its `report_type` is fixed: **400**). Only the owner, a project Admin or an instance admin (**403**). Audit-logged. |
| `DELETE` | `/api/projects/{project_id}/report_templates/{template_id}` | **204**. Same rule as `PATCH`. Audit-logged. |
| `POST` | `/api/projects/{project_id}/reports/{vcd\|coverage}.{pdf\|odt}` | Generate a report document. Optional body `{template_id?, definition?}`: an unsaved `definition` wins over `template_id`; without either the built-in default is used. The definition's `report_type` must match the URL (**400**). Returns the file (`application/pdf` or `application/vnd.oasis.opendocument.text`) with `Content-Disposition: attachment; filename="{doc_id}-{date}.{ext}"`. Requires `ViewRequirements`; session (with `X-CSRF-Token`) or Bearer. **404** for an unknown type or format, or a template the caller cannot see. |
| `GET` | `/api/projects/{project_id}/exports/requirements.reqif` | Current project requirements as ReqIF 1.2 XML. Requires `ViewRequirements`. Responds with `application/xml` and `Content-Disposition: attachment`. |
| `GET` | `/api/projects/{project_id}/exports/baselines/{baseline_id}.reqif` | Immutable baseline snapshot as ReqIF 1.2 XML. Requires `ViewRequirements`. **404** if the baseline is missing or belongs to another project. |
| `GET` | `/api/projects/{project_id}/exports/requirements.reqifz` | ReqIFZ archive (`application/zip`): `requirements-project-{id}.reqif` plus each requirement attachment at `files/{attachment_id}/{filename}`, linked from the statement with `<xhtml:object class="marreq-attachment" data=… type=…>`. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/baselines/{baseline_id}.reqifz` | The baseline as a ReqIFZ archive with the files it recorded, including ones deleted since. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/bundle.json` | Versioned JSON project snapshot (`marreq.project-bundle.v1`): catalog, current requirements, verifications, matrix links, comments, members by username, and `use_cases: [{reference_code, document, published, links: [{element_id, requirement_reference_code \| verification_reference_code, rationale}]}]` (the current version's document, or the draft's before the first publish; the section is left out when empty). Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/exports/bundle.zip` | The bundle with files (`application/zip`, filename `project-{slug}-bundle.zip`): `bundle.json` (same format, plus `attachments: [{entity_type, entity_reference_code, filename, content_type, path}]`) and each live attachment of the current requirements and verifications at `files/{attachment_id}/{filename}`. Requires `ViewRequirements`; **500** when attachment storage is not configured. |
| `POST` | `/api/projects/imports/bundle` | Create a **new** project from a bundle (multipart `file`, optional `group_id`): `bundle.json`, or a `bundle.zip` detected by its ZIP signature (up to `MARREQ_REQIFZ_MAX_MB`, default 100 MB; **413** above). Same auth as `POST /api/projects`. Maps users by username/email; missing users become warnings. Each listed file is attached with the upload checks (type, per-file limit, the new project's quota); a file that fails, is missing or has an unknown reference becomes a warning (`imported_counts.attachments` counts the rest). An unsafe archive or one without `bundle.json` is **400**. Use cases are created with their document, published as version 1 when they were published, and linked again by reference code (`imported_counts.use_cases`, `use_case_links`); a bundle without them still imports. Does not import into an existing project. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions` | List requirement versions (newest first). Each item is a `RequirementVersion` row (title, statement, catalog FKs, justification, approval). Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions/{version_id}` | One version snapshot. Same fields as the version row plus `custom_fields` (`field_id`, `label`, `value`) and `verification_method_ids`. **404** if the version is missing, belongs to another requirement, or the requirement is not in the project. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/versions/{old_version_id}/diff/{new_version_id}` | Structured, read-only requirement version diff. Returns added/removed/unchanged title, statement, and justification lines plus status, category, applicability, verification-method, and custom-field comparisons. Requires `ViewRequirements`; both versions must belong to the requirement. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/requirements/{requirement_id}/diff/current` | Compare the requirement version frozen in a baseline with its current version. Same structured requirement-diff payload. |
| `GET` | `/api/projects/{project_id}/verifications/{verification_id}/snapshots` | Chronological verification snapshots reconstructed from the audit log (`CREATE` / `UPDATE` / `STATUS_CHANGE`). Snapshot `id` is the producing log id, except `0` which is the state before the first recorded update when no create log exists. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/verifications/{verification_id}/snapshots/{old_snapshot_id}/diff/{new_snapshot_id}` | Structured, read-only verification snapshot diff. Returns added/removed/unchanged name, description, source, and reference lines plus status, verification type, and parent comparisons. Requires `ViewRequirements`; both snapshot ids must belong to the verification. |
| `GET` | `/api/projects/{project_id}/baselines/{baseline_id}/verifications/{verification_id}/diff/current` | Compare the verification definition and status frozen in a baseline with the current verification. Returns the structured verification diff payload. |
| `GET` | `/api/projects/{project_id}/verifications/{verification_id}/control` | Verification control data (issue #353): `verification_level`, `verification_stage`, `evidence_reference` (each `null` when unset), `updated_by`, `updated_at`, and `suggestions` (`levels`, `stages` already used in the project). Requires `ViewRequirements`. **404** if the verification is in another project. |
| `PUT` | `/api/projects/{project_id}/verifications/{verification_id}/control` | Replace the three fields. Body: `verification_level`, `verification_stage` (≤ 40 chars), `evidence_reference` (≤ 2000); values are trimmed and an empty string or a missing field clears it; unknown fields → **422**. Requires `EditRequirements`; send `X-CSRF-Token` with a session. Audit-logged as `UPDATE VERIFICATION`. **400** too long. |
| `GET` | `/api/projects/{project_id}/requirements/{requirement_id}/close_out` | The requirement's `compliance` (`C`, `PC`, `NC` or `null`), `note`, `set_by`, `set_at`, its linked `verifications` (`id`, `reference_code`, `name`, `status_id`, `status_title`, `outcome`) and the derived `close_out` (`status`: `open`/`closed`, `reason`). Closed when at least one verification is linked, all have outcome `passed`, and compliance is `C` or `PC`. Requires `ViewRequirements`. |
| `GET` | `/api/projects/{project_id}/close_out` | The same payload for every requirement of the project, keyed by requirement id. Requires `ViewRequirements`. |
| `PUT` | `/api/projects/{project_id}/requirements/{requirement_id}/compliance` | Body `{"compliance": "C"\|"PC"\|"NC"\|null, "note": …}`; `null` clears the assessment. Project **reviewers** only (like changing a verification status) and `EditRequirements`; send `X-CSRF-Token` with a session. Returns the close-out payload; audit-logged as `UPDATE REQUIREMENT`. **400** invalid value, **403** not a reviewer, **404** requirement in another project. |

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

## Use cases

Use cases with a structured scenario, a shared working draft and approved versions (issue #398) live under `/api/projects/{project_id}/use-cases`. Their routes, document format, draft operations and error codes are described in [use-cases-api.md](use-cases-api.md).

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
| `GET` | `/api/admin/backup/info` | `{ attachments_available, attachment_files, attachment_bytes }`: what a backup with files would contain (`attachments_available: false` when attachment storage is not configured). **403** for non-admins, **410** like below. |
| `POST` | `/api/admin/backup` | No body; needs the CSRF header. Query `attachments` (default `true`). Runs `pg_dump --no-owner --no-privileges` against `DATABASE_URL` and returns **200** with `Content-Type: application/gzip` and `Content-Disposition: attachment; filename="marreq-backup_<YYYYMMDD>_<HHMMSS>.tar.gz"`: a tar.gz with `marreq-backup_<…>/database.sql` (plain SQL), `attachments/` (the attachment store, unless `attachments=false` or storage is not configured) and `manifest.json` (`format: "marreq.backup.v1"`, `includes_attachments`, `attachment_files`, `attachment_bytes`). Before issue #341 this was a `.sql.gz`. **403** for non-admins; **410** when the deployment mode disallows backups (`allows_database_backup: false` in `GET /api/meta/deployment`, i.e. cloud); **500** with the `pg_dump` error summary if the dump fails. Success and failure are recorded as `EXPORT` audit rows. The `pg_dump` binary can be overridden with `MARREQ_PG_DUMP`. The Compose nginx allows up to 15 minutes for this route. |

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

## P0 contract-hardening audit and current coverage (#423)

**This is an incomplete bootstrap, not completion of the P0 prerequisite.** The
OpenAPI inventory lists all 245 compiled HTTP API operations: 241 on Server and
245 on Cloud, including origin-root OAuth routes. Currently 39 operations have
explicit wire schemas; 206 are marked `x-marreq-unverified` and are inventory
entries only. The release completeness gate deliberately rejects them. A passing
OpenAPI validator or PR check does **not** establish full API compatibility.

Architecture assessment:

| Boundary | Implementation and observations |
| --- | --- |
| Shared backend | `marreq-core/src/app.rs::declared_mounts` composes policy-classified API and root OAuth routes. Rocket guards enforce sessions, Bearer scopes, membership and permissions. Diesel models are not automatically wire schemas: flattened DTOs, computed JSON, omitted fields and custom responders matter. |
| Server | `marreq-server/src/routes.rs` currently adds no endpoints. User creation/admin promotion and database backup are shared endpoints with deployment-mode restrictions, not absent Server/Cloud routes. |
| Cloud | Four additional public account endpoints: register, verify-email, forgot-password and reset-password. Their presence differs from Server; shared capabilities also differ at runtime. |
| Frontend | `frontend/src/api/*.ts`, plus direct fetches in pages/components. Session credentials and CSRF are essential; binary downloads, multipart imports and redirects cannot be modeled as ordinary JSON. Only metadata/provider discovery and selected use-case response models have migrated to generated types so far. |
| MCP | `mcp-server/src/client.ts` wraps project-scoped REST calls; `index.ts` defines tools, `remote.ts` owns MCP HTTP sessions and delegated credentials. Principal identity has nullable `client_id`/`grant_id`, even though the previous TypeScript definitions described optional strings/numbers. The Node MCP transport and `/healthz` are outside the Rust HTTP API. |
| Authentication | `/api/home/attention` now accepts `ProjectsRead` scoped credentials; the older session-only description above is stale. `/api/search` remains session-only. `/oauth/*` and `/.well-known/*` are mounted at the origin root. Guard/fairing responses can be empty or framework-generated; consumers must not assume every error is an `ApiError` JSON object. |
| Release | Existing component tags remain the publishing triggers. Independent artifact versions are allowed. Current `main` is never substituted for an unavailable historical contract. No deployment or global release migration is implemented here. |

The explicitly modeled surface includes public build/deployment/CSRF/provider
metadata, current user, MCP principal, the four Cloud account routes, three OAuth
discovery routes, and all 25 use-case operations (including draft operations,
publishing, versions, approvals, comments, links, baseline content and diffs).
Use-case document requests distinguish omitted defaults from response properties
that Serde always emits. Unmodeled inventory entries do not promise fabricated
response bodies/status codes. Origin-root UI/HTML and internal CSRF-denial routes
are excluded from the API inventory comparison.

### Source of truth and change workflow

`openapi.yaml` is the wire contract. This retains Rocket/Diesel and introduces no
backend runtime dependency. `api-contract-routes.json` and both
`generated-contract.ts` files are reproducible derivatives, not independently
maintained contracts. The source-review digest records which producer/consumer
sources were audited. It detects changes requiring review, but **does not prove
serialization correctness** and is not a replacement for integration tests.

When changing an endpoint:

1. Inspect the actual Rocket mount, guards, Serde serialization/default/flatten/
   skip behavior, response status, and both consumers. Update OpenAPI accordingly.
2. Add or extend producer exchanges and consumer tests. Remove
   `x-marreq-unverified` only after auditing the complete operation, not merely
   because its route exists. Preserve unknown error-body handling.
3. Regenerate derived artifacts and explicitly record the source review:

   ```sh
   python3 scripts/contracts/generate.py
   python3 scripts/contracts/check.py --record-source-review
   python3 scripts/contracts/check.py
   python3 -m unittest discover -s scripts/contracts -v
   ```

4. Run the compiled route check and isolated real-backend checks:

   ```sh
   cargo test --locked -p marreq-cloud --test api_contract_inventory
   npm ci --prefix frontend
   npm ci --prefix mcp-server
   scripts/contracts/test-backend.sh
   ```

Install Python tooling with `python3 -m pip install -r
scripts/contracts/requirements.txt` in a virtual environment. Backend builds need
`libpq-dev`; Docker and Node 22 are needed for the isolated smoke check. The script
creates a new loopback-only PostgreSQL container with tmpfs storage, applies
migrations, seeds synthetic OAuth credentials through the integration test, and
starts a real `marreq-server` process. It exercises the actual frontend transport
and built MCP client over HTTP. It records requests/responses, never Authorization
headers or cookies. It ignores existing database URLs when creating its test DB;
it never connects to a production database. Cleanup runs on exit. Missing/skipped
critical producer exchanges fail transcript validation.

### Remaining risks before P0 can be accepted

- Audit the remaining **206** operations: authentication protocol/session flows,
  projects/groups, requirements/verifications, traceability, matrices, baselines,
  catalogs, administration, search, imports, exports, reports and attachments.
  Presence in the inventory is not schema coverage.
- Complete per-consumer operation selection and generated request/response type
  adoption, including direct frontend fetches. Bundles currently conservatively
  require the entire backend surface; this can reject unrelated changes.
- Add exhaustive consumer request checks and live Cloud/session/OAuth protocol
  coverage. Current executable producer/consumer coverage is representative,
  not exhaustive, and no historical release is retrospectively certified.
- Retain verified bundles for real released artifact versions and bootstrap an
  accurate deployment plan. Versions in package manifests alone are insufficient.
- Close existing test/security failures before making this a release-ready change.

Until these are resolved, leave the PR in draft and keep publication blocked by
`check.py --require-complete`. Do not remove that gate to make a release green.

### Implementation validation record (2026-10-27)

All npm installs used the committed lockfiles (`npm ci` at root, in `frontend`,
and in `mcp-server`). Rust commands on the development host used temporary
`LIBRARY_PATH`/`LD_LIBRARY_PATH` settings pointing to an extracted `libpq` package;
no system/production database configuration was changed.

| Exact command | Result |
| --- | --- |
| `python3 scripts/contracts/check.py` | PASS; 245 inventoried operations, 39 modeled operations, 206 explicit gaps |
| `python3 scripts/contracts/generate.py --check` | PASS; generated types and compiled-route expectations reproducible |
| `python3 -m unittest discover -s scripts/contracts -v` | PASS; 39 positive/negative tests, including historical plans and process exit codes |
| `python3 scripts/contracts/check.py --require-complete` | EXPECTED FAIL (exit 1); incomplete producer contract plus 206 unverified operations |
| `python3 scripts/contracts/compatibility.py --manifest /tmp/marreq-nonexistent-plan.json` | EXPECTED FAIL (exit 1); structured missing-evidence error |
| `cargo test --locked -p marreq-cloud --test api_contract_inventory` | PASS; compiled Server and Cloud routes match derived expectations |
| `scripts/contracts/test-backend.sh` | PASS; disposable migrated PostgreSQL, production Rocket handlers, real Server HTTP process and actual frontend/MCP clients; 15 validated exchanges, 33 explicitly inventory-only exchanges, three corrupted-traffic negative controls rejected |
| `cargo test --workspace` | PASS; 2,061 passed, one ignored. Default workspace feature unification enables test helpers; DB-specific tests without explicit test DB may skip internally. The separate isolated production-path run above is required evidence. |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `npm run build --prefix frontend` | PASS; existing large-chunk warning |
| `npm run build --prefix mcp-server` | PASS |
| `npm test --prefix mcp-server` | PASS; 51 tests |
| `npm run test:coverage --prefix frontend` | PASS; 108 suites, 704 tests, 70.36% line coverage. An earlier ordinary full run had one report-builder failure; its isolated rerun and full coverage rerun passed. |
| `npx vitest run marreq-core/tests/js` | PASS; 411 tests, seven skipped |
| `npx --no-install stylelint 'frontend/static/**/*.css' --config .stylelintrc.json` | PASS |
| `npm run check:unused-css` | PASS (exit 0); existing warning reports 379 unused selectors |
| `npm audit --prefix frontend --omit=dev --audit-level=high` | PASS; no production vulnerabilities reported |
| `npm audit --prefix mcp-server --omit=dev --audit-level=high` | FAIL; existing lockfile reports one high and three moderate advisories |
| `cargo deny check advisories` | FAIL; existing locked dependencies trigger RUSTSEC-2026-0384, RUSTSEC-2026-0385 and RUSTSEC-2026-0387, plus yanked-crate warnings |
| `bash -n scripts/check-api-compatibility.sh scripts/contracts/test-backend.sh` | PASS |
| `git diff --check` | PASS |

Dependency/security failures are reported, not suppressed or allowlisted. Their
remediation remains necessary before release readiness. The source-review digest
is deliberately conservative: backend service/model/guard changes and direct
frontend calls outside API modules require renewed review as well. This does not
replace the remaining serialization/consumer verification work.

Changed files in this draft implementation (including generated artifacts):

- `.github/workflows/api-contracts.yml`
- `.github/workflows/marreq-ci.yml`
- `.github/workflows/release-artifacts.yml`
- `.gitignore`
- `Cargo.lock`
- `docker/Dockerfile`
- `docker/frontend/Dockerfile`
- `docs/developer/api-contract-routes.json`
- `docs/developer/api-contract-source-review.json`
- `docs/developer/http-api-contract.md`
- `docs/developer/mcp-setup.md`
- `docs/developer/openapi.yaml`
- `docs/developer/use-cases-api.md`
- `docs/releases.md`
- `frontend/src/api/generated-contract.ts`
- `frontend/src/api/types.ts`
- `frontend/src/hooks/__tests__/useBuildInfo.test.ts`
- `frontend/src/pages/__tests__/AccountPage.test.tsx`
- `frontend/src/pages/__tests__/AdminPage.test.tsx`
- `frontend/src/pages/__tests__/HelpPage.test.tsx`
- `frontend/src/pages/__tests__/LoginPage.test.tsx`
- `marreq-cloud/Cargo.toml`
- `marreq-cloud/tests/api_contract_inventory.rs`
- `marreq-core/tests/delegated_oauth_mcp_tools_postgres_integration.rs`
- `mcp-server/Dockerfile`
- `mcp-server/compatibility.json`
- `mcp-server/src/client.ts`
- `mcp-server/src/generated-contract.ts`
- `mcp-server/src/remote.ts`
- `scripts/check-api-compatibility.sh`
- `scripts/contracts/bundle.py`
- `scripts/contracts/check.py`
- `scripts/contracts/check_baseline.py`
- `scripts/contracts/compatibility.py`
- `scripts/contracts/generate.py`
- `scripts/contracts/inventory.py`
- `scripts/contracts/requirements.txt`
- `scripts/contracts/smoke-consumers.mjs`
- `scripts/contracts/test-backend.sh`
- `scripts/contracts/test_compatibility.py`
- `scripts/contracts/test_contract_tooling.py`
- `scripts/contracts/verify_transcript.py`
