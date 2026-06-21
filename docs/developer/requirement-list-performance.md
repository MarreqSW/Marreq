# Project requirement listing performance

Tracks: #202

## Current hot path

The project-scoped endpoint `GET /api/projects/<project_id>/requirements` currently builds the response in layers:

1. `marreq-core/src/api/requirements.rs::list_by_project` asks `RequirementService::list_by_project_with_approval_and_tests` for the project requirements.
2. `RequirementService::list_by_project` loads the project requirements, then enriches each row with:
   - `enrich_parent_id_from_version_links`, which calls `get_parent_requirement_ids_for_version` per current version.
   - `attach_custom_fields`, which calls `get_custom_field_values_for_version` per current version.
3. The API handler then maps each `Requirement` into `RequirementListRow` and calls:
   - `get_verification_method_ids(requirement.id)` per requirement.
   - `get_parent_requirement_ids_for_version(current_version_id)` again per requirement.

This means the unfiltered list endpoint performs query work proportional to the number of requirements in the project. On a project with `N` requirements, the route-level enrichment alone adds up to `2N` service calls, and service-level enrichment adds another parent/custom-field pass.

## Required response shape

The optimized implementation must preserve the current serialized shape:

```rust
pub struct RequirementListRow {
    #[serde(flatten)]
    pub requirement: Requirement,
    pub verification_method_ids: Vec<i32>,
    pub parent_requirement_ids: Vec<i32>,
}
```

The `Requirement` payload must continue to include current custom field values through `requirement.custom_fields`, and `requirement.parent_id` must remain the first parent requirement id for backwards compatibility.

## Bounded-query target

The target implementation should compose each list page from preloaded maps instead of querying inside loops:

```rust
get_verification_method_ids_for_requirements(requirement_ids)
    -> HashMap<i32, Vec<i32>>

get_custom_field_values_for_versions(version_ids)
    -> HashMap<i32, Vec<CustomFieldValueDisplay>>

get_requirement_ids_for_version_ids(version_ids)
    -> HashMap<i32, i32>
```

Together with one project requirement query and one project link query, the common unfiltered path can be reduced to a bounded number of database round trips independent of project size.

## Proposed repository additions

### RequirementsRepository

Add bulk helpers with default loop-based fallbacks so tests and non-Diesel implementations remain source-compatible:

```rust
fn get_verification_method_ids_for_requirements(
    &self,
    requirement_ids: &[i32],
) -> Result<HashMap<i32, Vec<i32>>, RepoError>;

fn get_requirement_ids_for_version_ids(
    &self,
    version_ids: &[i32],
) -> Result<HashMap<i32, i32>, RepoError>;
```

The Diesel implementation can load these with `eq_any`:

- join `requirements.current_version_id` to `requirement_version_verification_methods.requirement_version_id`, returning `(requirement_id, verification_method_id)`.
- read `requirement_versions` for all target version ids, returning `(version_id, requirement_id)`.

### CustomFieldRepository

Add a bulk helper:

```rust
fn get_custom_field_values_for_versions(
    &self,
    version_ids: &[i32],
) -> Result<HashMap<i32, Vec<CustomFieldValueDisplay>>, RepoError>;
```

The Diesel implementation can join `custom_field_values` to `custom_field_definitions` once and group rows by `requirement_version_id`.

### CacheRepository

Forward all bulk helpers to the inner repository. These are list-composition reads; caching individual rows is less important than avoiding repeated per-row database access.

## Service composition plan

Add an internal list enrichment function that receives all requirements for a project and mutates them in one pass:

```rust
fn enrich_requirement_list(
    &self,
    project_id: i32,
    requirements: &mut [Requirement],
) -> Result<HashMap<i32, Vec<i32>>, RepoError>
```

Suggested flow:

1. Collect `requirement_ids` and current `version_ids` from the list.
2. Load project links once with `list_links_by_project(project_id, None, None, None)`.
3. Collect the linked `target_version_id`s and resolve them with `get_requirement_ids_for_version_ids`.
4. Build `parent_ids_by_source_version: HashMap<i32, Vec<i32>>`.
5. Load custom field values with `get_custom_field_values_for_versions(version_ids)`.
6. Load verification methods with `get_verification_method_ids_for_requirements(requirement_ids)`.
7. For each requirement:
   - set `parent_id` to the first sorted/deduplicated parent id when the legacy field is empty.
   - assign `custom_fields` to `Some(values)` only when values are present, matching current behavior.
8. Return `verification_ids_by_requirement` so the API handler can build `RequirementListRow` without per-row calls.

## Regression coverage

A practical regression test should exercise the API/service list path with at least two requirements and assert:

- both rows include the same `verification_method_ids` as before.
- parent ids are stable and deduplicated.
- custom field values are present on the correct requirement only.
- the repository mock records a constant number of bulk calls as `N` grows, or at minimum verifies that the new bulk methods are used by the list path.

## Manual validation checklist

After the implementation PR:

```bash
cd marreq-core
cargo fmt
cargo check
cargo test api_requirements_integration_test
```

For performance validation, seed a project with 10, 100, and 1,000 requirements and compare the number of SQL statements generated by `GET /api/projects/<project_id>/requirements`. The optimized path should remain bounded for parent, custom-field, and verification-method enrichment.
