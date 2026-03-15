# Testing the requirement version approval workflow

This guide explains how to test the **approval workflow** for requirement versions as a user. The feature is exposed via the **REST API**; the web UI does not yet have approval buttons, so you use the API (e.g. curl or browser DevTools) after logging in.

## What the feature does

- Each **requirement version** has an **approval state**: `draft` → `reviewed` → `approved`.
- **Transitions**: only `draft`→`reviewed` and `reviewed`→`approved` are allowed (no backwards steps).
- **Who can approve**: project **Owner** (role 1), **Manager** (role 2), or any **admin**. Contributors and viewers get 403.
- **Baselines**: when you create a baseline, **all** requirements in the project are included (current version snapshot). Approval state is tracked for workflow/reporting but does not filter baseline contents.

## Prerequisites

1. **App and DB running**  
  - Database set up (e.g. `./scripts/db_setup.sh --seed`)  
   - App: `cargo run --bin marreq` (or your usual run)

2. **Logged-in session**  
   - Open http://localhost:8000 (or your base URL) and log in (e.g. **alice** / **ChangeMe123!**).  
   - You need a user that is **Owner** or **Manager** of the project (or admin).  
   - Default data: **alice** is Owner of project 2 (Marreq Project), **admin** is Manager of project 2.

## Step 1: Get requirement and version IDs

List requirements (optional, to see `approval_state` and `current_version_id`):

```bash
curl -s -b cookies.txt -c cookies.txt \
  'http://localhost:8000/api/requirements' | jq '.[0] | {id, current_version_id, approval_state}'
```

List versions for a specific requirement (replace `REQ_ID` with the requirement id, e.g. `1`):

```bash
curl -s -b cookies.txt \
  'http://localhost:8000/api/requirements/REQ_ID/versions' | jq '.[0] | {id, requirement_id, approval_state}'
```

Example: for requirement `1`, you might get `version_id = 1`. Use that `requirement_id` and `version_id` in the next steps.

**Getting a session cookie (cookies.txt)**  
- Log in once in the browser, then in DevTools → Application → Cookies copy the session cookie name and value, and create `cookies.txt` in Netscape format, e.g.:

  ```text
  # Netscape HTTP Cookie File
  localhost	FALSE	/	FALSE	0	marreq_session	<value-from-browser>
  ```

  Or use the browser’s “Copy as cURL” for a request after login and reuse the `Cookie` header in curl with `-H "Cookie: ..."`.

## Step 2: Set version to “reviewed”

Only **Owner**, **Manager**, or **admin** can call this. Replace `REQ_ID` and `VERSION_ID` (e.g. `1` and `1`):

```bash
curl -s -X PUT -b cookies.txt \
  -H 'Content-Type: application/json' \
  -d '{"state":"reviewed"}' \
  'http://localhost:8000/api/requirements/REQ_ID/versions/VERSION_ID/approval' | jq .
```

You should get back the updated version JSON with `"approval_state": "reviewed"` and `approved_by`/`approved_at` still null.

## Step 3: Set version to “approved”

Same endpoint, different state:

```bash
curl -s -X PUT -b cookies.txt \
  -H 'Content-Type: application/json' \
  -d '{"state":"approved"}' \
  'http://localhost:8000/api/requirements/REQ_ID/versions/VERSION_ID/approval' | jq .
```

Response should show `"approval_state": "approved"` and `approved_by` / `approved_at` set.

## Step 4: Verify baselines include requirements

1. **Create a baseline** (project 2 example):

   ```bash
   curl -s -X POST -b cookies.txt \
     -H 'Content-Type: application/json' \
     -d '{"name":"Test baseline","description":"After approval"}' \
     'http://localhost:8000/api/projects/2/baselines' | jq .
   ```

2. **Get baseline requirements** (use the returned `id` as `BASELINE_ID`):

   ```bash
   curl -s -b cookies.txt \
     "http://localhost:8000/api/projects/2/baselines/BASELINE_ID/requirements" | jq 'length'
   ```

   - The baseline includes **all** requirements in the project (current version at creation time). The count should match the number of requirements in the project.
   - Each requirement in the response includes its `approval_state` (draft, reviewed, or approved) and optional `approved_by` / `approved_at`.

## Step 5: Test authorization (optional)

- Log in as a **Contributor** or **Viewer** (e.g. **eng_jones** or **qa_wilson** for project 1).  
- Call the same `PUT .../approval` with `"state":"reviewed"`.  
- You should get **403 Forbidden** with a message that only project owners or managers can approve.

## Step 6: Test invalid transitions (optional)

- With an **approved** version, try:

  ```bash
  curl -s -X PUT -b cookies.txt \
    -H 'Content-Type: application/json' \
    -d '{"state":"reviewed"}' \
    'http://localhost:8000/api/requirements/REQ_ID/versions/VERSION_ID/approval'
  ```

  You should get **400** (or equivalent) because going back from `approved` to `reviewed` is not allowed.

- Try `"state":"draft"` or `"state":"invalid"`: should be rejected (invalid state or invalid transition).

## Quick reference

| Item | Value |
|------|--------|
| Endpoint | `PUT /api/requirements/<req_id>/versions/<version_id>/approval` |
| Body | `{"state": "reviewed"}` or `{"state": "approved"}` |
| Auth | Session cookie (log in via web UI first) |
| Allowed roles | Project Owner (1), Manager (2), or admin |
| Transitions | draft → reviewed → approved only |
| Baselines | Include all project requirements (current version snapshot) |

## Seeing approval state in the UI

- **Requirement list/detail**: If the UI loads requirement data from the API (e.g. `/api/requirements/<id>`), the response already includes `approval_state`, `approved_by`, and `approved_at` for the current version. You can show them in the template or in browser DevTools when inspecting the API response.
- **Version history**: `GET /api/requirements/<id>/versions` returns each version with its `approval_state`, `approved_by`, and `approved_at`.

To **change** approval state from the UI you still need to call the API (e.g. from a custom “Approve” / “Mark reviewed” button that sends the PUT request above).
