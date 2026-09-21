// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#[cfg(not(any(test, feature = "test-helpers")))]
use diesel::{ExpressionMethods, JoinOnDsl, NullableExpressionMethods, QueryDsl, RunQueryDsl};
use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::{
    ProjectRequirementsApprove, ProjectRequirementsRead, ProjectRequirementsWrite,
};
use crate::models::{
    CustomFieldValueDisplay, CustomFieldValueInput, NewRequirement, Requirement,
    RequirementVersion, RequirementVersionLink, Verification,
};
use crate::repository::{
    errors::RepoError, MatrixRepository, RequirementsRepository, SavedViewRepository,
    UserRepository,
};
use crate::services::RequirementService;
use rocket::form::FromForm;
use std::collections::HashSet;

/// Trace summary for a requirement (parent, parent_links, children, linked tests). Used in project-scoped get.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct TraceSummary {
    /// Typed links from this requirement's current version to parent versions (multi-parent DAG).
    #[serde(default)]
    pub parent_links: Vec<RequirementVersionLink>,
    pub child_ids: Vec<i32>,
    pub linked_test_ids: Vec<i32>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementWithTraceSummary {
    #[serde(flatten)]
    pub requirement: Requirement,
    pub trace_summary: TraceSummary,
}

/// Single requirement version plus version-scoped custom fields and verification methods.
#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementVersionDetail {
    #[serde(flatten)]
    pub version: RequirementVersion,
    #[serde(default)]
    pub custom_fields: Vec<CustomFieldValueDisplay>,
    #[serde(default)]
    pub verification_method_ids: Vec<i32>,
}

fn version_detail(
    service: &RequirementService,
    version: RequirementVersion,
) -> ApiResult<RequirementVersionDetail> {
    let custom_fields = service.custom_fields_for_version(version.id)?;
    let verification_method_ids = service.verification_method_ids_for_version(version.id)?;
    Ok(RequirementVersionDetail {
        version,
        custom_fields,
        verification_method_ids,
    })
}

/// One parent link when creating a requirement (target version + link type).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct ParentLinkInput {
    pub target_version_id: i32,
    pub link_type: String,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementCreateRequest {
    pub title: String,
    pub description: String,
    pub author_id: i32,
    pub category_id: i32,
    pub status_id: i32,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
    #[serde(default)]
    pub verification_method_ids: Vec<i32>,
    #[serde(default)]
    pub custom_fields: Vec<CustomFieldValueInput>,
    #[serde(default)]
    pub parent_links: Vec<ParentLinkInput>,
}

/// One row in `GET /api/projects/:id/requirements`: full [`Requirement`] plus verification methods.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementListRow {
    #[serde(flatten)]
    pub requirement: Requirement,
    pub verification_method_ids: Vec<i32>,
    /// All parent requirement ids from version links (current version as source); `parent_id` on
    /// [`Requirement`] remains the first for backwards compatibility.
    pub parent_requirement_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<i32>,
    pub verification_method_ids: Option<Vec<i32>>,
    pub author_id: Option<i32>,
    pub reviewer_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub custom_fields: Option<Vec<CustomFieldValueInput>>,
}

type ParentLinkCommand = (i32, String, Option<String>);

struct CreateRequirementCommand {
    requirement: NewRequirement,
    verification_method_ids: Vec<i32>,
    custom_fields: Vec<CustomFieldValueInput>,
    parent_links: Vec<ParentLinkCommand>,
}

struct UpdateRequirementCommand {
    requirement: NewRequirement,
    verification_method_ids: Vec<i32>,
    custom_fields: Option<Vec<CustomFieldValueInput>>,
}

impl RequirementPatch {
    fn has_updates(&self) -> bool {
        self.title.is_some()
            || self.description.is_some()
            || self.status_id.is_some()
            || self.verification_method_ids.is_some()
            || self.author_id.is_some()
            || self.reviewer_id.is_some()
            || self.category_id.is_some()
            || self.applicability_id.is_some()
            || self.custom_fields.is_some()
    }
}

fn filter_positive_ids(ids: Vec<i32>) -> Vec<i32> {
    ids.into_iter().filter(|&id| id > 0).collect()
}

fn require_patch_updates(patch: &RequirementPatch) -> ApiResult<()> {
    if patch.has_updates() {
        Ok(())
    } else {
        Err(ApiError::BadRequest("no fields provided".into()))
    }
}

fn build_new_requirement_command(
    payload: RequirementCreateRequest,
) -> ApiResult<CreateRequirementCommand> {
    let verification_method_ids = filter_positive_ids(payload.verification_method_ids);
    if verification_method_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one verification_method_id required".into(),
        ));
    }

    let requirement = NewRequirement {
        id: None,
        title: payload.title,
        description: payload.description,
        author_id: payload.author_id,
        category_id: payload.category_id,
        status_id: payload.status_id,
        reference_code: payload.reference_code,
        reviewer_id: payload.reviewer_id,
        applicability_id: payload.applicability_id,
        justification: payload.justification,
        project_id: payload.project_id,
    };

    let parent_links = payload
        .parent_links
        .into_iter()
        .map(|pl| (pl.target_version_id, pl.link_type, pl.rationale))
        .collect();

    Ok(CreateRequirementCommand {
        requirement,
        verification_method_ids,
        custom_fields: payload.custom_fields,
        parent_links,
    })
}

fn apply_requirement_patch(
    requirement: Requirement,
    patch: RequirementPatch,
    default_verification_method_ids: Vec<i32>,
) -> ApiResult<UpdateRequirementCommand> {
    require_patch_updates(&patch)?;

    let RequirementPatch {
        title,
        description,
        status_id,
        verification_method_ids,
        author_id,
        reviewer_id,
        category_id,
        applicability_id,
        custom_fields,
    } = patch;

    let verification_method_ids =
        filter_positive_ids(verification_method_ids.unwrap_or(default_verification_method_ids));

    let requirement = NewRequirement {
        id: Some(requirement.id),
        title: title.unwrap_or(requirement.title),
        description: description.unwrap_or(requirement.description),
        author_id: author_id.unwrap_or(requirement.author_id),
        category_id: category_id.unwrap_or(requirement.category_id),
        status_id: status_id.unwrap_or(requirement.status_id),
        reference_code: requirement.reference_code,
        reviewer_id: reviewer_id.unwrap_or(requirement.reviewer_id),
        applicability_id: applicability_id.unwrap_or(requirement.applicability_id),
        justification: requirement.justification,
        project_id: requirement.project_id,
    };

    Ok(UpdateRequirementCommand {
        requirement,
        verification_method_ids,
        custom_fields,
    })
}

#[derive(Debug, Default, FromForm)]
pub struct RequirementListQuery {
    pub approval_state: Option<String>,
    pub has_tests: Option<bool>,
    pub status_id: Option<i32>,
    pub category_id: Option<i32>,
    pub q: Option<String>,
    pub sort_column: Option<String>,
    pub sort_dir: Option<String>,
    pub view_id: Option<i32>,
}

fn filter_project_requirement_list(
    state: &AppState,
    project_id: i32,
    query: &RequirementListQuery,
    viewer_user_id: i32,
) -> Result<Vec<Requirement>, RepoError> {
    let mut requirements = state.repo_read().get_requirements_by_project(project_id)?;

    let mut approval = query.approval_state.clone();
    let mut status = query.status_id;
    let mut category = query.category_id;
    let mut text_query = query.q.clone();
    let mut sort_col = query.sort_column.clone();
    let mut sort_direction = query.sort_dir.clone().unwrap_or_else(|| "asc".to_string());

    if let Some(vid) = query.view_id {
        let view = state.repo_read().get_saved_view_by_id(vid)?;
        if view.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let visible = view.visibility == "shared"
            || view.owner_id == viewer_user_id
            || state
                .repo_read()
                .get_user_by_id(viewer_user_id)
                .map(|u| u.is_admin)
                .unwrap_or(false);
        if !visible {
            return Err(RepoError::NotFound);
        }
        let applied = crate::saved_view_definition::filters_from_definition(&view.definition);
        if status.is_none() {
            status = applied.status_id;
        }
        if category.is_none() {
            category = applied.category_id;
        }
        if approval.is_none() {
            approval = applied.approval_state;
        }
        if text_query.is_none() {
            text_query = applied.q;
        }
        if sort_col.is_none() {
            sort_col = applied.sort_column;
            sort_direction = applied.sort_dir;
        }
    }

    if let Some(state_filter) = approval.as_deref() {
        let state_lower = state_filter.to_lowercase();
        requirements.retain(|requirement| requirement.approval_state.to_lowercase() == state_lower);
    }

    if let Some(sid) = status {
        requirements.retain(|requirement| requirement.status_id == sid);
    }
    if let Some(cid) = category {
        requirements.retain(|requirement| requirement.category_id == cid);
    }
    if let Some(raw_q) = text_query.as_deref() {
        let needle = raw_q.trim().to_lowercase();
        if !needle.is_empty() {
            requirements.retain(|requirement| {
                [
                    requirement.reference_code.as_str(),
                    requirement.title.as_str(),
                    requirement.description.as_str(),
                    requirement.id.to_string().as_str(),
                ]
                .join(" ")
                .to_lowercase()
                .contains(&needle)
            });
        }
    }

    if let Some(has_tests_filter) = query.has_tests {
        let links = state.repo_read().get_matrix_by_project(project_id)?;
        let req_ids_with_tests: HashSet<i32> = links.into_iter().map(|link| link.req_id).collect();
        if has_tests_filter {
            requirements.retain(|requirement| req_ids_with_tests.contains(&requirement.id));
        } else {
            requirements.retain(|requirement| !req_ids_with_tests.contains(&requirement.id));
        }
    }

    if let Some(col) = sort_col.as_deref() {
        let desc = sort_direction.eq_ignore_ascii_case("desc");
        requirements.sort_by(|a, b| {
            let ord = match col {
                "key" => a.reference_code.cmp(&b.reference_code),
                "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                "category" => a.category_id.cmp(&b.category_id),
                "status" => a.status_id.cmp(&b.status_id),
                "approval" => a.approval_state.cmp(&b.approval_state),
                "modified" => a.update_date.cmp(&b.update_date),
                "author" => a.author_id.cmp(&b.author_id),
                _ => std::cmp::Ordering::Equal,
            };
            if desc {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    Ok(requirements)
}

#[cfg(not(any(test, feature = "test-helpers")))]
fn build_requirement_list_rows(
    state: &AppState,
    project_id: i32,
    mut requirements: Vec<Requirement>,
) -> Result<Vec<RequirementListRow>, RepoError> {
    use crate::models::CustomFieldValueDisplay;
    use crate::schema::custom_field_definitions as cfd;
    use crate::schema::custom_field_values as cfv;
    use crate::schema::requirement_version_links as rvl;
    use crate::schema::requirement_version_verification_methods as rvvm;
    use crate::schema::requirement_versions as rv;
    use crate::schema::requirements as req;
    use std::collections::HashMap;

    if requirements.is_empty() {
        return Ok(Vec::new());
    }

    let requirement_ids: Vec<i32> = requirements
        .iter()
        .map(|requirement| requirement.id)
        .collect();
    let version_ids: Vec<i32> = requirements
        .iter()
        .filter_map(|requirement| requirement.current_version_id)
        .collect();

    let mut verification_method_ids_by_requirement: HashMap<i32, Vec<i32>> = requirement_ids
        .iter()
        .copied()
        .map(|id| (id, Vec::new()))
        .collect();
    let mut parent_requirement_ids_by_requirement: HashMap<i32, Vec<i32>> = requirement_ids
        .iter()
        .copied()
        .map(|id| (id, Vec::new()))
        .collect();
    let mut custom_fields_by_version: HashMap<i32, Vec<CustomFieldValueDisplay>> = HashMap::new();
    let mut parent_ids_by_source_version: HashMap<i32, Vec<i32>> = HashMap::new();

    let repo = state.repo_read();
    let mut conn = repo.inner_repo().get_conn()?;

    let verification_rows: Vec<(i32, i32)> = req::table
        .inner_join(
            rvvm::table.on(req::current_version_id.eq(rvvm::requirement_version_id.nullable())),
        )
        .filter(req::id.eq_any(&requirement_ids))
        .select((req::id, rvvm::verification_method_id))
        .order((req::id, rvvm::verification_method_id))
        .load(conn.as_mut())
        .map_err(RepoError::from)?;
    for (requirement_id, verification_method_id) in verification_rows {
        verification_method_ids_by_requirement
            .entry(requirement_id)
            .or_default()
            .push(verification_method_id);
    }

    if !version_ids.is_empty() {
        let custom_field_rows: Vec<(i32, i32, String, Option<String>)> = cfv::table
            .inner_join(cfd::table.on(cfv::custom_field_definition_id.eq(cfd::id)))
            .filter(cfv::requirement_version_id.eq_any(&version_ids))
            .select((cfv::requirement_version_id, cfd::id, cfd::label, cfv::value))
            .order((cfv::requirement_version_id, cfd::sort_order, cfd::id))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        for (version_id, field_id, label, value) in custom_field_rows {
            custom_fields_by_version
                .entry(version_id)
                .or_default()
                .push(CustomFieldValueDisplay {
                    field_id,
                    label,
                    value,
                });
        }

        let link_rows: Vec<(i32, i32)> = rvl::table
            .filter(rvl::project_id.eq(project_id))
            .filter(rvl::source_version_id.eq_any(&version_ids))
            .select((rvl::source_version_id, rvl::target_version_id))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;

        let mut target_version_ids: Vec<i32> = link_rows
            .iter()
            .map(|(_, target_version_id)| *target_version_id)
            .collect();
        target_version_ids.sort_unstable();
        target_version_ids.dedup();

        if !target_version_ids.is_empty() {
            let target_version_rows: Vec<(i32, i32)> = rv::table
                .filter(rv::id.eq_any(&target_version_ids))
                .select((rv::id, rv::requirement_id))
                .load(conn.as_mut())
                .map_err(RepoError::from)?;
            let target_version_to_requirement: HashMap<i32, i32> =
                target_version_rows.into_iter().collect();

            for (source_version_id, target_version_id) in link_rows {
                if let Some(&parent_requirement_id) =
                    target_version_to_requirement.get(&target_version_id)
                {
                    parent_ids_by_source_version
                        .entry(source_version_id)
                        .or_default()
                        .push(parent_requirement_id);
                }
            }
        }
    }

    for parent_ids in parent_ids_by_source_version.values_mut() {
        parent_ids.sort_unstable();
        parent_ids.dedup();
    }

    for requirement in &requirements {
        if let Some(version_id) = requirement.current_version_id {
            if let Some(parent_ids) = parent_ids_by_source_version.get(&version_id) {
                parent_requirement_ids_by_requirement.insert(requirement.id, parent_ids.clone());
            }
        }
    }

    let mut rows = Vec::with_capacity(requirements.len());
    for mut requirement in requirements.drain(..) {
        let parent_requirement_ids = parent_requirement_ids_by_requirement
            .remove(&requirement.id)
            .unwrap_or_default();
        if requirement.parent_id.is_none() {
            requirement.parent_id = parent_requirement_ids.first().copied();
        }

        if let Some(version_id) = requirement.current_version_id {
            let custom_fields = custom_fields_by_version
                .remove(&version_id)
                .unwrap_or_default();
            requirement.custom_fields = if custom_fields.is_empty() {
                None
            } else {
                Some(custom_fields)
            };
        } else {
            requirement.custom_fields = Some(Vec::new());
        }

        let verification_method_ids = verification_method_ids_by_requirement
            .remove(&requirement.id)
            .unwrap_or_default();
        rows.push(RequirementListRow {
            requirement,
            verification_method_ids,
            parent_requirement_ids,
        });
    }

    Ok(rows)
}

#[cfg(any(test, feature = "test-helpers"))]
fn build_requirement_list_rows(
    state: &AppState,
    _project_id: i32,
    requirements: Vec<Requirement>,
) -> Result<Vec<RequirementListRow>, RepoError> {
    let service = RequirementService::new(state);
    Ok(requirements
        .into_iter()
        .map(|requirement| {
            let requirement = service.get_by_id(requirement.id).unwrap_or(requirement);
            let verification_method_ids = service
                .get_verification_method_ids(requirement.id)
                .unwrap_or_default();
            let parent_requirement_ids = requirement
                .current_version_id
                .map(|vid| service.get_parent_requirement_ids_for_version(vid))
                .unwrap_or_default();
            RequirementListRow {
                requirement,
                verification_method_ids,
                parent_requirement_ids,
            }
        })
        .collect())
}

#[get("/requirements")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Requirement>>> {
    let service = RequirementService::new(state.inner());
    let requirements = service.list_all()?;
    Ok(Json(requirements))
}

/// Project-scoped list with optional filters (MCP and API). Accepts session or Bearer token.
/// Query: approval_state, has_tests, status_id, category_id, q, sort_column, sort_dir, view_id.
#[get("/projects/<project_id>/requirements?<query..>")]
pub async fn list_by_project(
    access: ProjectRequirementsRead,
    project_id: i32,
    query: RequirementListQuery,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementListRow>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let requirements =
        filter_project_requirement_list(state.inner(), project_id, &query, access.user().id)?;
    let rows = build_requirement_list_rows(state.inner(), project_id, requirements)?;
    Ok(Json(rows))
}

#[get("/requirements/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Requirement>> {
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    Ok(Json(requirement))
}

/// Project-scoped get with trace summary (parent_id, child_ids, linked_test_ids). Accepts session or Bearer.
#[get("/projects/<project_id>/requirements/<id>", rank = 2)]
pub async fn get_by_project(
    access: ProjectRequirementsRead,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementWithTraceSummary>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let children = service.get_children_by_parent_and_project(project_id, id)?;
    let linked_tests = service.get_linked_verifications(id)?;
    let parent_links = requirement
        .current_version_id
        .map(|vid| {
            service
                .get_parent_links_for_version(vid)
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let trace_summary = TraceSummary {
        parent_links,
        child_ids: children.iter().map(|r| r.id).collect(),
        linked_test_ids: linked_tests.iter().map(|t| t.id).collect(),
    };
    Ok(Json(RequirementWithTraceSummary {
        requirement,
        trace_summary,
    }))
}

/// List all versions for a requirement (newest first).
#[get("/requirements/<id>/versions")]
pub async fn list_versions(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementVersion>>> {
    let service = RequirementService::new(state.inner());
    let versions = service.list_versions(id)?;
    Ok(Json(versions))
}

/// Project-scoped list versions (session or Bearer). Enforces requirement belongs to project.
#[get("/projects/<project_id>/requirements/<id>/versions")]
pub async fn list_versions_by_project(
    access: ProjectRequirementsRead,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementVersion>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let versions = service.list_versions(id)?;
    Ok(Json(versions))
}

/// Get a single requirement version by id (version must belong to the given requirement).
#[get("/requirements/<req_id>/versions/<version_id>")]
pub async fn get_version(
    _user: ApiUser,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementVersionDetail>> {
    let service = RequirementService::new(state.inner());
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    Ok(Json(version_detail(&service, version)?))
}

/// Project-scoped get version (session or Bearer). Enforces requirement belongs to project.
#[get("/projects/<project_id>/requirements/<req_id>/versions/<version_id>")]
pub async fn get_version_by_project(
    access: ProjectRequirementsRead,
    project_id: i32,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementVersionDetail>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(req_id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    Ok(Json(version_detail(&service, version)?))
}

/// List tests linked to the requirement that are currently marked suspect (impacted by requirement changes).
#[get("/requirements/<id>/impacted_tests")]
pub async fn get_impacted_tests(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Verification>>> {
    let service = RequirementService::new(state.inner());
    let _requirement = service.get_by_id(id)?;
    let verifications = service.get_impacted_verifications(id)?;
    Ok(Json(verifications))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct SetApprovalRequest {
    /// Target state: "reviewed" or "approved"
    pub state: String,
}

/// Transition a requirement version's approval state (draft→reviewed, reviewed→approved).
/// Restricted to project owners (role 1), managers (role 2), or admins.
#[put(
    "/requirements/<req_id>/versions/<version_id>/approval",
    data = "<payload>"
)]
pub async fn set_version_approval(
    user: ApiUser,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
    payload: Json<SetApprovalRequest>,
) -> ApiResult<Json<RequirementVersion>> {
    let service = RequirementService::new(state.inner());
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    let requirement = service.get_by_id(req_id)?;
    let u = user.user();
    require_project_reviewer(state, u, requirement.project_id)?;
    let new_state = payload.state.trim();
    if new_state != "reviewed" && new_state != "approved" {
        return Err(ApiError::BadRequest(
            "state must be 'reviewed' or 'approved'".into(),
        ));
    }
    let updated = state
        .repo_write()
        .set_requirement_version_approval(version_id, new_state, u.id)?;

    if new_state == "reviewed" {
        let ns = crate::services::NotificationService::new(state.inner());
        ns.notify_approval_requested(u, &requirement, requirement.project_id);
    }

    Ok(Json(updated))
}

#[post("/requirements", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<RequirementCreateRequest>,
) -> ApiResult<Value> {
    let payload = payload.into_inner();
    require_project_permission(
        state,
        user.user(),
        payload.project_id,
        Permission::EditRequirements,
    )?;
    require_project_reviewer_unless_requirement_create_status_is_draft_like(
        state,
        user.user(),
        payload.project_id,
        payload.status_id,
    )?;

    let CreateRequirementCommand {
        requirement,
        verification_method_ids,
        custom_fields,
        parent_links,
    } = build_new_requirement_command(payload)?;
    let service = RequirementService::new(state.inner());
    let custom_fields = if custom_fields.is_empty() {
        None
    } else {
        Some(custom_fields.as_slice())
    };
    let id = service.create(
        user.user(),
        requirement,
        &verification_method_ids,
        custom_fields,
        Some(parent_links),
    )?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = RequirementService::new(state.inner());
    service.delete(user.user(), id)?;
    Ok(Status::NoContent)
}

#[patch("/requirements/<id>", data = "<patch>")]
pub async fn patch_requirement(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    patch: Json<RequirementPatch>,
) -> ApiResult<Value> {
    let patch = patch.into_inner();
    require_patch_updates(&patch)?;
    let status_id_is_changed = patch.status_id.is_some();
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    require_project_permission(
        state,
        user.user(),
        requirement.project_id,
        Permission::EditRequirements,
    )?;
    if status_id_is_changed {
        require_project_reviewer(state, user.user(), requirement.project_id)?;
    }

    let default_verification_method_ids = if patch.verification_method_ids.is_some() {
        Vec::new()
    } else {
        service.get_verification_method_ids(id).unwrap_or_default()
    };
    let UpdateRequirementCommand {
        requirement: payload,
        verification_method_ids,
        custom_fields,
    } = apply_requirement_patch(requirement, patch, default_verification_method_ids)?;

    service.update(
        user.user(),
        id,
        payload,
        &verification_method_ids,
        custom_fields.as_deref(),
        None,
    )?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

/// Project-scoped create (session or Bearer). For MCP Phase 2 draft_write.
#[post("/projects/<project_id>/requirements", data = "<payload>")]
pub async fn create_by_project(
    access: ProjectRequirementsWrite,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<RequirementCreateRequest>,
    idempotency_key: crate::api::idempotency::OptionalIdempotencyKey,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let payload = payload.into_inner();
    if payload.project_id != project_id {
        return Err(ApiError::BadRequest(
            "payload.project_id must match route project_id".into(),
        ));
    }
    require_project_reviewer_unless_requirement_create_status_is_draft_like(
        state,
        access.user(),
        project_id,
        payload.status_id,
    )?;

    let CreateRequirementCommand {
        requirement,
        verification_method_ids,
        custom_fields,
        parent_links,
    } = build_new_requirement_command(payload.clone())?;
    let principal = access.auth().idempotency_principal();
    let target = format!("project:{project_id}");
    let operation_identity = crate::api::idempotency::operation_identity(
        access.user().id,
        &principal,
        &target,
        "create_requirement",
        &idempotency_key,
    );
    if let Some(response) = crate::api::idempotency::claim(
        state,
        access.user().id,
        &principal,
        &target,
        "create_requirement",
        &idempotency_key,
        &payload,
    )? {
        return Ok(response);
    }
    let service = RequirementService::new(state.inner());
    let custom_fields = if custom_fields.is_empty() {
        None
    } else {
        Some(custom_fields.as_slice())
    };
    let id = crate::api::idempotency::release_on_repo_error(
        service.create_with_idempotency(
            access.user(),
            requirement,
            &verification_method_ids,
            custom_fields,
            Some(parent_links),
            operation_identity.as_deref(),
        ),
        state,
        access.user().id,
        &principal,
        &target,
        "create_requirement",
        &idempotency_key,
    )?;
    let response = json!({ "status": "ok", "id": id });
    crate::api::idempotency::complete(
        state,
        access.user().id,
        &access.auth().idempotency_principal(),
        &format!("project:{project_id}"),
        "create_requirement",
        &idempotency_key,
        &response,
    )?;
    Ok(response)
}

/// Project-scoped patch (session or Bearer). For MCP Phase 2 draft_write.
#[patch("/projects/<project_id>/requirements/<id>", data = "<patch>")]
pub async fn patch_by_project(
    access: ProjectRequirementsWrite,
    project_id: i32,
    id: i32,
    patch: Json<RequirementPatch>,
    state: &State<AppState>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let patch = patch.into_inner();
    require_patch_updates(&patch)?;
    let status_id_is_changed = patch.status_id.is_some();
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    if status_id_is_changed {
        require_project_reviewer(state, access.user(), project_id)?;
    }

    let default_verification_method_ids = if patch.verification_method_ids.is_some() {
        Vec::new()
    } else {
        service.get_verification_method_ids(id).unwrap_or_default()
    };
    let UpdateRequirementCommand {
        requirement: payload,
        verification_method_ids,
        custom_fields,
    } = apply_requirement_patch(requirement, patch, default_verification_method_ids)?;

    service.update(
        access.user(),
        id,
        payload,
        &verification_method_ids,
        custom_fields.as_deref(),
        None,
    )?;
    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

/// Project-scoped set version approval (session or Bearer). For MCP Phase 2 draft_write.
#[put(
    "/projects/<project_id>/requirements/<req_id>/versions/<version_id>/approval",
    data = "<payload>"
)]
pub async fn set_version_approval_by_project(
    access: ProjectRequirementsApprove,
    project_id: i32,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
    payload: Json<SetApprovalRequest>,
) -> ApiResult<Json<RequirementVersion>> {
    let service = RequirementService::new(state.inner());
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    let requirement = service.get_by_id(req_id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    require_project_reviewer(state, access.user(), project_id)?;
    let u = access.user();
    let new_state = payload.state.trim();
    if new_state != "reviewed" && new_state != "approved" {
        return Err(ApiError::BadRequest(
            "state must be 'reviewed' or 'approved'".into(),
        ));
    }
    let updated = state
        .repo_write()
        .set_requirement_version_approval(version_id, new_state, u.id)?;

    if new_state == "reviewed" {
        let ns = crate::services::NotificationService::new(state.inner());
        ns.notify_approval_requested(u, &requirement, requirement.project_id);
    }

    Ok(Json(updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;

    fn auth_cookie_for(
        client: &rocket::local::asynchronous::Client,
        user_id: i32,
    ) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, user_id)
    }
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount(
                "/api",
                routes![
                    list,
                    get,
                    list_versions,
                    get_version,
                    get_version_by_project,
                    create,
                    delete,
                    patch_requirement,
                ],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(client: &rocket::local::asynchronous::Client) -> rocket::http::Cookie<'static> {
        auth_cookie_for(client, ADMIN_ID)
    }

    fn sample_requirement(title: &str) -> Value {
        json!({
            "title": title,
            "description": format!("{title} description"),
            "verification_method_ids": [1],
            "author_id": 1,
            "category_id": 1,
            "status_id": 1,
            "reference_code": "REF-1",
            "reviewer_id": 2,
            "applicability_id": 3,
            "justification": null,
            "project_id": 1
        })
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .get("/api/requirements")
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Requirement> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_requirement("First").to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn patch_updates_fields() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_requirement("Original").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(
                json!({
                    "title": "Updated",
                    "description": "Updated description"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("success"), Some(&Value::from(true)));

        let get_response = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        let requirement: Requirement = get_response.into_json().await.unwrap();
        assert_eq!(requirement.title, "Updated");
        assert_eq!(requirement.description, "Updated description");
    }

    #[rocket::async_test]
    async fn patch_creates_new_version_and_versions_list_returns_history() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let mut req = sample_requirement("V1 Title");
        req["reference_code"] = serde_json::Value::from("REQ-001");
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(req.to_string())
            .dispatch()
            .await;
        assert_eq!(
            create_response.status(),
            Status::Ok,
            "create should succeed"
        );
        let created: Value = create_response.into_json().await.unwrap();
        let id = created
            .get("id")
            .and_then(Value::as_i64)
            .expect("create response should have id") as i32;

        let versions_after_create = client
            .get(format!("/api/requirements/{id}/versions"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(versions_after_create.status(), Status::Ok);
        let versions: Vec<RequirementVersion> = versions_after_create.into_json().await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].title, "V1 Title");

        client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(json!({ "title": "V2 Updated" }).to_string())
            .dispatch()
            .await;

        let versions_after_patch = client
            .get(format!("/api/requirements/{id}/versions"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(versions_after_patch.status(), Status::Ok);
        let versions: Vec<RequirementVersion> = versions_after_patch.into_json().await.unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].title, "V2 Updated");
        assert_eq!(versions[1].title, "V1 Title");

        let first_version_id = versions[1].id;
        let current_version_id = versions[0].id;
        let single = client
            .get(format!(
                "/api/requirements/{id}/versions/{first_version_id}"
            ))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(single.status(), Status::Ok);
        let v: RequirementVersionDetail = single.into_json().await.unwrap();
        assert_eq!(v.version.id, first_version_id);
        assert_eq!(v.version.requirement_id, id);
        assert_eq!(v.version.title, "V1 Title");

        let current = client
            .get(format!(
                "/api/requirements/{id}/versions/{current_version_id}"
            ))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(current.status(), Status::Ok);
        let current_detail: RequirementVersionDetail = current.into_json().await.unwrap();
        assert_eq!(current_detail.verification_method_ids, vec![1]);

        let scoped = client
            .get(format!(
                "/api/projects/1/requirements/{id}/versions/{first_version_id}"
            ))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(scoped.status(), Status::Ok);

        let wrong_project = client
            .get(format!(
                "/api/projects/2/requirements/{id}/versions/{first_version_id}"
            ))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(wrong_project.status(), Status::NotFound);

        let wrong_req = client
            .get(format!("/api/requirements/999/versions/{first_version_id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(wrong_req.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn get_version_includes_custom_fields() {
        use crate::models::CustomFieldDefinitionPayload;
        use crate::repository::CustomFieldRepository;

        let mut repo = DieselRepoMock::default();
        repo.create_custom_field_definition(
            1,
            &CustomFieldDefinitionPayload {
                label: "Priority".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: Some(0),
            },
        )
        .unwrap();
        let client = client_with_repo(repo).await;
        let mut req = sample_requirement("V1 Title");
        req["custom_fields"] = json!([{ "field_id": 1, "value": "High" }]);
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(req.to_string())
            .dispatch()
            .await;
        assert_eq!(create_response.status(), Status::Ok);
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let versions_resp = client
            .get(format!("/api/requirements/{id}/versions"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        let versions: Vec<RequirementVersion> = versions_resp.into_json().await.unwrap();
        let version_id = versions[0].id;

        let single = client
            .get(format!("/api/requirements/{id}/versions/{version_id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(single.status(), Status::Ok);
        let detail: RequirementVersionDetail = single.into_json().await.unwrap();
        assert_eq!(detail.custom_fields.len(), 1);
        assert_eq!(detail.custom_fields[0].field_id, 1);
        assert_eq!(detail.custom_fields[0].label, "Priority");
        assert_eq!(detail.custom_fields[0].value.as_deref(), Some("High"));
        assert_eq!(detail.verification_method_ids, vec![1]);
    }

    #[rocket::async_test]
    async fn patch_without_fields_returns_bad_request() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_requirement("Original").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(json!({}).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(
            payload.get("message"),
            Some(&Value::from("no fields provided"))
        );
    }

    #[rocket::async_test]
    async fn delete_removes_requirement() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_requirement("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}
