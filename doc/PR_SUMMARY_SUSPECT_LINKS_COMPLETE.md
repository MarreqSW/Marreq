# PR: Suspect links – triggering metadata, impacted tests API, baseline snapshot

## Summary

Completes the suspect-link feature with: **triggering metadata** (which version/user caused a link to be marked suspect), **marking links suspect on approval transition** (draft→reviewed, reviewed→approved), an **impacted-tests API**, **baseline snapshot of suspect state**, and updates to **init/setup scripts** so new and reset DBs get the full schema.

## Changes

### Database

- **Migration `2026-02-09-000001_matrix_triggering_metadata`**
  - `matrix`: `triggering_version_id` (FK to `requirement_versions`), `triggering_user_id` (FK to `users`), both nullable.
  - Populated when a link is marked suspect (edit or approval); supports audit and version-scoped impacted queries.
- **Migration `2026-02-09-000002_baseline_traceability_suspect`**
  - `baseline_traceability`: `suspect`, `suspect_at`, `suspect_reason` so the baseline captures whether each link was suspect at creation time.
- **Scripts**
  - `scripts/init_complete.sql`: matrix table now includes `triggering_version_id` and `triggering_user_id`; `__diesel_schema_migrations` records the matrix triggering migration.
  - `scripts/setup_database.sh`: cleanup drops baseline tables before re-init; verification checks for `triggering_version_id` on matrix.
  - `scripts/apply_baselines_migration.sql`: `baseline_traceability` created with suspect columns; migration table records the baseline suspect migration.

### Backend

- **Mark suspect on approval**
  - When `set_requirement_version_approval` runs (draft→reviewed or reviewed→approved), all matrix links for that requirement are marked suspect with reason `"Approval state changed"` and the version id / user id stored.
- **Triggering metadata**
  - `mark_links_suspect_for_requirement(requirement_id, reason, triggering_version_id, triggering_user_id)` on `MatrixRepository` (Diesel, mock, cache).
  - Requirement **edit**: after creating a new version, links are marked suspect with the new `current_version_id` and actor id.
  - Requirement **approval**: links marked suspect with the version id and approving user id.
- **Impacted tests**
  - `TestsCaseRepository::get_impacted_tests_for_requirement(requirement_id)` returns tests linked to the requirement that are currently suspect.
  - `RequirementService::get_impacted_tests(requirement_id)`.
  - **API**: `GET /api/requirements/<id>/impacted_tests` returns JSON array of test objects (requires auth; requirement must exist).
- **Baseline creation**
  - When creating a baseline, each row in `baseline_traceability` now stores `suspect`, `suspect_at`, and `suspect_reason` from the live matrix at that time.

### Models and call sites

- `MatrixLink`: added `triggering_version_id`, `triggering_user_id`.
- `NewMatrixLink`: same (optional at insert).
- `BaselineTraceability` and `NewBaselineTraceability`: added `suspect`, `suspect_at`, `suspect_reason`.
- All constructions of `MatrixLink` / `NewMatrixLink` and baseline traceability updated in src and tests.

### Tests

- `set_requirement_version_approval_marks_links_suspect`: after approval transition, the requirement’s matrix links are suspect with reason `"Approval state changed"`.
- Existing suspect and clear_suspect tests updated for the new `mark_links_suspect_for_requirement` signature and matrix/baseline fields.

## How to test

1. **Migrations**  
   `diesel migration run` (or use `./scripts/setup_database.sh` for a full reset with updated init_complete.sql).
2. **Mark suspect on edit**  
   Link a requirement to a test, edit and save the requirement → link becomes suspect; in DB, `triggering_version_id` and `triggering_user_id` are set.
3. **Mark suspect on approval**  
   With a linked requirement in draft, transition to reviewed or approved → link becomes suspect with reason `"Approval state changed"`.
4. **Impacted tests API**  
   `GET /api/requirements/<requirement_id>/impacted_tests` (with auth) returns tests that are linked and currently suspect.
5. **Baseline snapshot**  
   Create a baseline while some links are suspect; inspect `baseline_traceability` (or baseline traceability API) and confirm `suspect`, `suspect_at`, `suspect_reason` are stored.

## Checklist

- [x] Matrix triggering metadata (migration, schema, repo, edit + approval paths)
- [x] Mark links suspect on approval transition
- [x] Impacted-tests repo method and API
- [x] Baseline traceability suspect columns and creation logic
- [x] init_complete.sql, setup_database.sh, apply_baselines_migration.sql updated
- [x] Tests updated; new test for approval→suspect
- [x] `./run_checks.sh` passes

## Labels

`tier-1`, `compliance`, `traceability`, `api`, `database`
