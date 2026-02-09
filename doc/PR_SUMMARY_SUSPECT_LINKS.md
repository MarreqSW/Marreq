# Suspect Links (Change Impact Analysis) – PR Summary

## Summary

Implements **Suspect Links** for change impact analysis: when a requirement is changed (new version created), all downstream traceability links are automatically marked **suspect** until explicitly reviewed and cleared. Supports filtering and full audit of suspect state.

## Changes

### Database
- **Migration** `2026-02-08-000002_matrix_suspect_links`: adds to `matrix` table:
  - `suspect` (bool, default false), `suspect_at`, `suspect_reason`, `cleared_by`, `cleared_at`
- **Schema** updated in `scripts/init_complete.sql` and verified in `scripts/setup_database.sh`

### Backend
- **Repository**: `mark_links_suspect_for_requirement(requirement_id, reason)` and `clear_suspect(req_id, test_id, user_id)` on `MatrixRepository` (Diesel, mock, cache with invalidation).
- **Requirement update**: after `edit_requirement` (new version), all matrix links for that requirement are marked suspect with reason `"Requirement updated"`.
- **Matrix service**: `clear_suspect(actor, req_id, test_id)` and matrix view now include `suspect_links` and optional `suspect` filter (all / suspect only / not suspect).
- **API**: `POST /api/traceability/clear_suspect` with body `{ "req_id", "test_id" }`; `GET /api/matrix` exposes suspect fields.

### Frontend
- **Traceability matrix**: suspect links show ⚠ icon and "Clear" button; "Suspect" filter (All / Suspect only / Not suspect); legend and styling for suspect state.
- **Clear action**: form POST to `/p/<project_id>/matrix/clear_suspect`; API clear records user and timestamp for audit.

### Tests
- Repository: `mark_links_suspect_for_requirement`, `clear_suspect` (success and link missing).
- Matrix service: `clear_suspect` (cleared vs missing), matrix view with `suspect_links` and suspect-only filter.
- API: clear_suspect returns ok/cleared when link was suspect, no_change when missing, 401 without auth.
- Requirement service: `update` marks traceability links suspect.

## Acceptance Criteria (from user story)

- [x] New requirement version → linked traceability items marked suspect
- [x] Suspect state: flag, timestamp, reason (e.g. "Requirement updated")
- [x] Suspect links visible and filterable in matrix
- [x] User can clear suspect flag (UI + API)
- [x] Clearing records user and timestamp
- [x] `POST /api/traceability/clear_suspect`; matrix endpoints expose suspect state

## How to Test

1. **Apply schema**: `diesel migration run` or run `migrations/2026-02-08-000002_matrix_suspect_links/up.sql` on existing DB; or use `./scripts/setup_database.sh` for a fresh DB.
2. **Mark suspect**: Link a requirement to a test, then edit and save the requirement → link becomes suspect (⚠ in matrix).
3. **Filter**: In matrix, use "Suspect" filter (e.g. "Suspect only").
4. **Clear**: Click "Clear" on a suspect cell or call `POST /api/traceability/clear_suspect` with `{ "req_id", "test_id" }`.

## Labels

`tier-1`, `compliance`, `traceability`, `api`
