# Use cases API

Use cases (issue #398, epic #388) are a project artifact next to requirements and verifications. This page describes the HTTP API; the design discussion is on #388.

All routes are under `/api/projects/{project_id}` and accept the session cookie (writes need `X-CSRF-Token`), a personal API token, or a delegated OAuth token with the scope shown. Reads need `ViewRequirements`; writes need `EditRequirements` and are refused on archived projects (**403**); approval needs a project reviewer (an instance administrator when the project has no reviewer list).

## Model

- A **use case** has a reference code (`UC-001`, unique per project) and a series of immutable **versions** `1, 2, …`. Each version holds one JSON document and an approval state (`draft → reviewed → approved`, as for requirement versions).
- Edits go to **one shared working draft** per use case. Every write names the draft **revision** it is based on; a write based on an older revision is refused with **409** (there is no real-time co-editing). **Publishing** the draft creates the next version and removes the draft.
- A new use case starts with a draft and no version. The first edit after a publish opens a new draft from the current version.

## The document (`marreq.use-case.v1`)

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
    { "id": "s_C3mz", "text": "User selects requirements and requests an export." }
  ],
  "branches": [{
    "id": "b_N0s1", "kind": "alternative", "title": "Nothing selected",
    "from_step": "s_C3mz",
    "steps": [{ "id": "s_Q4ee", "text": "System asks the user to select at least one requirement." }],
    "end": { "type": "rejoin", "step": "s_C3mz" }
  }],
  "postconditions": [{ "id": "q_2Lm0", "text": "The export file is available to the user." }]
}
```

Rules (a violation is **422** `invalid_document`, with every broken rule in `errors: [{path, message}]`):

1. Ids match `^[pqsb]_[A-Za-z0-9]{4,32}$`; the prefix gives the kind (precondition, postcondition, step, branch) and must match the list. Ids are unique in the document. Clients may choose ids (the server generates one when an operation leaves it out). **An id that a published version dropped is never reused**, except to restore an element of the version the draft started from.
2. `branches[].from_step` and a `rejoin` step are steps of `main`. Branches do not start from branch steps.
3. `end` is `{"type": "rejoin", "step": <main step>}` or `{"type": "end", "outcome": "success" | "failure"}`; `kind` is `alternative` or `exception`.
4. The title is 3–255 characters; texts are 1–2000 characters of statement Markdown; at most 20 secondary actors and 500 elements.
5. `main` needs at least one step to publish (an empty draft is fine).
6. Unknown keys are rejected.

**Labels** are computed, never stored, and returned with every document as `labels: {id: label}`: main steps `1…n`; a branch is `A`/`E` and its origin's number (`A3`, then `A3b`, `A3c` for more branches of the same kind from the same step); branch steps `A3.1`; preconditions `P1…`, postconditions `Q1…`.

## Endpoints

| Method | Path | Scope | Description |
|---|---|---|---|
| `GET` | `/use-cases` | `use_cases:read` | Rows `id, reference_code, title, primary_actor, step_count, alternative_count, exception_count, approval_state, current_version_number, has_draft, link_count, suspect_count, updated_at`. Query: `q` (code, title, actor), `approval_state` (`draft`, `reviewed`, `approved`, or `unpublished`), `actor`, `has_suspect=true`, `requirement_id`, `verification_id` (use cases linked to it), `sort_column` (`code`, `title`, `actor`, `approval`, `modified`), `sort_dir` (`asc`, `desc`). Title and counts come from the current version, or from the draft before the first publish; `approval_state` is `null` then. |
| `GET` | `/use-cases/next-code` | `use_cases:read` | `{ "reference_code": "UC-008" }`: the highest `UC-NNN` plus one. A suggestion only. |
| `POST` | `/use-cases` | `use_cases:write` | `{reference_code?, title?, primary_actor?, document?}`; `Idempotency-Key` supported. Without a code the server takes the next `UC-NNN`; codes follow `^[A-Z]{2,4}(-[A-Z0-9]{1,6})+$` (**400**) and are unique per project (**409**). `document` starts the draft from a whole document; `title` and `primary_actor` override its header. **201** with the detail. |
| `GET` | `/use-cases/{id}` | `use_cases:read` | `{id, project_id, reference_code, title, created_by, created_at, updated_at, current_version, draft}`. `current_version` is `{id, version_number, approval_state, title, primary_actor, change_note, created_by, created_by_name, created_at, reviewed_by/at, approved_by/at, document, labels}` or `null`; `?version_id=` returns that version instead. `draft` is `{revision, base_version_id, updated_by, updated_by_name, updated_at}` or `null`. |
| `DELETE` | `/use-cases/{id}` | `use_cases:write` | **204**. Removes the versions and the draft. |
| `GET` | `/use-cases/{id}/draft` | `use_cases:read` | `{use_case_id, revision, base_version_id, updated_by, updated_by_name, updated_at, document, labels}`; **404** when there is no draft. |
| `PUT` | `/use-cases/{id}/draft` | `use_cases:write` | `{base_revision, document}`: replace the whole draft (`base_revision: null` opens one when there is none). Returns the draft plus `effects: []`. |
| `POST` | `/use-cases/{id}/draft/operations` | `use_cases:write` | `{base_revision, operations: [...]}`: apply the operations below in order, all or nothing. `base_revision: null` opens a draft from the current version. Returns the draft plus `effects`. |
| `DELETE` | `/use-cases/{id}/draft` | `use_cases:write` | **204**. Links to elements that only the draft had are deleted. **409** for a use case that was never published (delete the use case instead). |
| `POST` | `/use-cases/{id}/draft/publish` | `use_cases:write` | `{base_revision, change_note?}`. Checks the publish rules, creates the next version in approval state `draft`, makes it current and removes the draft. Links to elements the new version no longer has are deleted. **201** with `{version, removed_links}`. `Idempotency-Key` supported. |
| `GET` | `/use-cases/{id}/versions` | `use_cases:read` | Version summaries (no document), newest first. |
| `GET` | `/use-cases/{id}/versions/{version_id}` | `use_cases:read` | One version with `document` and `labels`. |
| `PUT` | `/use-cases/{id}/versions/{version_id}/approval` | `use_cases:approve` | `{state: "reviewed" \| "approved"}`. Project reviewers only; `draft → reviewed → approved`, one step at a time (**400** otherwise; the same state again is a no-op). Returns the version. |

Create, delete, publish, approval changes and link changes are audited with entity type `USE_CASE`. Draft edits are not audited one by one.

### Links

A link ties the use case, or one of its elements (a step or condition id), to a requirement or a verification of the same project. Links point at the requirement, not at a version, like matrix rows. When the requirement gets a new version or its approval state changes, its use case links become **suspect**, in the same way and transaction as its matrix rows. Linking a verification to a use case never changes the matrix: it does not verify the use case's requirements.

| Method | Path | Scope | Description |
|---|---|---|---|
| `GET` | `/use-cases/{id}/links` | `use_cases:read` | `[{id, element_id, element_label, target_type, requirement_id, verification_id, target_reference_code, target_title, rationale, suspect, suspect_at, suspect_reason, linked_requirement_version_id, current_requirement_version_id, created_by, created_at}]`. `element_id` is `null` for the whole use case; `element_label` comes from the current version, or the draft for elements only it has. |
| `POST` | `/use-cases/{id}/links` | `use_cases:write` | `{element_id?, requirement_id? \| verification_id?, rationale?}`: exactly one target (**400** otherwise). The element must exist in the current version or the draft (**400**); the target must be in the project (**404**). The same link twice is **409**. **201** with the link. |
| `DELETE` | `/use-cases/{id}/links/{link_id}` | `use_cases:write` | **204**. |
| `POST` | `/use-cases/{id}/links/{link_id}/clear_suspect` | `use_cases:write` | `{status: "ok" \| "no_change", cleared}`. Records the requirement's current version as the linked one. |
| `GET` | `/requirements/{requirement_id}/use-cases` | `use_cases:read` | Where a requirement is used: `[{use_case_id, reference_code, title, link_id, element_id, element_label, suspect}]`. |
| `GET` | `/verifications/{verification_id}/use-cases` | `use_cases:read` | The same for a verification. |

### Draft operations

Operations name elements by id. In `add_element` and `move_element`, `after` is the id to insert after; `null` means first and leaving it out means last. `list` is `"preconditions"`, `"main"`, `"postconditions"` or `{"branch": "<branch id>"}`.

| `op` | Fields | Effect |
|---|---|---|
| `set_header` | `title?, primary_actor?, secondary_actors?, summary?` | Updates the header; `null` or `""` clears the actor or summary. |
| `add_element` | `list, after?, element: {id?, text}` | Inserts an element; reports `created` with the id when the server generated it. |
| `update_text` | `id, text` | Edits an element's text. |
| `move_element` | `id, list?, after?` | Reorders within a list, or moves a step between `main` and a branch. |
| `remove_element` | `id, dependents?` | Removes an element. A main step that branches start from or rejoin at needs `dependents: "cascade"` (remove those branches) or `dependents: {"rebranch_to": "<main step>"}`; without it the request fails with `element_has_dependents`. |
| `add_branch` | `branch: {id?, kind, title, from_step, end, steps?}` | Adds an alternative or exception, placed with the other branches of its origin. |
| `update_branch` | `id, title?, kind?, from_step?, end?` | Edits a branch. |
| `remove_branch` | `id` | Removes a branch and its steps. |

`effects` tell the client what the server did on its behalf, for toasts: `{type: "relabelled", id, from, to}` for every branch whose label changed, `{type: "removed", id}` for cascaded branches, `{type: "rebranched", id, from_step}` and `{type: "created", id}`. Undo is the client sending the inverse operations.

Example: move step 3 above step 2.

```http
POST /api/projects/1/use-cases/7/draft/operations
{ "base_revision": 12, "operations": [{ "op": "move_element", "id": "s_C3mz", "after": "s_A1x9" }] }
```

```json
{ "use_case_id": 7, "revision": 13, "document": { "…": "…" },
  "labels": { "s_C3mz": "2", "b_N0s1": "A2", "s_Q4ee": "A2.1", "…": "…" },
  "effects": [{ "type": "relabelled", "id": "b_N0s1", "from": "A3", "to": "A2" }] }
```

### Errors

The body is the usual `{status, error, message}`, with a `code` and details for these:

| Status | `code` | Details |
|---|---|---|
| 409 | `stale_revision` | `current: {revision, base_version_id, updated_by, updated_by_name, updated_at}` or `null`. Reload the draft, then replay the edit if it still applies. |
| 422 | `invalid_document` | `errors: [{path, message}]` |
| 422 | `element_has_dependents` | `operation_index, element_id, element_label, branches: [{id, label, title}]` |
| 422 | `unknown_element` | `operation_index, element_id` |
| 422 | `invalid_operation` | `operation_index` |

### Structural diff

| Method | Path | Scope | Description |
|---|---|---|---|
| `GET` | `/use-cases/{id}/draft/diff` | `use_cases:read` | The draft against the version it started from; before the first publish, against an empty document. **404** when there is no draft. |
| `GET` | `/use-cases/{id}/versions/{a}/diff/{b}` | `use_cases:read` | Version `a` (before) against version `b` (after). |

Both take `?include_unchanged=true` and return:

```json
{
  "from": { "version_id": 31, "version_number": 2, "draft_revision": null },
  "to":   { "version_id": null, "version_number": null, "draft_revision": 13 },
  "summary": { "added": 1, "removed": 1, "edited": 2, "moved": 1, "rebranched": 1 },
  "header": { "title": { "old": "Export requirements", "new": "Export a selection" } },
  "elements": [
    { "id": "s_C3mz", "kind": "step", "changes": ["moved"],
      "old": { "list": "main", "label": "3", "text": "…" }, "new": { "list": "main", "label": "2", "text": "…" } },
    { "id": "s_D8pw", "kind": "step", "changes": ["edited"], "old": { "…": "…" }, "new": { "…": "…" },
      "text_diff": [{ "op": "equal", "text": "System generates the file in the " },
                    { "op": "delete", "text": "chosen" }, { "op": "insert", "text": "selected" },
                    { "op": "equal", "text": " format." }] },
    { "id": "b_N0s1", "kind": "branch", "changes": ["rebranched"],
      "old": { "list": "branches", "label": "A3", "text": "Nothing selected", "from_step": "s_C3mz", "end": { "type": "rejoin", "step": "s_C3mz" }, "kind": "alternative" },
      "new": { "…": "…" } }
  ]
}
```

Elements are matched by id. **added** / **removed**: the id is on one side only. **edited**: the text (or a branch's title or kind) changed; `text_diff` is a word-level diff. **moved**: the element changed list (for example from the main scenario into a branch), or its place among the elements that kept their relative order; when two adjacent steps swap, the one that came forward is the one reported. **rebranched**: a branch's origin step or end changed. `old` / `new` give each side's list (`preconditions`, `main`, `postconditions`, `branch` with `branch_id`, or `branches` for a branch itself), label and text. Elements follow the new document's order; removed ones come last. Unchanged elements are left out unless asked for.

### Comments

| Method | Path | Scope | Description |
|---|---|---|---|
| `GET` | `/use-cases/{id}/comments` | `use_cases:read` | Oldest first: `[{id, use_case_id, use_case_version_id, element_id, element_label, author_id, author_name, body, created_at}]`. `?version_id=` keeps that version's comments and the ones on no version; `?element_id=` one element's. |
| `POST` | `/use-cases/{id}/comments` | `use_cases:write` | `{body, use_case_version_id?, element_id?}`. **201**. The element must exist in that version, the current version or the draft (**400**). With `LOCK_APPROVED_VERSION_COMMENTS`, an approved version refuses comments (**403**). |

Comments are immutable. `@username` mentions of project members send a `mentioned` notification; the use case's creator and the publisher of its current version get `comment_added` (nobody gets two for one comment, and never for their own). Publishing a version sends `approval_requested` to the project's reviewers. These notifications have `entity_type: "use_case"`. On the start screen (`/api/home/attention`), notification items carry `use_case_id`, and current use case versions marked reviewed appear as approvals for reviewers.

### Baselines, search, bundle and MCP

- **Baselines.** Creating a baseline records the current version of every published use case of the project, and its links. `GET /baselines/{bid}/use-cases` lists them: `{use_case_id, reference_code, title, primary_actor, version_id, version_number, approval_state, current_version_id, current_version_number, link_count}`. `GET /baselines/{bid}/use-cases/{id}/diff/current` gives the structural diff from the recorded version to the current one. `GET /baselines/diff` also returns `use_cases_only_in_a`, `use_cases_only_in_b` and `use_cases_changed: [{use_case_id, version_a, version_b}]`. A use case a baseline records cannot be deleted (**409**).
- **Search.** `GET /api/search` hits have `kind: "requirement" | "use_case"` and `use_case_id`; use cases match on code, title, actors and step text.
- **Project bundle.** `bundle.json` has an optional `use_cases` section (see the HTTP API contract).
- **MCP.** `list_use_cases`, `get_use_case`, `get_use_case_diff` and `list_use_case_links` (`use_cases:read`); `create_use_case`, `edit_use_case_draft`, `publish_use_case_draft` and `link_use_case_element` (`use_cases:write`, `MARREQ_MODE=draft_write`). Approval is not exposed to agents. See [mcp-setup.md](mcp-setup.md).
