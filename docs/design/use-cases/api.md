# Use cases: API design (RFC for #388)

Status: **proposal**, to be discussed on #388. It follows the UX prototype posted there (screens 1–9) and the conventions of the existing API (`docs/developer/http-api-contract.md`). Nothing here is implemented.

## 1. Design decisions

| # | Decision | Why |
|---|---|---|
| D1 | A use case's content is **one JSON document per version**. Every element in it carries a **stable id**. | Versions stay immutable and cheap to baseline (a baseline points at a version row, like requirements). Diffs are computed by element id, so *moved*, *rebranched* and *edited* can be told apart. |
| D2 | **Display numbers are never stored.** The server computes labels (`3`, `A3`, `A3.1`, `E4`) from order and branch origin, and returns them with every document. | References survive reordering ("references must survive reordering"). |
| D3 | **Links and comments are relational rows** anchored to `(use_case, element_id)`. `element_id` is `null` for the whole use case. | Reverse lookups ("which use cases use REQ-EXP-004?"), suspect flags and baselines need SQL, not JSON scans. |
| D4 | **One shared working draft per use case**, separate from versions. Edits change the draft. **Publish** turns the draft into a new immutable version. | Editing must not create a version per keystroke (issue §3). This differs from requirements, where every PATCH is a version. |
| D5 | The draft has a **revision counter**. Every write sends the revision it is based on. A stale write gets **409** with the current draft. | Marreq has no optimistic concurrency today. A structured document edited by two people needs it. There is no real-time co-editing (out of scope). |
| D6 | The UI edits the draft with **operations** (`add_step`, `move`, `rebranch`, …), validated by the server. A whole-document `PUT` exists for MCP and import. | Small payloads, server-enforced invariants, and undo maps to inverse operations. Agents can still write a whole document. |
| D7 | **Approval reuses the requirement model:** `draft → reviewed → approved` per version, the same reviewer pool, the same `/approval` endpoint shape. | No new concepts for reviewers. `may_change_review_gates` and notifications are reused. |
| D8 | **Permissions reuse the existing ones:** `ViewRequirements` reads, `EditRequirements` writes, the reviewer pool approves. Archived projects are read-only. | No new role matrix. |
| D9 | **New delegated scopes:** `use_cases:read`, `use_cases:write`, `use_cases:approve`. | Matches the existing per-artifact scopes. MCP tools declare them. |
| D10 | **Linking a verification to a use case never touches the matrix.** It does not verify the use case's requirements. | Required by the issue. |
| D11 | Project-scoped routes only (`/api/projects/{project_id}/use-cases…`), with no legacy unscoped duplicates. | Same as baselines and custom fields. |

## 2. Data model

```text
use_cases
  id, project_id, stable_code (unique per project), current_version_id NULL,
  created_by, created_at, mcp_idempotency_identity CHAR(64) UNIQUE NULL

use_case_versions                      -- immutable
  id, use_case_id, version_number (1, 2, …), document JSONB,
  title, primary_actor                 -- copied from the document for lists and search
  approval_state ('draft'|'reviewed'|'approved'), reviewed_by/at, approved_by/at,
  change_note NULL, created_by, created_at, search_vector

use_case_drafts                        -- at most one per use case
  use_case_id PK, base_version_id NULL, document JSONB, revision INT,
  updated_by, updated_at

use_case_links
  id, project_id, use_case_id, element_id VARCHAR(40) NULL,
  requirement_id NULL | verification_id NULL          -- exactly one (CHECK)
  linked_requirement_version_id NULL, rationale NULL,
  suspect, suspect_at, suspect_reason, triggering_version_id, triggering_user_id,
  cleared_by, cleared_at, created_by, created_at
  UNIQUE (use_case_id, element_id, requirement_id, verification_id)

use_case_comments                      -- immutable, like requirement comments
  id, use_case_id, use_case_version_id NULL, element_id NULL,
  author_id, body, created_at, mcp_idempotency_identity

baseline_use_cases       (baseline_id, use_case_id, version_id)
baseline_use_case_links  (baseline_id, use_case_id, element_id, requirement_id, verification_id, suspect)
```

Requirements and verifications link to the **artifact**, not a version, as the matrix does. `linked_requirement_version_id` records which requirement version the link was made or last cleared against. That is what makes "REQ-EXP-007 changed since it was linked (v3 → v4)" possible.

## 3. The document (`marreq.use-case.v1`)

```json
{
  "schema": "marreq.use-case.v1",
  "title": "Export requirements",
  "primary_actor": "Project member",
  "secondary_actors": ["Storage service"],
  "summary": "A member exports a selection of requirements to a file.",
  "preconditions":  [{ "id": "p_7Kq2", "text": "User is authenticated." }],
  "main": [
    { "id": "s_A1x9", "text": "User opens the requirements page." },
    { "id": "s_Bf20", "text": "System displays the available requirements." },
    { "id": "s_C3mz", "text": "User selects requirements and requests an export." },
    { "id": "s_D8pw", "text": "System generates the file in the chosen format." }
  ],
  "branches": [
    {
      "id": "b_N0s1", "kind": "alternative", "title": "Nothing selected",
      "from_step": "s_C3mz",
      "steps": [{ "id": "s_Q4ee", "text": "System asks the user to select at least one requirement." }],
      "end": { "type": "rejoin", "step": "s_C3mz" }
    },
    {
      "id": "b_F4il", "kind": "exception", "title": "Export fails",
      "from_step": "s_D8pw",
      "steps": [{ "id": "s_R7kk", "text": "System reports the error and keeps the selection." }],
      "end": { "type": "end", "outcome": "failure" }
    }
  ],
  "postconditions": [{ "id": "q_2Lm0", "text": "The export file is available to the user." }]
}
```

### Invariants (checked on every write; a violation returns **422**)
1. Ids match `^[pqsb]_[A-Za-z0-9]{4,32}$`. The prefix tells the kind: precondition, postcondition, step, branch.
   - Ids are unique within the use case **across all its versions**; a deleted id is never reused.
   - Clients may generate ids, so the UI can update optimistically. The server rejects duplicates.
2. `branches[].from_step` must be a step in `main`. Version 1 has no branches from branch steps; that can be relaxed later without a format change.
3. `end` is either `{"type":"rejoin","step":<main step id>}` or `{"type":"end","outcome":"success"|"failure"}`.
4. The title is 3–255 characters. Every `text` is 1–2000 characters of the existing statement Markdown subset (`rich_text.rs`).
5. `main` has at least 1 step once published (an empty draft is allowed). Total elements ≤ 500.
6. Unknown keys are rejected. That keeps v1 strict, so v2 can add fields safely.

### Labels (computed, returned alongside the document)
```json
"labels": { "s_C3mz": "3", "b_N0s1": "A3", "s_Q4ee": "A3.1", "b_F4il": "E4", "s_R7kk": "E4.1" }
```
- Main steps are numbered 1…n.
- A branch is `A` (alternative) or `E` (exception) followed by its origin step's number. A second branch of the same kind from the same step gets a letter: `A3`, `A3b`, `A3c`.
- Branch steps are numbered `<branch label>.<n>`.

## 4. Endpoints

All of them are under `/api/projects/{project_id}`. They take the session cookie (writes need `X-CSRF-Token`), a personal API token, or a delegated OAuth token with the scope shown. **Writes** need `EditRequirements` and are refused on archived projects (403). Errors use the standard `{status, error, message}` body.

### 4.1 Use cases
| Method | Path | Scope | Notes |
|---|---|---|---|
| `GET` | `/use-cases` | read | List. Query params: `q` (code, title, actor), `approval_state`, `actor`, `has_suspect=true`, `requirement_id`, `verification_id`, `sort_column` (`code`, `title`, `actor`, `approval`, `modified`), `sort_dir`. Rows: `id, reference_code, title, primary_actor, step_count, alternative_count, exception_count, link_count, suspect_count, approval_state, current_version_number, has_draft, updated_at`. |
| `GET` | `/use-cases/next-code` | read | `{ "reference_code": "UC-008" }`, a suggestion only. |
| `POST` | `/use-cases` | write | Body: `{reference_code?, title, primary_actor?, document?}`. Creates the use case and its **draft** (no version yet). If the code is omitted, the server takes the next free `UC-NNN`. Codes follow the requirement rule `^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$`, unique per project (409). `Idempotency-Key` is supported. Returns **201** with the detail. |
| `GET` | `/use-cases/{id}` | read | Detail: `{id, reference_code, current_version: {id, version_number, approval_state, document, labels, created_by, created_at}, draft: {revision, updated_by, updated_at, base_version_id} \| null, link_counts, suspect_count, comment_count}`. `?version_id=` reads an older version. |
| `DELETE` | `/use-cases/{id}` | write | **204**. Removes the draft, versions, links and comments. A use case that is in a baseline returns **409**, because baselines keep pointing at versions. |

### 4.2 Draft (editing)
| Method | Path | Scope | Notes |
|---|---|---|---|
| `GET` | `/use-cases/{id}/draft` | read | `{revision, base_version_id, document, labels, updated_by, updated_at}`, or **404** if there is no draft. |
| `PUT` | `/use-cases/{id}/draft` | write | Body: `{base_revision, document}`. Replaces the whole draft, or creates it from scratch (`base_revision: null`). For MCP, import and power users. |
| `POST` | `/use-cases/{id}/draft/operations` | write | Body: `{base_revision, operations: [...]}`. Applies the operations **atomically**, in order (§5). Returns `{revision, document, labels, effects}`. |
| `DELETE` | `/use-cases/{id}/draft` | write | Discards the draft. **204**. |
| `GET` | `/use-cases/{id}/draft/diff` | read | Structural diff, draft against its base version (§6). |
| `POST` | `/use-cases/{id}/draft/publish` | write | Body: `{base_revision, change_note?}`. Validates the publish invariants, then creates version N+1 in `draft` approval state and deletes the draft. Returns **201** with `{version, removed_links: [...]}` (links whose element no longer exists are removed and reported). Notifies reviewers, like moving a requirement to reviewed. `Idempotency-Key` is supported. |

The **first write that opens a draft** (an operation or PUT when none exists) copies the current version's document. In that case `base_revision` must be `null`.

**Conflict:** a stale `base_revision` returns **409** with `{status, error, message, current: {revision, updated_by, updated_at}}`. The client refetches, and replays its pending operations if they still apply.

### 4.3 Versions and approval
| Method | Path | Scope | Notes |
|---|---|---|---|
| `GET` | `/use-cases/{id}/versions` | read | `[{id, version_number, approval_state, change_note, created_by, created_at, reviewed_by/at, approved_by/at}]`, newest first. |
| `GET` | `/use-cases/{id}/versions/{vid}` | read | One version with `document` and `labels`. |
| `PUT` | `/use-cases/{id}/versions/{vid}/approval` | approve | `{state: "reviewed"\|"approved"}`, with the same transitions and reviewer-pool rule as requirements (`require_project_reviewer`). Returns the version. Moving to `approved` marks this use case's links' `linked_requirement_version_id` as confirmed. |
| `GET` | `/use-cases/{id}/versions/{a}/diff/{b}` | read | Structural diff (§6). |

### 4.4 Links (traceability)
| Method | Path | Scope | Notes |
|---|---|---|---|
| `GET` | `/use-cases/{id}/links` | read | `[{id, element_id, element_label, target_type, requirement_id?, verification_id?, target_reference_code, target_title, rationale, suspect, suspect_reason, linked_requirement_version_id, current_requirement_version_id}]` |
| `POST` | `/use-cases/{id}/links` | write | `{element_id?, requirement_id? \| verification_id?, rationale?}`. The element must exist in the current version or the draft. **201**. Duplicates return **409**. |
| `DELETE` | `/use-cases/{id}/links/{link_id}` | write | **204** |
| `POST` | `/use-cases/{id}/links/{link_id}/clear_suspect` | write | `{status: "ok"\|"no_change", cleared}`, same shape as the matrix endpoint. Sets `linked_requirement_version_id` to the requirement's current version. |
| `GET` | `/requirements/{rid}/use-cases` | read | Reverse direction: `[{use_case_id, reference_code, title, element_id, element_label, suspect}]`. Feeds "Used in use cases" (screen 7). |
| `GET` | `/verifications/{vid}/use-cases` | read | Same, for verifications. |

**Suspect rule:** when a requirement gets a new version or its approval state changes, the same transaction that marks matrix rows suspect also marks its `use_case_links` rows suspect (`suspect_reason`, `triggering_version_id`). The hook is in `update_requirement_atomic` and the approval path.

### 4.5 Comments
| Method | Path | Scope | Notes |
|---|---|---|---|
| `GET` | `/use-cases/{id}/comments` | read | `?version_id=&element_id=`. Rows: `{id, use_case_id, use_case_version_id, element_id, element_label, author_id, author_name, body, created_at}`. |
| `POST` | `/use-cases/{id}/comments` | write | `{body, use_case_version_id?, element_id?}`. **201**. `@mentions` (#389) and `Idempotency-Key` are supported. The approved-version lock applies, as for requirements. |

Notifications reuse `comment_added` and `mentioned` with `entity_type: "use_case"`. The frontend link builder and `/api/home/attention` learn the new entity type. `approval_requested` is reused for published versions.

### 4.6 Baselines, search, export
- **Baselines:** creating a baseline also snapshots every use case's current version into `baseline_use_cases`, and its links into `baseline_use_case_links`.
  - New: `GET /baselines/{bid}/use-cases` and `GET /baselines/{bid}/use-cases/{id}/diff/current`.
  - `GET /baselines/diff` gains `use_cases_only_in_a`, `use_cases_only_in_b` and `use_cases_changed`.
- **Search:** `GET /api/search` hits gain `kind: "requirement" | "use_case"` and `use_case_id`. Use cases are searched by code, title, actor and step text (`search_vector`).
- **Bundle export and import:** a new `use_cases` section in `marreq.project-bundle.v1`. It holds the current document plus links, by natural keys (`reference_code`, `element_id`). It is optional, so older bundles still import.
- **ReqIF:** out of scope for v1.

## 5. Draft operations

Every operation names elements by id. `after: null` means "first in the list".

| Operation | Fields | Effect |
|---|---|---|
| `set_header` | `title?, primary_actor?, secondary_actors?, summary?` | Updates the header. |
| `add_element` | `list: "preconditions"\|"main"\|"postconditions"\|{"branch": id}, after, element: {id?, text}` | Inserts an element. The server generates `id` if it is missing. |
| `update_text` | `id, text` | Edits the text in place. |
| `move_element` | `id, list?, after` | Reorders within a list, or moves a step between `main` and a branch. References follow the id. |
| `remove_element` | `id, dependents?: {"rebranch_to": stepId} \| "cascade"` | If the step is a branch origin or a rejoin target and `dependents` is missing, the request returns **422** `element_has_dependents`, listing the branches, links and comments. That drives screen 6. |
| `add_branch` | `branch: {id?, kind, title, from_step, end, steps?}` | Adds an alternative or exception. |
| `update_branch` | `id, title?, kind?, from_step?, end?` | Edits a branch. A changed `from_step` shows as *rebranched* in diffs. |
| `remove_branch` | `id` | Removes the branch and its steps. |

The response `effects` lists what the server did on the client's behalf, for example `[{ "type": "relabelled", "id": "b_N0s1", "from": "A3", "to": "A2" }]`. The UI shows these in toasts (screen 4). **Undo** is the client sending the inverse operations as a new request; the server keeps no undo stack.

Example: move step 3 above step 2.
```http
POST /api/projects/1/use-cases/7/draft/operations
{ "base_revision": 12,
  "operations": [{ "op": "move_element", "id": "s_C3mz", "after": "s_A1x9" }] }
```
```json
{ "revision": 13, "document": { "…": "…" },
  "labels": { "s_C3mz": "2", "s_Bf20": "3", "b_N0s1": "A2", "s_Q4ee": "A2.1" },
  "effects": [{ "type": "relabelled", "id": "b_N0s1", "from": "A3", "to": "A2" }] }
```

## 6. Structural diff

```json
{
  "from": { "version_id": 31, "version_number": 2 },
  "to":   { "draft_revision": 13 },
  "summary": { "added": 1, "removed": 1, "edited": 2, "moved": 2, "rebranched": 1 },
  "header": { "title": { "old": "…", "new": "…" } },
  "elements": [
    { "id": "s_C3mz", "kind": "step", "changes": ["moved"],
      "old": { "list": "main", "label": "3" }, "new": { "list": "main", "label": "2" } },
    { "id": "s_D8pw", "kind": "step", "changes": ["edited"],
      "old": { "label": "4", "text": "…chosen format." }, "new": { "label": "4", "text": "…selected format and offers it for download." },
      "text_diff": [{ "op": "equal", "text": "System generates the file in the " },
                    { "op": "delete", "text": "chosen" }, { "op": "insert", "text": "selected" },
                    { "op": "equal", "text": " format" }, { "op": "insert", "text": " and offers it for download" },
                    { "op": "equal", "text": "." }] },
    { "id": "b_N0s1", "kind": "branch", "changes": ["rebranched"],
      "old": { "label": "A3", "from_step": "s_C3mz" }, "new": { "label": "A2", "from_step": "s_Bf20" } }
  ]
}
```
- Elements are matched **by id**.
  - `added` / `removed`: the id exists on one side only.
  - `edited`: the text changed. The word-level `text_diff` uses `similar`, which already powers requirement diffs.
  - `moved`: the element's position changed relative to its unchanged neighbours, not merely its number.
  - `rebranched`: `from_step` or `end` changed.
- One element can carry several changes.
- Unchanged elements are left out unless `?include_unchanged=true` is passed.

## 7. MCP tools

| Tool | Scopes | Mode |
|---|---|---|
| `list_use_cases`, `get_use_case` (with labels), `get_use_case_diff`, `list_use_case_links` | `use_cases:read` | read_only |
| `create_use_case` (idempotent) | `use_cases:write` | draft_write |
| `edit_use_case_draft`: operations, or a whole document with `base_revision` | `use_cases:write` | draft_write |
| `publish_use_case_draft` (idempotent) | `use_cases:write` | draft_write |
| `link_use_case_element` | `use_cases:write` | draft_write |

Approval is deliberately **not** exposed to agents, as with requirements.

## 8. Audit and activity
- `EntityType::UseCase` is added.
- These actions are audited:
  - create, delete, publish (with the version number and the diff summary);
  - approval transitions;
  - link added, removed or cleared.
- Draft edits are **not** audited one by one; the publish entry summarises them.
- `GET /use-cases/{id}/activity` mirrors the requirement activity endpoint.

## 9. Delivery slices (matching the issue's plan)
1. **Core:**
   - tables `use_cases`, `use_case_versions` and `use_case_drafts`;
   - the document validator and labels;
   - list, create, get and delete;
   - the draft (PUT and operations), publish, versions and approval;
   - the UI: screens 1–6.
2. **Links:** `use_case_links`, the suspect hook, the reverse endpoints and screen 7.
3. **Review:** structural diff, comments with element anchors, notifications, and screen 8.
4. **Baselines, search, bundle and MCP.**
5. **Flow view (screen 9):** frontend only, built from the same document and labels; no API needed.

## 10. Open questions for #388
1. **Actors:** free text in v1 (`primary_actor`, `secondary_actors`), or project-level actor entities from the start? Free text keeps v1 small, and a later migration can turn distinct values into entities.
2. **Branches from branch steps** (nested alternatives): needed in v1, or later?
3. **Link anchoring:** should a link to a step **pin** the use case version, or float like the matrix does (the proposal)?
4. **Deleting a use case that is in a baseline:** refuse (the proposal), or keep a tombstone?
5. **Draft visibility:** is the shared draft visible to every project member, or only to editors (`EditRequirements`)?
