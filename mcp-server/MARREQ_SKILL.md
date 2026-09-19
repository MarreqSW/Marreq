# Marreq requirements-engineering workflow

Use Marreq tools as a constrained requirements-engineering interface. Resolve
the project with `list_projects` first and pass its `project_id` to every
project-scoped tool. Do not guess a project from its display name.

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
only when requested and return its stable reference code. Reuse the same unique
`reference_code`, title, and description when retrying an ambiguous create.

## Modify

Fetch the current requirement and version first. Preserve version-producing
patch semantics and explain material changes. Editing never implies approval.

## Approval

Never approve automatically. Approval requires explicit user instruction, the
`requirements:approve` delegated scope, and the user's existing Marreq reviewer
or administrator permission. These instructions guide behavior; Marreq backend
authorization remains the security control.

Avoid destructive operations. Do not attempt requirement deletion or generic
REST calls outside the published MCP tools.
