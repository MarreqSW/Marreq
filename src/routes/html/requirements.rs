use std::collections::HashMap;

use chrono::Utc;
use regex::Regex;
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::auth::SessionUser;
use crate::helper_functions::generate_requirement_reference;
use crate::helper_functions::{
    decorate_requirements, filter_requirements, get_selected_project_id,
};
use crate::logger::{LogCtx, Logger};
use crate::models::*;
use crate::repository::{
    LookupRepository, ProjectsRepository, RequirementsRepository, UserRepository,
};

use super::helpers::{
    build_context_with_projects, get_category_by_id_cached, get_db_connection,
    get_linked_tests_for_requirement_cached, get_requirement_by_id_cached_safe,
};
#[get("/requirements?<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_requirements(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let mut ctx = build_context_with_projects(state, user, cookies);

    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);

    let requirements = if let Some(project_id) = selected_project_id {
        state.repo_read().get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_requirements_by_project(first_project.project_id)
        } else {
            state.repo_read().get_requirements_all()
        }
    };

    match requirements {
        Ok(req) => {
            // Apply filters
            let filtered_requirements =
                filter_requirements(req, status_filter, verification_filter, category_filter);
            let requirements_decorate = decorate_requirements(filtered_requirements);
            ctx["requirements"] = json!(requirements_decorate);
        }
        Err(_) => {
            ctx["requirements"] = json!([]);
        }
    };

    // Add filter data to context for the template
    let statuses = state.repo_read().get_status_all().unwrap_or_default();
    let verifications = if let Some(project_id) = selected_project_id {
        state.repo_read().get_verification_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_verification_by_project(first_project.project_id)
        } else {
            state.repo_read().get_verification_all()
        }
    };

    // Get categories filtered by selected project
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };

    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications.unwrap_or_default());
    ctx["categories"] = json!(categories.unwrap_or_default());
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);

    Ok(Template::render("requirements", ctx))
}

#[get("/requirements/<req_id>")]
pub fn show_requirement_id(
    session_user: SessionUser,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();

    // Use the safe function that returns a Result
    match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => {
            let req_decorate = decorate_requirements(vec![req]);

            // Get linked tests for this requirement
            let linked_tests =
                get_linked_tests_for_requirement_cached(state, req_id).unwrap_or_default();
            let linked_tests_json = json!(linked_tests);

            let ctx = json!({
                "requirements": req_decorate,
                "linked_tests": linked_tests_json,
                "user": user
            });

            Ok(Template::render("requirement_by_id", ctx))
        }
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're looking for could not be found.",
                "details": error_msg,
                "user": user
            });

            Ok(Template::render("error", ctx))
        }
    }
}

#[get("/edit_requirement/<req_id>")]
pub fn get_edit_requirement(
    session_user: SessionUser,
    req_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();

    // Use the safe function that returns a Result
    let req = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => req,
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're trying to edit could not be found.",
                "details": error_msg,
                "user": user
            });

            return Ok(Template::render("error", ctx));
        }
    };

    let req_decorate = decorate_requirements(vec![req.clone()]);
    let req_decorate_json = json!(req_decorate[0]);

    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent requirements filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        state.repo_read().get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_requirements_by_project(first_project.project_id)
        } else {
            state.repo_read().get_requirements_all()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = state.repo_read().get_users_all().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        state.repo_read().get_verification_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_verification_by_project(first_project.project_id)
        } else {
            state.repo_read().get_verification_all()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get applicability filtered by project
    let applicability = if let Some(project_id) = selected_project_id {
        state.repo_read().get_applicability_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_applicability_by_project(first_project.project_id)
        } else {
            state.repo_read().get_applicability_all()
        }
    };
    let applicability_json = json!(applicability.unwrap_or_default());

    let ctx = json!({
        "requirements": req_decorate_json,
        "req_author_id": req.req_author,
        "req_reviewer_id": req.req_reviewer,
        "req_category_id": req.req_category,
        "req_applicability_id": req.req_applicability,
        "req_current_status_id": req.req_current_status,
        "req_verification_id": req.req_verification,
        "req_parent_id": req.req_parent,
        "categories": categories_json,
        "status": status_json,
        "parent": parents_json,
        "users": users_json,
        "verification": verification_json,
        "applicability": applicability_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Requirement: {:#}", ctx);
    Ok(Template::render("edit_requirement", ctx))
}

#[post("/edit_requirement/<req_id>", data = "<new_req>")]
pub fn post_edit_requirement(
    session_user: SessionUser,
    req_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();

    let requirement_data = new_req.into_inner();

    // Server-side validation: Check if reference follows general format
    if !requirement_data.req_reference.is_empty() {
        // Check general format: REQ-TAG-NUMBER
        let general_pattern = match Regex::new(r"^REQ-[A-Z]+-\d+$") {
            Ok(pattern) => pattern,
            Err(_) => {
                eprintln!("Failed to compile regex pattern");
                return Err(Redirect::to(uri!(get_edit_requirement(req_id))));
            }
        };
        if !general_pattern.is_match(&requirement_data.req_reference) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(get_edit_requirement(req_id))));
        }

        // Get the category to check if reference matches
        let category = get_category_by_id_cached(state, requirement_data.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);

        // Only warn if reference doesn't match category, but don't block the update
        if !requirement_data.req_reference.starts_with(&expected_prefix) {
            // Log a warning but continue with the update
            println!(
                "Warning: Reference '{}' doesn't match category tag '{}' for requirement {}",
                requirement_data.req_reference, category.cat_tag, req_id
            );
        }
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(post_edit_requirement(req_id)))
    })?;

    // Get the old values before updating
    let old_requirement = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => req,
        Err(_) => {
            // Requirement not found - redirect back to requirements list
            return Err(Redirect::to(uri!(show_requirements(
                None::<i32>,
                None::<i32>,
                None::<i32>
            ))));
        }
    };

    state
        .repo_write()
        .edit_requirement(&requirement_data)
        .map_err(|e| {
            eprintln!("Error editing requirement: {:?}", e);
            Redirect::to(uri!(show_requirements(
                None::<i32>,
                None::<i32>,
                None::<i32>
            )))
        })?;

    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::updated(
        connection,
        &log_ctx,
        &old_requirement,
        &state
            .repo_read()
            .get_requirement_by_id(req_id)
            .expect("Error reading table Requirements after update"),
    );

    Ok(Redirect::to(uri!(show_requirement_id(req_id))))
}

#[delete("/delete_requirement/<req_id>")]
pub fn delete_requirement_route(
    session_user: SessionUser,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = session_user.into_inner();
    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(rocket::http::Status::InternalServerError);
        }
    };

    // Get the requirement details before deleting
    let requirement = match get_requirement_by_id_cached_safe(state, req_id) {
        Ok(req) => req,
        Err(_) => {
            // Requirement not found
            return Err(rocket::http::Status::NotFound);
        }
    };

    // Check if user can delete this requirement
    // Only allow deletion if status is Draft (1) or Proposal (2), or if user is admin
    if requirement.req_current_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    match state.repo_write().delete_requirement(req_id) {
        Ok(deleted) => {
            // Log the requirement deletion
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::deleted(connection.as_mut(), &log_ctx, &deleted);

            // Redirect to requirements list page
            Ok(Redirect::to(uri!(show_requirements(
                None::<i32>,
                None::<i32>,
                None::<i32>
            ))))
        }
        Err(crate::repository::errors::RepoError::NotFound) => Err(rocket::http::Status::NotFound),
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting requirement: {:?}", _e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

#[get("/new_requirement")]
pub fn new_requirement(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent requirements filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        state.repo_read().get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_requirements_by_project(first_project.project_id)
        } else {
            state.repo_read().get_requirements_all()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = state.repo_read().get_users_all().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        state.repo_read().get_verification_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_verification_by_project(first_project.project_id)
        } else {
            state.repo_read().get_verification_all()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get applicability filtered by project
    let applicability = if let Some(project_id) = selected_project_id {
        state.repo_read().get_applicability_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_applicability_by_project(first_project.project_id)
        } else {
            state.repo_read().get_applicability_all()
        }
    };
    let applicability_json = json!(applicability.unwrap_or_default());

    let ctx = json!({
        "categories": categories_json,
        "status": status_json,
        "parent": parents_json,
        "users": users_json,
        "verification": verification_json,
        "applicability": applicability_json,
        "selected_project_id": selected_project_id.unwrap_or(1),
        "req_title": "",
        "req_description": "",
        "req_justification": "",
        "req_reference": "",
        "req_link": "",
        "user": user
    });

    Ok(Template::render("new_requirement", ctx))
}

#[post("/new_requirement", data = "<new_req>")]
pub fn post_requirement(
    session_user: SessionUser,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_requirement))
    })?;

    let mut requirement_data = new_req.into_inner();

    // Server-side validation: Check if reference matches category
    if !requirement_data.req_reference.is_empty() {
        // Get the category to validate the reference
        let category = get_category_by_id_cached(state, requirement_data.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);

        if !requirement_data.req_reference.starts_with(&expected_prefix) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(new_requirement)));
        }

        // Validate format: REQ-TAG-NUMBER
        let reference_pattern = format!("^REQ-{}-\\d+$", category.cat_tag);
        let regex = match Regex::new(&reference_pattern) {
            Ok(pattern) => pattern,
            Err(_) => {
                eprintln!("Failed to compile regex pattern: {}", reference_pattern);
                return Err(Redirect::to(uri!(new_requirement)));
            }
        };
        if !regex.is_match(&requirement_data.req_reference) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(new_requirement)));
        }
    }

    // Generate automatic reference code if not provided
    if requirement_data.req_reference.is_empty() {
        let repo = state.repo_write();
        match generate_requirement_reference(
            &*repo,
            requirement_data.req_category,
            requirement_data.project_id,
        ) {
            Ok(reference) => {
                requirement_data.req_reference = reference;
            }
            Err(_e) => {
                // If generation fails, use a fallback reference
                requirement_data.req_reference = format!("REQ-UNKNOWN-{}", Utc::now().timestamp());
            }
        }
    }

    let req_id = state
        .repo_write()
        .insert_new_requirement(&requirement_data)
        .map_err(|e| {
            eprintln!("Error inserting new requirement: {:?}", e);
            Redirect::to(uri!(show_requirements(
                None::<i32>,
                None::<i32>,
                None::<i32>
            )))
        })?;

    let new_requirement = state
        .repo_read()
        .get_requirement_by_id(req_id)
        .expect("Error reading table Requirements");

    // Log the requirement creation
    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::created(
        connection,
        &log_ctx,
        req_id, // TODO: isn't this redundant?
        &new_requirement,
    );

    Ok(Redirect::to(uri!(show_requirement_id(req_id))))
}

#[get("/requirements/tree")]
pub fn show_requirements_tree(
    session_user: SessionUser,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();

    // Get all requirements
    let all_requirements = state.repo_read().get_requirements_all().unwrap_or_default();

    // Build tree structure
    let mut tree_data = Vec::new();
    let mut children_map: HashMap<i32, Vec<&Requirement>> = HashMap::new();

    // Group requirements by parent
    for req in &all_requirements {
        if req.req_parent == 0 {
            // Root requirements
            tree_data.push(req);
        } else {
            // Child requirements
            children_map
                .entry(req.req_parent)
                .or_insert_with(Vec::new)
                .push(req);
        }
    }

    // Sort requirements by ID
    tree_data.sort_by(|a, b| a.req_id.cmp(&b.req_id));

    // Create tree structure with children
    let mut tree_structure = Vec::new();
    for root_req in tree_data {
        let mut node = json!({
            "requirement": root_req,
            "children": Vec::<serde_json::Value>::new()
        });

        // Add children if any
        if let Some(children) = children_map.get(&root_req.req_id) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| a.req_id.cmp(&b.req_id));

            let children_json: Vec<serde_json::Value> = sorted_children
                .iter()
                .map(|child| {
                    json!({
                        "requirement": child,
                        "children": Vec::<serde_json::Value>::new()
                    })
                })
                .collect();

            node["children"] = json!(children_json);
        }

        tree_structure.push(node);
    }

    let ctx = json!({
        "tree_data": tree_structure,
        "total_requirements": all_requirements.len(),
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
