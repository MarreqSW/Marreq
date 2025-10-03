use std::collections::HashMap;

use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::helper_functions::generate_requirement_reference;
use crate::helper_functions::{
    decorate_requirements, filter_requirements,
};
use crate::logger::{LogCtx, Logger};
use crate::models::*;
use crate::repository::{
    LookupRepository, RequirementsRepository, UserRepository,
};

use super::helpers::{
    build_context_with_projects, get_category_by_id_cached, get_db_connection,
    get_linked_tests_for_requirement_cached, get_requirement_by_id_cached_safe,
};

#[get("/<project_id>/requirements?<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_requirements(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let mut ctx = build_context_with_projects(state, user, cookies);

    // Repo handle once; reuse
    let repo = state.repo_read();

    // Load → filter → decorate in one go; default to empty on error
    let decorated = repo
        .get_requirements_by_project(project_id)
        .map(|reqs| {
            let filtered = filter_requirements(reqs, status_filter, verification_filter, category_filter);
            decorate_requirements(filtered)
        })
        .unwrap_or_default();
    ctx["requirements"] = json!(decorated);

    // Static lists; all default to empty on error
    let statuses      = repo.get_status_all().unwrap_or_default();
    let verifications = repo.get_verification_by_project(project_id).unwrap_or_default();
    let categories    = repo.get_categories_by_project(project_id).unwrap_or_default();

    // Filters for template state
    ctx["statuses"]                    = json!(statuses);
    ctx["verifications"]               = json!(verifications);
    ctx["categories"]                  = json!(categories);
    ctx["current_status_filter"]       = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"]     = json!(category_filter);

    Ok(Template::render("requirements", ctx))
}

#[get("/<project_id>/requirements/show/<req_id>")]
pub fn show_requirement_id(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let requirement = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => req,
        Err(error_msg) => {
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're looking for could not be found.",
                "details": error_msg,
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    // Enforce project ownership
    if requirement.project_id != project_id {
        let reqs_url = uri!(
            "/p",
            show_requirements(
                project_id = requirement.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );

        eprintln!(
            "Project ID mismatch: route {}, requirement {}",
            project_id, requirement.project_id
        );

        return Err(Redirect::to(reqs_url));
    }

    let reqs = decorate_requirements(vec![requirement]);
    let linked_tests =
        get_linked_tests_for_requirement_cached(state, req_id).unwrap_or_default();

    let ctx = json!({
        "requirements": reqs,
        "linked_tests": linked_tests,
        "user": user
    });

    Ok(Template::render("requirement_by_id", ctx))
}

#[get("/<project_id>/requirements/edit/<req_id>")]
pub fn get_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let repo = state.repo_read();

    let req = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(r) => r,
        Err(error_msg) => {
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're trying to edit could not be found.",
                "details": error_msg,
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    // Enforce project ownership; redirect if mismatched
    if req.project_id != project_id {
        eprintln!(
            "Project mismatch on edit: route {}, requirement {}",
            project_id, req.project_id
        );

        let url = uri!(
            "/p",
            show_requirements(
                project_id = req.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );
        return Err(Redirect::to(url));
    }

    // Keep IDs without cloning the whole req later
    let req_author_id         = req.req_author;
    let req_reviewer_id       = req.req_reviewer;
    let req_category_id       = req.req_category;
    let req_applicability_id  = req.req_applicability;
    let req_current_status_id = req.req_current_status;
    let req_verification_id   = req.req_verification;
    let req_parent_id         = req.req_parent;

    // Decorate for the template (single-item vec)
    let requirement_json = json!(decorate_requirements(vec![req]).remove(0));

    // Project-scoped lookups; default to empty on error
    let statuses      = repo.get_status_all().unwrap_or_default();
    let categories    = repo.get_categories_by_project(project_id).unwrap_or_default();
    let parents       = repo.get_requirements_by_project(project_id).unwrap_or_default();
    let users         = repo.get_users_all().unwrap_or_default();
    let verifications = repo.get_verification_by_project(project_id).unwrap_or_default();
    let applicability = repo.get_applicability_by_project(project_id).unwrap_or_default();

    let ctx = json!({
        "requirements": requirement_json,
        "req_author_id": req_author_id,
        "req_reviewer_id": req_reviewer_id,
        "req_category_id": req_category_id,
        "req_applicability_id": req_applicability_id,
        "req_current_status_id": req_current_status_id,
        "req_verification_id": req_verification_id,
        "req_parent_id": req_parent_id,
        "categories": categories,
        "status": statuses,
        "parent": parents,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Edit requirement ctx: {:#}", ctx);

    Ok(Template::render("edit_requirement", ctx))
}

#[post("/<project_id>/requirements/edit/<req_id>", data = "<new_req>")]
pub fn post_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user_id = project_access.into_user().user_id;

    let edit_url = uri!("/p", get_edit_requirement(project_id, req_id));
    let list_url = uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None
        )
    );
    let show_url = uri!("/p", show_requirement_id(project_id, req_id));

    let requirement_data = new_req.into_inner();

    if !requirement_data.req_reference.is_empty() {
        let general_pattern = regex::Regex::new(r"^REQ-[A-Z]+-\d+$").unwrap();
        if !general_pattern.is_match(&requirement_data.req_reference) {
            return Err(Redirect::to(edit_url));
        }

        let category = get_category_by_id_cached(state, requirement_data.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        if !requirement_data.req_reference.starts_with(&expected_prefix) {
            eprintln!(
                "Warning: reference '{}' doesn't match category tag '{}' (req_id={})",
                requirement_data.req_reference, category.cat_tag, req_id
            );
        }
    }

    let old = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => req,
        Err(_) => return Err(Redirect::to(list_url)),
    };

    if old.project_id != project_id {
        let url = uri!(
            "/p",
            show_requirements(
                project_id = old.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );
        return Err(Redirect::to(url));
    }

    state.repo_write().edit_requirement(&requirement_data).map_err(|e| {
        #[cfg(debug_assertions)]
        eprintln!("Error editing requirement {} in project {}: {:?}", req_id, project_id, e);
        Redirect::to(list_url.clone())
    })?;

    if let Ok(mut conn) = get_db_connection(state) {
        if let Ok(new_row) = state.repo_read().get_requirement_by_id(req_id) {
            let log_ctx = LogCtx::new(user_id);
            let _ = Logger::updated(&mut conn, &log_ctx, &old, &new_row);
        }
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/requirements/delete/<req_id>")]
pub fn delete_requirement_route(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();
    let user_id = user.user_id;
    let list_url = uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None
        )
    );

    // 1) Load requirement or 404
    let req = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(r) => r,
        Err(_) => return Err(rocket::http::Status::NotFound),
    };

    // 2) Enforce project ownership; if mismatched, just bounce to the right project’s list
    if req.project_id != project_id {
        let right_list = uri!(
            "/p",
            show_requirements(
                project_id = req.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );
        return Ok(Redirect::to(right_list));
    }

    // 3) Permission gate: allow only Draft(1) or Proposal(2) or admin
    if req.req_current_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    // 4) Delete
    let deleted = match state.repo_write().delete_requirement(req_id) {
        Ok(d) => d,
        Err(crate::repository::errors::RepoError::NotFound) => {
            return Err(rocket::http::Status::NotFound)
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_requirement({}) failed: {:?}", req_id, e);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    // 5) Best-effort logging (don’t affect result)
    if let Ok(mut conn) = get_db_connection(state) {
        let log_ctx = LogCtx::new(user_id);
        let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);
    }

    // 6) Redirect to this project’s requirements
    Ok(Redirect::to(list_url))
}

#[get("/<project_id>/requirements/new")]
pub fn new_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    _cookies: &CookieJar<'_>, // not needed; keep underscored if you can't remove it yet
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let repo = state.repo_read();

    // Project-scoped lookups; default to empty on error
    let statuses      = repo.get_status_all().unwrap_or_default();
    let categories    = repo.get_categories_by_project(project_id).unwrap_or_default();
    let parents       = repo.get_requirements_by_project(project_id).unwrap_or_default();
    let users         = repo.get_users_all().unwrap_or_default();
    let verifications = repo.get_verification_by_project(project_id).unwrap_or_default();
    let applicability = repo.get_applicability_by_project(project_id).unwrap_or_default();

    let ctx = json!({
        "categories": categories,
        "status": statuses,
        "parent": parents,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "project_id": project_id,
        // empty defaults for the form
        "req_title": "",
        "req_description": "",
        "req_justification": "",
        "req_reference": "",
        "req_link": "",
        "user": user
    });

    Ok(Template::render("new_requirement", ctx))
}


#[post("/<project_id>/requirements/new", data = "<new_req>")]
pub fn post_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user_id = project_access.into_user().user_id;

    // Reuse these URLs
    let new_url = uri!("/p", new_requirement(project_id));
    let list_url = uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None
        )
    );

    // Take ownership and enforce project_id from the route
    let mut req = new_req.into_inner();
    req.project_id = project_id;

    // --- Reference validation / generation ---
    if !req.req_reference.is_empty() {
        // Validate against the category’s tag
        let category = get_category_by_id_cached(state, req.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        if !req.req_reference.starts_with(&expected_prefix) {
            return Err(Redirect::to(new_url));
        }

        // Strict pattern: REQ-<CAT_TAG>-<NUMBER>
        // Escape the tag just in case and compile once.
        let pat = format!(r"^REQ-{}-\d+$", regex::escape(&category.cat_tag));
        let re = match regex::Regex::new(&pat) {
            Ok(r) => r,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("regex compile failed for '{}': {:?}", pat, e);
                return Err(Redirect::to(new_url));
            }
        };
        if !re.is_match(&req.req_reference) {
            return Err(Redirect::to(new_url));
        }
    } else {
        // Generate when missing
        match generate_requirement_reference(&*state.repo_write(), req.req_category, req.project_id)
        {
            Ok(reference) => req.req_reference = reference,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("reference generation failed: {:?}", e);
                req.req_reference = format!("REQ-UNKNOWN-{}", chrono::Utc::now().timestamp());
            }
        }
    }

    // --- Insert ---
    let req_id = state.repo_write().insert_new_requirement(&req).map_err(|e| {
        #[cfg(debug_assertions)]
        eprintln!("insert_new_requirement failed: {:?}", e);
        Redirect::to(list_url.clone())
    })?;

    // --- Best-effort logging (don’t affect control flow) ---
    if let (Ok(mut conn), Ok(new_row)) = (
        get_db_connection(state),
        state.repo_read().get_requirement_by_id(req_id),
    ) {
        let log_ctx = LogCtx::new(user_id);
        let _ = Logger::created(&mut conn, &log_ctx, req_id, &new_row);
    }

    // --- Success: show the new requirement ---
    Ok(Redirect::to(uri!("/p", show_requirement_id(project_id, req_id))))
}

#[get("/<project_id>/requirements/tree")]
pub fn show_requirements_tree(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let repo = state.repo_read();

    // Only this project's requirements
    let reqs = repo.get_requirements_by_project(project_id).unwrap_or_default();

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
        let kids = idx.get(&req.req_id).map(|vs| {
            vs.iter().map(|c| build_node(c, idx)).collect::<Vec<_>>()
        }).unwrap_or_default();

        json!({
            "requirement": req,
            "children": kids
        })
    }

    let tree = roots.into_iter().map(|r| build_node(r, &children)).collect::<Vec<_>>();

    let ctx = json!({
        "tree_data": tree,
        "total_requirements": reqs.len(),
        "user": user
    });

    Ok(Template::render("requirements_tree", ctx))
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
        show_requirements_tree
    ]
}
