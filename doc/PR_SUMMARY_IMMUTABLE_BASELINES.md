# PR: Immutable project baselines

## Summary

Adds **immutable project baselines**: snapshots of current requirement versions and traceability at a point in time. Baselines reference `requirement_versions` (not live requirements) and cannot be changed after creation.

## What’s included

### Schema & migrations
- **`baselines`**: `id`, `project_id`, `name`, `description`, `created_at`, `created_by`
- **`baseline_requirements`**: `baseline_id`, `requirement_id`, `version_id` (snapshot of which version was current)
- **`baseline_traceability`**: `baseline_id`, `requirement_id`, `test_id` (snapshot of matrix)
- **Immutability**: DB triggers prevent `UPDATE`/`DELETE` on these tables
- **Migration**: `2026-02-08-000001_immutable_baselines` (up/down)
- **Script**: `scripts/apply_baselines_migration.sql` for DBs where the full migration chain cannot be run (e.g. schema already from `init_complete.sql`)

### Backend
- **Rust models**: `Baseline`, `BaselineRequirement`, `BaselineTraceability`, `NewBaseline`, insertables
- **Repository**: `BaselineRepository` — create baseline (snapshot versions + matrix), list by project, get by id, get requirements/traceability for baseline
- **Service**: `BaselineService` (thin wrapper)
- **Baseline creation**: In one transaction, inserts baseline row, then all current `(requirement_id, current_version_id)` into `baseline_requirements`, then all project matrix rows into `baseline_traceability`

### API
- **POST** `/api/projects/<project_id>/baselines` — create (body: `name`, `description`; `created_by` = current user)
- **GET** `/api/projects/<project_id>/baselines` — list
- **GET** `/api/projects/<project_id>/baselines/<baseline_id>` — get one
- **GET** `/api/projects/<project_id>/baselines/<baseline_id>/requirements` — requirements in baseline (from snapshot)
- **GET** `/api/projects/<project_id>/baselines/<baseline_id>/traceability` — traceability in baseline

### Export
- **ReqIF**: `GET /p/<project_id>/export_reqif` (current project) and `GET /p/<project_id>/export_reqif?baseline_id=<id>` (baseline snapshot)
- **ReqIFService**: `export_project(project_id)`, `export_baseline(project_id, baseline_id)`

### UI
- **Baselines list**: `/p/<project_id>/baselines` — list baselines, “New baseline”, “View”, “Export ReqIF” per row
- **New baseline**: `/p/<project_id>/baselines/new` — form (name, optional description)
- **View baseline**: `/p/<project_id>/baselines/<baseline_id>` — metadata, requirements table, traceability list, “Export ReqIF”
- **Entry points**: Project detail quick actions, project dashboard card, nav “Baselines”, requirements page export dropdown (“ReqIF (from baseline…)”)
- **Route fix**: `show_baseline` has `rank = 2` to avoid collision with `.../baselines/new`

## How to test

1. **Apply migration** (if DB already has schema):  
   `psql "$DATABASE_URL" -f scripts/apply_baselines_migration.sql`  
   Or run `diesel migration run` if no earlier migration conflicts.
2. Open a project → **Baselines** → **New baseline** → enter name (and optional description) → Create.
3. On the list, use **View** and **Export ReqIF** for a baseline.
4. From Requirements, use **Export → ReqIF (from baseline…)** to open the baselines list and export a baseline.

## Checklist

- [x] Migrations (up/down) and standalone apply script
- [x] Schema, models, repository (diesel + cache + mock), service
- [x] Immutability enforced in DB (triggers)
- [x] API: create, list, get, get requirements, get traceability
- [x] ReqIF export for baseline
- [x] UI: list, create form, view baseline; entry points
- [x] Route collision resolved (`rank = 2` on view route)
- [x] `cargo fmt` applied; tests passing
