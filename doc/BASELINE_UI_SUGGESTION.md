# Baseline UI – Where and How to Add

## Recommended approach: Dedicated "Baselines" section

Add a **Baselines** area under the project (like Requirements, Tests, Matrix, Members) so users can:

1. **List** existing baselines (name, description, created date, created by).
2. **Create** a new baseline (form: name + optional description).
3. **Use** a baseline: view snapshot requirements and/or export as ReqIF.

Baselines are immutable (no edit/delete in UI).

---

## 1. Where to add entry points

### A. Project detail – Quick Actions

**File:** `templates/project_detail.html.hbs`

Add a new button in the "Quick Actions" card (around line 60), e.g. after "View Matrix":

```handlebars
<a href="/p/{{selected_project_id}}/baselines" class="btn btn-outline-primary">
    <i class="fas fa-camera"></i> Baselines
</a>
```

(Use `project.project_id` if that is what the page exposes; the project detail uses both `project.project_id` and `selected_project_id`.)

### B. Nav dropdown (optional)

**File:** `templates/partials/nav.html.hbs`

In the project dropdown (where "Import ReqIF" lives), add:

- **View Baselines** → `/p/{{project.id}}/baselines` (and same with `selected_project_id` when no `project.id`).
- Optionally **Create baseline** → `/p/{{project.id}}/baselines/new` (or you can keep creation only from the baselines list page).

### C. Requirements page – Export dropdown

**File:** `templates/requirements/_view_controls.html.hbs`

- Keep **ReqIF** as "current project" export: `/p/{{project.id}}/export_reqif`.
- Add a way to **export a baseline** as ReqIF:
  - Either add a second item "ReqIF (from baseline…)" that links to the baselines list (or a modal), from where the user picks a baseline and gets `/p/{{project.id}}/export_reqif?baseline_id=<id>`.
  - Or add a submenu "Export → ReqIF (current)" and "Export → ReqIF (from baseline…)" that goes to `/p/{{project.id}}/baselines` with a note that they can export from there.

Minimal change: add one link "Export from baseline" → `/p/{{project.id}}/baselines` (user chooses baseline there, then uses "Export ReqIF" per baseline).

---

## 2. New routes and URLs

| URL | Purpose |
|-----|--------|
| `GET /p/<project_id>/baselines` | List baselines for the project |
| `GET /p/<project_id>/baselines/new` | Form to create a new baseline |
| `POST /p/<project_id>/baselines/new` | Submit create (then redirect to list) |
| `GET /p/<project_id>/baselines/<baseline_id>` | Optional: view baseline (e.g. snapshot requirements + traceability) |
| `GET /p/<project_id>/export_reqif?baseline_id=<id>` | Already exists – export that baseline as ReqIF |

---

## 3. New backend module (HTML routes)

**New file:** `src/routes/html/project/baselines.rs`

- `show_baselines(project_id)` – render list template; load baselines via `BaselineService::list_by_project(project_id)`; pass `user`, `projects`, `selected_project_id`, `baselines`, `page_title`.
- `new_baseline_form(project_id)` – render "new baseline" template (name, description).
- `post_baseline(project_id, Form<CreateBaselineForm>)` – validate; call `BaselineService::create_baseline(project_id, user.id, &payload)`; redirect to `show_baselines(project_id)` on success.
- Optional: `show_baseline(project_id, baseline_id)` – load baseline + requirements + traceability and render a read-only "view baseline" page.

**Form struct** (in that module or a shared forms module): e.g. `CreateBaselineForm { name: String, description: Option<String> }` with `FromForm`; map to `NewBaseline` when calling the service.

**Register in:** `src/routes/html/project/mod.rs`  
- `mod baselines;`  
- `routes.extend(baselines::routes());`

**Prelude:** Ensure `BaselineService` (and if needed `BaselineRepository` via repo) is available; use existing `get_accessible_projects`, `ProjectAccess`, etc., like in `categories.rs`.

---

## 4. New templates

**Directory:** `templates/baselines/`

### List page – `baselines.html.hbs`

- Same layout pattern as `categories/categories.html.hbs`: page title "Baselines", button **New baseline** → `/p/{{selected_project_id}}/baselines/new`.
- List of baselines (card or list-group). Each row:
  - **Name**, **Description** (if present), **Created** (date), **Created by** (user name if you pass it).
  - Actions (no Edit/Delete):
    - **View** → `/p/{{selected_project_id}}/baselines/{{id}}` (if you add the view route).
    - **Export ReqIF** → `/p/{{selected_project_id}}/export_reqif?baseline_id={{id}}`.
- Empty state: "No baselines yet. Create one to snapshot current requirements and traceability."

### Create form – `new_baseline.html.hbs`

- Same layout pattern as `categories/new_category.html.hbs`.
- Form fields:
  - **Name** (required), **Description** (optional, textarea).
- Form: `POST /p/{{selected_project_id}}/baselines/new`.
- Buttons: **Create baseline**, **Cancel** (back to list).
- Short note: "This will snapshot all current requirement versions and the traceability matrix. The baseline cannot be changed later."

### Optional – View baseline – `baseline.html.hbs`

- Show baseline metadata (name, description, created_at, created_by).
- Table (or list) of requirements in the snapshot (from `get_requirements_for_baseline`).
- Table of traceability links (from `get_baseline_traceability`).
- Link: **Export ReqIF** → `export_reqif?baseline_id=...`.

---

## 5. Data to pass from backend

- **List:** `baselines: Vec<Baseline>`. Optionally resolve `created_by` to user names (e.g. via `UserRepository::get_user_by_id`) and pass `created_by_name` or a map `baseline_id -> user_name` so the template can show "Created by Alice".
- **New form:** Only `selected_project_id`, `projects`, `user`, `page_title`.
- **View (if implemented):** `baseline`, `requirements` (snapshot), `traceability`, plus project/user context.

---

## 6. Summary checklist

- [ ] Add **Baselines** link in `project_detail.html.hbs` (Quick Actions).
- [ ] Optionally add **View Baselines** / **Create baseline** in `partials/nav.html.hbs` (project dropdown).
- [ ] Add **Export from baseline** in `requirements/_view_controls.html.hbs` (link to baselines list or baseline-specific export).
- [ ] Create `src/routes/html/project/baselines.rs` with list, new form, POST create, and optional view.
- [ ] Register `baselines` module and routes in `project/mod.rs`.
- [ ] Create `templates/baselines/baselines.html.hbs` (list).
- [ ] Create `templates/baselines/new_baseline.html.hbs` (create form).
- [ ] Optional: view route + `templates/baselines/baseline.html.hbs`.
- [ ] Use existing `BaselineService` and `export_reqif?baseline_id=`; no API changes required for the UI.

This keeps baselines visible next to other project features and makes creating a baseline and exporting it as ReqIF straightforward from the UI.
