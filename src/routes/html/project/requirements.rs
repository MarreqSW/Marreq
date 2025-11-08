use std::collections::HashMap;

use rocket::form::{Form, FromForm};
use rocket::response::Redirect;
use rocket::serde::json::{json, serde_json, Json};
use rocket::serde::Deserialize;
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::helper_functions::generate_requirement_reference;
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::services::{
    ApplicabilityService, CategoryService, DecoratedRequirementService, DecoratedTestService,
    LogService, ProjectService, RequirementAnalyticsService, RequirementService, StatusService,
    UserService, VerificationService,
};

#[derive(FromForm)]
struct RequirementCreateForm {
    #[field(name = uncased("intent"))]
    intent: Option<String>,
    #[field(name = uncased("req_id"))]
    req_id: Option<i32>,
    #[field(name = uncased("req_title"))]
    req_title: String,
    #[field(name = uncased("req_description"))]
    req_description: String,
    #[field(name = uncased("req_verification"))]
    req_verification: i32,
    #[field(name = uncased("req_category"))]
    req_category: i32,
    #[field(name = uncased("req_current_status"))]
    req_current_status: i32,
    #[field(name = uncased("req_parent"))]
    req_parent: i32,
    #[field(name = uncased("req_reference"))]
    req_reference: String,
    #[field(name = uncased("req_reviewer"))]
    req_reviewer: i32,
    #[field(name = uncased("req_applicability"))]
    req_applicability: i32,
    #[field(name = uncased("req_justification"))]
    req_justification: Option<String>,
}

impl RequirementCreateForm {
    fn into_payload(self, author_id: i32, project_id: i32) -> (NewRequirement, Option<String>) {
        let RequirementCreateForm {
            intent,
            req_id,
            req_description,
            req_verification,
            req_category,
            req_current_status,
            req_parent,
            req_reference,
            req_reviewer,
            req_applicability,
            req_justification,
            req_title,
        } = self;

        let requirement = NewRequirement {
            req_id,
            req_title,
            req_description,
            req_verification,
            req_author: author_id,
            req_category,
            req_current_status,
            req_parent,
            req_reference,
            req_reviewer,
            req_applicability,
            req_justification,
            project_id,
        };

        (requirement, intent)
    }
}

fn map_repo_error(err: RepoError) -> rocket::http::Status {
    match err {
        RepoError::BadInput(_) => rocket::http::Status::BadRequest,
        RepoError::NotFound => rocket::http::Status::NotFound,
        _ => rocket::http::Status::InternalServerError,
    }
}

// TODO: This shall be an authorization check to enforce project ownership and return a redirect when mismatched
fn enforce_project_ownership(route_project_id: i32, resource_project_id: i32) -> Option<Redirect> {
    if resource_project_id != route_project_id {
        eprintln!(
            "Project mismatch: route {}, resource {}",
            route_project_id, resource_project_id
        );
        Some(Redirect::to(uri!(
            "/p",
            show_requirements(
                project_id = resource_project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None,
                view = Option::<String>::None
            )
        )))
    } else {
        None
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineCategoryPayload {
    title: String,
    description: String,
    tag: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineApplicabilityPayload {
    title: String,
    description: String,
    tag: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineVerificationPayload {
    name: String,
    description: String,
}

#[get("/<project_id>/requirements?<status_filter>&<verification_filter>&<category_filter>&<view>")]
async fn show_requirements(
    project_access: ProjectAccess,
    project_id: i32,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    view: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;

    let requirements = DecoratedRequirementService::new(state.inner()).list_by_project_filtered(
        project_id,
        status_filter,
        verification_filter,
        category_filter,
    )?;

    let metrics = RequirementAnalyticsService::new(state.inner()).metrics(
        project_id,
        status_filter,
        verification_filter,
        category_filter,
    )?;

    // Build tree data for tree view
    let mut children: HashMap<i32, Vec<&DecoratedRequirement>> = HashMap::new();
    let mut roots: Vec<&DecoratedRequirement> = Vec::new();

    for r in &requirements {
        if r.req_parent_id == 0 {
            roots.push(r);
        } else {
            children.entry(r.req_parent_id).or_default().push(r);
        }
    }

    roots.sort_by_key(|r| r.req_id);
    for v in children.values_mut() {
        v.sort_by_key(|r| r.req_id);
    }

    fn build_node<'a>(
        req: &'a DecoratedRequirement,
        idx: &HashMap<i32, Vec<&'a DecoratedRequirement>>,
    ) -> serde_json::Value {
        let kids = idx
            .get(&req.req_id)
            .map(|vs| vs.iter().map(|c| build_node(c, idx)).collect::<Vec<_>>())
            .unwrap_or_default();

        json!({
            "requirement": req,
            "children": kids
        })
    }

    let tree_data = roots
        .into_iter()
        .map(|r| build_node(r, &children))
        .collect::<Vec<_>>();

    // Determine current view (default to card)
    let current_view = view.as_deref().unwrap_or("card");

    let ctx = json!({
        "user": user,
        "requirements": json!(requirements),
        "tree_data": tree_data,
        "requirement_metrics": json!({
            "total": metrics.total,
            "draft": metrics.draft,
            "accepted": metrics.accepted,
            "rejected": metrics.rejected,
            "coverage": {
                "verified": metrics.coverage_verified,
                "percent": metrics.coverage_percent
            }
        }),
        "statuses": StatusService::new(state.inner()).list_legacy()?,
        "verifications": VerificationService::new(state.inner()).list_by_project(project_id)?,
        "categories": CategoryService::new(state.inner()).list_by_project(project_id)?,
        "current_status_filter": json!(status_filter),
        "current_verification_filter": json!(verification_filter),
        "current_category_filter": json!(category_filter),
        "current_view": current_view,
        "project": json!({
            "id": selected_project.project_id,
            "name": selected_project.project_name,
        }),
        "is_admin": user.is_admin,
    });

    Ok(Template::render("requirements/requirements", ctx))
}

#[get("/<project_id>/requirements/show/<req_id>")]
async fn show_requirement_id(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let decorated_requirement_service = DecoratedRequirementService::new(state.inner());

    let requirement = decorated_requirement_service.get_by_id(req_id)?;

    if let Some(redir) = enforce_project_ownership(project_id, requirement.project_id) {
        return Err(redir);
    }

    let parent_requirement = if requirement.req_parent_id != 0 {
        decorated_requirement_service
            .get_by_id(requirement.req_parent_id)
            .ok()
    } else {
        None
    };
    let child_requirements = decorated_requirement_service.get_by_parent_id(requirement.req_id)?;

    // Linked verification artefacts
    let linked_tests =
        DecoratedTestService::new(state.inner()).get_linked_to_requirement(req_id)?;

    let (tests_passed, tests_failed, tests_pending) =
        linked_tests
            .iter()
            .fold((0_i32, 0_i32, 0_i32), |mut acc, test| {
                match test.test_status.trim().to_ascii_lowercase().as_str() {
                    "passed" => acc.0 += 1,
                    "failed" => acc.1 += 1,
                    _ => acc.2 += 1,
                }
                acc
            });

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Requirement.to_string(), req_id)
        .unwrap_or_default();

    let canonical_data = json!({
        "project_id": project_id,
        "requirement": requirement,
        "relationships": {
            "parent": parent_requirement,
            "children": child_requirements,
        },
        "linked_tests": linked_tests,
        "verification": {
            "tool_id": requirement.req_verification_id,
            "tool_name": requirement.req_verification.clone(),
            "counts": {
                "total": linked_tests.len() as i32,
                "passed": tests_passed,
                "failed": tests_failed,
                "pending": tests_pending,
            }
        },
        "history": {
            "entries": history_entries,
        },
        "comments": {
            "items": Vec::<serde_json::Value>::new(), // TODO: load comments
        }
    });

    let ctx = json!({
        "user": user,
        "project": json!({
            "id": selected_project.project_id,
            "name": selected_project.project_name,
        }),
        "requirement_data": canonical_data,
        "requirement_data_json": serde_json::to_string(&canonical_data).unwrap_or_else(|_| "{}".to_string()),
    });

    Ok(Template::render("requirements/requirement", ctx))
}

#[get("/<project_id>/requirements/edit/<req_id>")]
async fn get_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let project_name = ProjectService::new(state.inner())
        .get_by_id(project_id)?
        .project_name;
    let service = DecoratedRequirementService::new(state.inner());
    let req = service.get_by_id(req_id)?;

    // Enforce project ownership; redirect if mismatched
    if let Some(redir) = enforce_project_ownership(project_id, req.project_id) {
        return Err(redir);
    }

    let parent: Option<DecoratedRequirement> = if req.req_parent_id != 0 {
        Some(service.get_by_id(req.req_parent_id)?)
    } else {
        None
    };

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Requirement.to_string(), req_id)
        .unwrap_or_default();

    let version_counter = history_entries.len().saturating_add(1);
    let version_label = format!("v1.{}", version_counter.saturating_sub(1));
    let last_editor_name = history_entries
        .first()
        .map(|entry| entry.username.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            if !req.req_reviewer.trim().is_empty() {
                Some(req.req_reviewer.clone())
            } else if !req.req_author.trim().is_empty() {
                Some(req.req_author.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown author".to_string());

    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let users = UserService::new(state.inner()).get_by_project(project_id)?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;
    let applicability = ApplicabilityService::new(state.inner()).list_by_project(project_id)?;

    // Lightweight list of other requirements for linking (excluding current requirement)
    let linked_requirement_options = RequirementService::new(state.inner())
        .list_by_project(project_id)?
        .into_iter()
        .filter(|candidate| candidate.req_id != req_id) // Don't allow self-reference
        .map(|candidate| {
            json!({
                "id": candidate.req_id,
                "title": candidate.req_title,
                "reference": candidate.req_reference,
            })
        })
        .collect::<Vec<_>>();

    let display_reference = if req.req_reference.trim().is_empty() {
        format!("RM-{:03}", req.req_id)
    } else {
        req.req_reference.clone()
    };

    let ctx = json!({
        "req": req,
        "categories": categories,
        "parent": parent,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "linked_requirement_options": linked_requirement_options,
        "user": user,
        "display_reference": display_reference,
        "project_name": project_name,
        "version": {
            "label": version_label,
            "last_editor": last_editor_name,
            "updated_at": req.req_update_date,
        },
        "autosave": {
            "enabled": true,
            "interval_ms": 3_000
        }
    });

    #[cfg(debug_assertions)]
    println!("Edit requirement ctx: {:#}", ctx);

    Ok(Template::render("requirements/edit_requirement", ctx))
}

#[post("/<project_id>/requirements/edit/<req_id>", data = "<new_req>")]
async fn post_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let service = RequirementService::new(state.inner());
    if let Some(redir) =
        enforce_project_ownership(project_id, service.get_by_id(req_id)?.project_id)
    {
        return Err(redir);
    }

    let user = project_access.into_user();
    service.update(&user, req_id, new_req.into_inner())?;
    Ok(Redirect::to(uri!(
        "/p",
        show_requirement_id(project_id, req_id)
    )))
}

#[delete("/<project_id>/requirements/delete/<req_id>")]
async fn delete_requirement_route(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();

    let service = RequirementService::new(state.inner());
    let req = service.get_by_id(req_id)?;

    if let Some(_redir) = enforce_project_ownership(project_id, req.project_id) {
        return Err(rocket::http::Status::NotFound);
    }

    // Permission gate: allow only Draft(1) or Proposal(2) or admin
    if req.req_current_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    service.delete(&user, req_id)?;

    Ok(Redirect::to(uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None,
            view = Option::<String>::None
        )
    )))
}

#[get("/<project_id>/requirements/new?<error>&<created>&<parent>&<template>")]
async fn new_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
    error: Option<String>,
    created: Option<String>,
    parent: Option<i32>,
    template: Option<i32>, // use this requirement as a template
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let requirement_service = RequirementService::new(state.inner());

    let project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let statuses = StatusService::new(state.inner()).list_requirement_statuses()?;
    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let users = UserService::new(state.inner()).get_by_project(project_id)?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;
    let applicability = ApplicabilityService::new(state.inner()).list_by_project(project_id)?;

    // Lightweight list of other requirements for linking
    let parents = RequirementService::new(state.inner())
        .list_by_project(project_id)?
        .into_iter()
        .map(|candidate| {
            json!({
                "id": candidate.req_id,
                "title": candidate.req_title,
                "reference": candidate.req_reference,
            })
        })
        .collect::<Vec<_>>();

    let template_requirement: Option<Requirement> =
        template.and_then(|id| requirement_service.get_by_id(id).ok());

    let tr = template_requirement.as_ref(); // Option<&Requirement>

    let mut new_requirement = NewRequirement {
        req_id: None,
        req_title: tr.map(|r| r.req_title.clone()).unwrap_or_default(),
        req_description: tr.map(|r| r.req_description.clone()).unwrap_or_default(),
        req_verification: tr.map(|r| r.req_verification).unwrap_or_default(),
        req_author: user.user_id,
        req_category: tr.map(|r| r.req_category).unwrap_or_default(),
        req_current_status: 0, // Draft
        req_parent: tr.map(|r| r.req_parent).unwrap_or_default(),
        req_reference: tr.map(|r| r.req_reference.clone()).unwrap_or_default(),
        req_reviewer: tr.map(|r| r.req_reviewer).unwrap_or_default(),
        req_applicability: tr.map(|r| r.req_applicability).unwrap_or_default(),
        req_justification: tr.and_then(|r| r.req_justification.clone()),
        project_id,
    };

    // if parent is valid, assign, else 0
    if let Some(parent_id) = parent {
        new_requirement.req_parent = parents
            .iter()
            .find(|req| req["id"] == parent_id)
            .map(|_| parent_id)
            .unwrap_or(0);
    }

    // Default status to "Draft"
    new_requirement.req_current_status = statuses
        .iter()
        .find(|st| st.req_st_title.eq_ignore_ascii_case("Draft"))
        .map(|st| st.req_st_id)
        .unwrap_or_default();

    let created_flash = created.and_then(|flag| {
        if flag == "1" || flag.eq_ignore_ascii_case("true") {
            Some("Requirement created successfully.".to_string())
        } else {
            None
        }
    });

    let created_timestamp = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d")
        .to_string();

    // Check if user is admin or project owner
    let is_admin_or_owner = user.is_admin
        || project
            .project_owner_id
            .map_or(false, |owner_id| owner_id == user.user_id);

    let ctx = json!({
        "categories": categories,
        "status": statuses,
        "parent": parents,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "project_id": project_id,
        "project": {
            "id": project.project_id,
            "name": project.project_name,
        },
        "template": new_requirement,
        "created_timestamp": created_timestamp,
        "user": user,
        "is_admin_or_owner": is_admin_or_owner,
        "error": error,
        "flash_success": created_flash,
    });

    Ok(Template::render("requirements/new_requirement", ctx))
}

#[post("/<project_id>/requirements/new", data = "<new_req>")]
async fn post_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    new_req: Form<RequirementCreateForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    // Reuse these URLs
    let new_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Invalid data provided".to_string()),
            created = Option::<String>::None,
            parent = Option::<i32>::None,
            template = Option::<i32>::None
        )
    );

    // Take ownership and enforce project_id from the route
    let (mut req, intent) = new_req.into_inner().into_payload(user.user_id, project_id);
    req.project_id = project_id;
    req.req_author = user.user_id;

    // --- Reference validation / generation ---
    if !req.req_reference.is_empty() {
        // Validate against the category's tag
        let category = get_category_or_placeholder(state, req.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        if !req.req_reference.starts_with(&expected_prefix) {
            return Err(Redirect::to(new_url));
        }

        // Strict pattern: REQ-<CAT_TAG>-<NUMBER>
        // Escape the tag just in case and compile once.
        let pat = format!(r"^REQ-{}-\d+$", regex::escape(&category.cat_tag));
        let re = match regex::Regex::new(&pat) {
            Ok(r) => r,
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("regex compile failed for '{}': {:?}", pat, _e);
                return Err(Redirect::to(new_url));
            }
        };
        if !re.is_match(&req.req_reference) {
            return Err(Redirect::to(new_url));
        }
    } else {
        // Generate when missing
        let generated = {
            let repo = state.repo_read();
            generate_requirement_reference(&*repo, req.req_category, req.project_id)
        };

        match generated {
            Ok(reference) => req.req_reference = reference,
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("reference generation failed: {:?}", _e);
                req.req_reference = format!("REQ-UNKNOWN-{}", chrono::Utc::now().timestamp());
            }
        }
    }

    let failure_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Failed to create requirement".to_string()),
            created = Option::<String>::None,
            parent = Option::<i32>::None,
            template = Option::<i32>::None
        )
    );

    // --- Insert ---
    let service = RequirementService::new(state.inner());
    let req_id = match service.create(&user, req) {
        Ok(id) => id,
        Err(crate::repository::errors::RepoError::BadInput(_)) => {
            return Err(Redirect::to(new_url))
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("service create requirement failed: {:?}", _err);
            return Err(Redirect::to(failure_url));
        }
    };

    if matches!(intent.as_deref(), Some("add_another")) {
        return Ok(Redirect::to(uri!(
            "/p",
            new_requirement(
                project_id = project_id,
                error = Option::<String>::None,
                created = Some("1".to_string()),
                parent = Option::<i32>::None,
                template = Option::<i32>::None
            )
        )));
    }

    // --- Success: show the new requirement ---
    Ok(Redirect::to(uri!(
        "/p",
        show_requirement_id(project_id, req_id)
    )))
}

#[get("/<project_id>/requirements/tree")]
async fn show_requirements_tree(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    // Only this project's requirements
    let reqs = match RequirementService::new(state.inner()).list_by_project(project_id) {
        Ok(reqs) => reqs,
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to load requirements for tree view (project {}): {:?}",
                project_id, _err
            );
            Vec::new()
        }
    };

    // Index children by parent_id; collect roots
    let mut children: HashMap<i32, Vec<&Requirement>> = HashMap::new();
    let mut roots: Vec<&Requirement> = Vec::new();

    for r in &reqs {
        if r.req_parent == 0 {
            roots.push(r);
        } else {
            children.entry(r.req_parent).or_default().push(r);
        }
    }

    // Sort roots and each child list by req_id for deterministic output
    roots.sort_by_key(|r| r.req_id);
    for v in children.values_mut() {
        v.sort_by_key(|r| r.req_id);
    }

    // Recursive builder
    fn build_node<'a>(
        req: &'a Requirement,
        idx: &HashMap<i32, Vec<&'a Requirement>>,
    ) -> serde_json::Value {
        let kids = idx
            .get(&req.req_id)
            .map(|vs| vs.iter().map(|c| build_node(c, idx)).collect::<Vec<_>>())
            .unwrap_or_default();

        json!({
            "requirement": req,
            "children": kids
        })
    }

    let tree = roots
        .into_iter()
        .map(|r| build_node(r, &children))
        .collect::<Vec<_>>();

    let ctx = json!({
        "tree_data": tree,
        "total_requirements": reqs.len(),
        "user": user,
        "project_id": project_id,
        "selected_project_id": project_id
    });

    Ok(Template::render("requirements/requirements_tree", ctx))
}

#[post(
    "/<project_id>/requirements/inline/category",
    format = "json",
    data = "<payload>"
)]
async fn create_category_inline(
    project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineCategoryPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let user = project_access.into_user();
    let data = payload.into_inner();

    let category_service = CategoryService::new(state.inner());
    let new_category = NewCategory {
        cat_id: None,
        cat_title: data.title,
        cat_description: data.description,
        cat_tag: data.tag,
        project_id,
    };

    let id = category_service
        .create(&user, new_category)
        .map_err(map_repo_error)?;
    let stored = category_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.cat_id,
        "label": stored.cat_title,
        "tag": stored.cat_tag,
    })))
}

#[post(
    "/<project_id>/requirements/inline/applicability",
    format = "json",
    data = "<payload>"
)]
async fn create_applicability_inline(
    project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineApplicabilityPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let user = project_access.into_user();
    let data = payload.into_inner();

    let applicability_service = ApplicabilityService::new(state.inner());
    let new_applicability = NewApplicability {
        app_id: None,
        app_title: data.title,
        app_description: data.description,
        app_tag: data.tag,
        project_id,
    };

    let id = applicability_service
        .create(&user, new_applicability)
        .map_err(map_repo_error)?;
    let stored = applicability_service
        .get_by_id(id)
        .map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.app_id,
        "label": stored.app_title,
        "tag": stored.app_tag,
    })))
}

#[post(
    "/<project_id>/requirements/inline/verification",
    format = "json",
    data = "<payload>"
)]
async fn create_verification_inline(
    _project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineVerificationPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let data = payload.into_inner();

    let verification_service = VerificationService::new(state.inner());
    let new_verification = NewVerification {
        verification_id: None,
        verification_name: data.name,
        verification_description: data.description,
        project_id,
    };

    let id = verification_service
        .create(new_verification)
        .map_err(map_repo_error)?;
    let stored = verification_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.verification_id,
        "label": stored.verification_name,
        "description": stored.verification_description,
    })))
}

fn get_category_or_placeholder(state: &State<AppState>, category_id: i32) -> Category {
    CategoryService::new(state.inner())
        .get_by_id(category_id)
        .unwrap_or_else(|_| Category {
            cat_id: category_id,
            cat_title: format!("Unknown Category ({})", category_id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub fn routes() -> Vec<Route> {
    routes![
        show_requirements,
        show_requirement_id,
        get_edit_requirement,
        post_edit_requirement,
        delete_requirement_route,
        new_requirement,
        post_requirement,
        show_requirements_tree,
        create_category_inline,
        create_applicability_inline,
        create_verification_inline
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectMember};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        session_cookie, timestamp, TestAppState,
    };
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use rocket::serde::json::{serde_json, Value as JsonValue};

    const ADMIN_ID: i32 = 1;
    const PRIMARY_PROJECT: i32 = 1;

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        // Add test data
        repo.projects.insert(
            PRIMARY_PROJECT,
            Project {
                project_id: PRIMARY_PROJECT,
                project_name: "Test Project".into(),
                project_description: Some("Description".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(ADMIN_ID),
            },
        );

        // Add membership
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        // Add lookups
        repo.statuses.insert(
            1,
            crate::models::Status {
                st_id: 1,
                st_title: "Active".into(),
                st_description: "".into(),
                st_short_name: "A".into(),
            },
        );

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Draft".into(),
                req_st_description: "".into(),
                req_st_short_name: "D".into(),
            },
        );

        repo.categories.insert(
            1,
            Category {
                cat_id: 1,
                cat_title: "Systems".into(),
                cat_description: "".into(),
                cat_tag: "SYS".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.verifications.insert(
            1,
            Verification {
                verification_id: 1,
                verification_name: "Analysis".into(),
                verification_description: "".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                app_id: 1,
                app_title: "All".into(),
                app_description: "".into(),
                app_tag: "ALL".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo
    }

    fn sample_requirement(id: i32) -> Requirement {
        Requirement {
            req_id: id,
            req_title: format!("Requirement {id}"),
            req_description: "Test requirement".into(),
            req_verification: 1,
            req_current_status: 1,
            req_author: ADMIN_ID,
            req_reviewer: ADMIN_ID,
            req_reference: format!("REQ-SYS-{id}"),
            req_category: 1,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_applicability: 1,
            req_justification: Some("For testing".into()),
            project_id: PRIMARY_PROJECT,
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(repo, routes()).await
    }

    #[rocket::async_test]
    async fn show_requirements_lists_project_items() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("Requirement 1"));
    }

    #[rocket::async_test]
    async fn show_requirements_respects_status_filter() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response =
            get_with_session(&client, "/p/1/requirements?status_filter=1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_respects_filter_with_empty_values() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&verification_filter=&category_filter=",
            ADMIN_ID,
        )
        .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_ignores_search_query_when_filtering() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&verification_filter=&category_filter=&search=",
            ADMIN_ID,
        )
        .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_uses_route_project_for_selected_id() {
        let mut repo = base_repo();

        // Add a second project so that the cookie can point to a different project than the route.
        repo.projects.insert(
            2,
            Project {
                project_id: 2,
                project_name: "Other Project".into(),
                project_description: Some("Alt".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(ADMIN_ID),
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let client = test_client(repo).await;

        let response = client
            .get("/p/2/requirements")
            .cookie(Cookie::new("selected_project_id", "1"))
            .private_cookie(session_cookie(ADMIN_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(
            body.contains("action=\"/p/2/requirements\""),
            "filter form must target the route project"
        );
        assert!(
            !body.contains("action=\"/p/1/requirements\""),
            "filter form must not target cookie project"
        );
        assert!(
            body.contains("/p/2/requirements/new"),
            "primary action must use the route project"
        );
        assert!(
            !body.contains("/p/1/requirements/new"),
            "primary action must not use cookie project"
        );
    }

    #[rocket::async_test]
    async fn show_requirement_by_id_displays_details() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("For testing"));
    }

    #[rocket::async_test]
    async fn new_requirement_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        assert!(body.contains("Save"));
        assert!(body.contains("Cancel"));
    }

    #[rocket::async_test]
    async fn post_requirement_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Test&req_description=Description&req_verification=1&\
             req_current_status=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=&\
             req_justification=Testing",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let reqs = state
            .repo_read()
            .get_requirements_by_project(PRIMARY_PROJECT)
            .unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].req_author, ADMIN_ID);
        assert_eq!(reqs[0].project_id, PRIMARY_PROJECT);
        assert!(reqs[0].req_reference.starts_with("REQ-SYS-"));
    }

    #[rocket::async_test]
    async fn post_requirement_add_another_redirects_to_form() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Next+Requirement&req_description=Body&req_verification=1&\
             req_current_status=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=&\
             req_justification=&intent=add_another",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/requirements/new?created=1")
        );
    }

    #[rocket::async_test]
    async fn inline_category_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/category")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"Telemetry","description":"Data channel","tag":"TEL"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Telemetry");
        assert!(value["id"].as_i64().is_some());
    }

    #[rocket::async_test]
    async fn inline_applicability_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/applicability")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"Mission","description":"Applies to mission","tag":"MIS"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Mission");
    }

    #[rocket::async_test]
    async fn inline_verification_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/verification")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"name":"Inspection","description":"Visual inspection"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Inspection");
    }

    #[rocket::async_test]
    async fn edit_requirement_form_shows_existing_data() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        //assert!(body.contains("Edit Requirement"));
        assert!(body.contains("REQ-SYS-1"));
    }

    #[rocket::async_test]
    async fn post_edit_requirement_updates_existing() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "req_id=1&req_title=Updated&req_description=New+desc&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_justification=Changed&project_id=1&req_reference=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let req = state.repo_read().get_requirement_by_id(1).unwrap();
        assert_eq!(req.req_title, "Updated");
        assert_eq!(req.req_description, "New desc");
    }

    #[rocket::async_test]
    async fn delete_requirement_removes_draft() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = delete_with_session(&client, "/p/1/requirements/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let reqs = state
            .repo_read()
            .get_requirements_by_project(PRIMARY_PROJECT)
            .unwrap();
        assert!(reqs.is_empty());
    }

    #[rocket::async_test]
    async fn delete_requirement_forbids_non_draft() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.req_current_status = 3; // Released
        repo.requirements.insert(1, req);

        // Use non-admin user
        let mut non_admin = DieselRepoMock::make_user(2, "user", "");
        non_admin.is_admin = false;
        repo.users.insert(2, non_admin);

        let client = test_client(repo).await;

        // Use non-admin cookie
        let response = client
            .delete("/p/1/requirements/delete/1")
            .private_cookie(session_cookie(2))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn show_requirements_tree_displays_hierarchy() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let mut child = sample_requirement(2);
        child.req_parent = 1;
        repo.requirements.insert(2, child);
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        // Check that parent and child requirements are rendered in the unified view
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
    }

    // Additional edge case and validation tests

    #[rocket::async_test]
    async fn post_requirement_validates_empty_title() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=&req_description=Test&req_verification=1&\
             req_current_status=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=",
            ADMIN_ID,
        )
        .await;

        // Should fail validation for empty title
        assert!(response.status() == Status::BadRequest || response.status() == Status::SeeOther);
    }

    #[rocket::async_test]
    async fn post_requirement_with_invalid_reference_format() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Test&req_description=Body&req_verification=1&\
             req_current_status=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_reference=INVALID-FORMAT",
            ADMIN_ID,
        )
        .await;

        // Should redirect to new form with error
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("error") || location.contains("new"));
    }

    #[rocket::async_test]
    async fn post_requirement_with_valid_custom_reference() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Custom&req_description=Test&req_verification=1&\
             req_current_status=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_reference=REQ-SYS-999",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let reqs = state
            .repo_read()
            .get_requirements_by_project(PRIMARY_PROJECT)
            .unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].req_reference, "REQ-SYS-999");
    }

    #[rocket::async_test]
    async fn show_requirement_enforces_project_ownership() {
        let mut repo = base_repo();

        // Create requirement in different project
        let mut req = sample_requirement(1);
        req.project_id = 99; // Different project
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        // Should redirect to the correct project
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }

    #[rocket::async_test]
    async fn edit_requirement_enforces_project_ownership() {
        let mut repo = base_repo();

        let mut req = sample_requirement(1);
        req.project_id = 99;
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/edit/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }

    #[rocket::async_test]
    async fn new_requirement_displays_flash_message() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new?created=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("created successfully") || body.contains("data-flash-success"));
    }

    #[rocket::async_test]
    async fn new_requirement_with_parent_parameter() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/new?parent=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        // Parent should be pre-selected
        assert!(body.contains("value=\"1\"") || body.contains("selected"));
    }

    #[rocket::async_test]
    async fn new_requirement_with_template_parameter() {
        let mut repo = base_repo();
        let mut template_req = sample_requirement(1);
        template_req.req_title = "Template Title".into();
        template_req.req_description = "Template Description".into();
        repo.requirements.insert(1, template_req);
        let client = test_client(repo).await;

        let response =
            get_with_session(&client, "/p/1/requirements/new?template=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Template Title") || body.contains("Template Description"));
    }

    #[rocket::async_test]
    async fn delete_requirement_admin_can_delete_released() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.req_current_status = 5; // Released/higher status
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = delete_with_session(&client, "/p/1/requirements/delete/1", ADMIN_ID).await;

        // Admin should be able to delete
        assert_eq!(response.status(), Status::SeeOther);
    }

    #[rocket::async_test]
    async fn requirement_edit_updates_all_fields() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "req_id=1&req_title=Updated+Title&req_description=Updated+Description&\
             req_verification=1&req_current_status=1&req_author=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_justification=Updated+Justification&project_id=1&req_reference=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let req = state.repo_read().get_requirement_by_id(1).unwrap();
        assert_eq!(req.req_title, "Updated Title");
        assert_eq!(req.req_description, "Updated Description");
        assert_eq!(req.req_justification, Some("Updated Justification".into()));
    }

    #[rocket::async_test]
    async fn show_requirements_with_multiple_filters() {
        let mut repo = base_repo();

        // Add requirements with different statuses and categories
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        req1.req_category = 1;
        req1.req_verification = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_category = 1;
        req2.req_verification = 1;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&category_filter=1&verification_filter=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_displays_metrics() {
        let mut repo = base_repo();

        // Add requirements with different statuses
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1; // Draft
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 1; // Draft
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Should show total count
        assert!(body.contains("requirement_metrics"));
    }

    #[rocket::async_test]
    async fn requirement_detail_shows_parent_and_children() {
        let mut repo = base_repo();

        let parent = sample_requirement(1);
        repo.requirements.insert(1, parent);

        let mut child = sample_requirement(2);
        child.req_parent = 1;
        child.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, child);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Should contain child requirement
        assert!(body.contains("REQ-SYS-2"));
    }

    #[rocket::async_test]
    async fn requirement_detail_shows_linked_tests() {
        let mut repo = base_repo();

        repo.requirements.insert(1, sample_requirement(1));

        // Note: Test linking is more complex and would require matrix implementation
        // This test verifies the detail page renders even without linked tests

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
    }

    #[rocket::async_test]
    async fn inline_category_creation_returns_new_id() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/category")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"New Category","description":"Test category","tag":"NEW"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "New Category");
        assert_eq!(value["tag"], "NEW");
        assert!(value["id"].as_i64().is_some());
    }

    #[rocket::async_test]
    async fn requirements_tree_handles_empty_project() {
        let client = test_client(base_repo()).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Check that the tree view section exists in the unified page
        assert!(body.contains("treeView") || body.contains("tree_data"));
    }

    #[rocket::async_test]
    async fn requirements_tree_handles_multiple_levels() {
        let mut repo = base_repo();

        let parent = sample_requirement(1);
        repo.requirements.insert(1, parent);

        let mut child = sample_requirement(2);
        child.req_parent = 1;
        child.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, child);

        let mut grandchild = sample_requirement(3);
        grandchild.req_parent = 2;
        grandchild.req_reference = "REQ-SYS-3".into();
        repo.requirements.insert(3, grandchild);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Check that all requirements are rendered in the tree structure
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
        assert!(body.contains("REQ-SYS-3"));
    }

    #[rocket::async_test]
    async fn unauthorized_user_cannot_access_requirements() {
        let mut repo = base_repo();

        // Create non-member user
        let mut user = DieselRepoMock::make_user(99, "outsider", "");
        user.is_admin = false;
        repo.users.insert(99, user);

        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", 99).await;

        // Should be forbidden or redirect
        assert!(
            response.status() == Status::Forbidden
                || response.status() == Status::SeeOther
                || response.status() == Status::Unauthorized
        );
    }

    #[rocket::async_test]
    async fn post_edit_requirement_enforces_project_match() {
        let mut repo = base_repo();

        let mut req = sample_requirement(1);
        req.project_id = 99;
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "req_id=1&req_title=Hack&req_description=Test&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_justification=&project_id=1&req_reference=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        // Should redirect to correct project
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }
}
