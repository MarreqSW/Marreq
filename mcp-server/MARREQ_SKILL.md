# Marreq requirements-engineering workflow

Use Marreq tools as a constrained requirements-engineering interface. Resolve
the project with `list_projects` first and pass its `project_id` to every
project-scoped tool. Do not guess a project from its display name. For an
ambiguous write retry, reuse the same `idempotency_key`; never reuse a key for
a different operation, target, project, or payload.

## Review

1. Search existing requirements (semantic search when enabled, otherwise list).
2. Fetch relevant requirements and inspect hierarchy, versions, and diffs.
3. Inspect verification, traceability, coverage, suspect links, and baselines.
4. Identify duplicates, ambiguity, missing requirements, missing verification,
   and suspect traceability.
5. Present findings before changing data unless the user explicitly requested
   a write.

## Create

Before creation, search for overlap, inspect the project catalog, and determine
the correct parent/hierarchy. Draft one atomic, testable requirement. Create it
only when requested and return its stable reference code.

## Modify

Fetch the current requirement and version first. Preserve version-producing
patch semantics and explain material changes. Editing never implies approval.

## Use cases

A use case is one JSON document per version: preconditions, numbered main
steps, alternatives and exceptions that branch from a main step, and
postconditions. Refer to elements by their stable ids (`s_…`, `b_…`), never by
their labels (`3`, `A3`), which change when steps move. To edit, read the use
case with `get_use_case`, then send `edit_use_case_draft` with the draft's
`revision` as `base_revision` (`null` when there is no draft). On a stale
revision, read again and redo the change; never overwrite someone else's draft.
Publish only when asked, and use `get_use_case_diff` to show what changes. Use
case approval is for people only.

## Traceability

Read links before changing them: `trace_up` and `trace_down` for a requirement,
`list_requirement_links` for the version links, `list_requirement_link_types`
for the allowed types. Fix a missing link with `link_requirements` (reuse the
`idempotency_key` on a retry) and remove a wrong one with `unlink_requirements`
or `unlink_use_case_element`, only when the user asked and after saying which
link goes. Removing a link never deletes the requirement, verification or use
case, and a baseline keeps what it recorded. After a requirement changes, run
`impacted_tests`, and clear a use case link's suspect flag only after checking
that the use case still fits.

## Discovery and history

Filter `list_requirements` instead of reading everything: `status_id`,
`category_id`, `q`, `view_id` (see `list_saved_views`) and `sort_column`. Page
long lists with `limit` and the `next_cursor` of the previous page. Call
`get_my_permissions` before a write that may be refused (approval, status
change). `get_close_out` shows what lacks a passing verification;
`list_verification_snapshots` and `compare_verification_snapshots` show how a
verification changed; `get_baseline_contents` and
`diff_baseline_verification_vs_current` read what a baseline recorded.
`ask_requirements` calls an LLM on the server, so prefer
`semantic_search_requirements` unless a written answer is needed. Use cases
have `list_use_case_versions`, `list_use_case_comments` and
`create_use_case_comment`; `discard_use_case_draft` throws away unpublished
edits, so read `get_use_case_diff` first and confirm with the user.

## Resources and prompts

Clients that support them can read `marreq://projects/{project_id}/requirements/{id}`,
`.../use-cases/{id}`, `.../catalog` and `marreq://guide`, and start from the
prompts `review_requirement`, `derive_children`, `coverage_gap_analysis` and
`use_case_from_requirement`.

## Approval

Never approve automatically. Approval requires explicit user instruction, the
`requirements:approve` delegated scope, and the user's existing Marreq reviewer
or administrator permission. These instructions guide behavior; Marreq backend
authorization remains the security control.

Avoid destructive operations. Do not attempt requirement deletion or generic
REST calls outside the published MCP tools.
