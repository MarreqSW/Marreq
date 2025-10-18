use std::collections::HashMap;

use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::serde::json::{json, serde_json};
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::helper_functions::generate_requirement_reference;
use crate::helper_functions::decorators::decorate_requirements_with_repo;
use crate::models::*;
use crate::repository::{LookupRepository, UserRepository};
use crate::services::{ProjectService, RequirementService};

use super::helpers::{
    build_context_with_projects, get_category_by_id_cached,
    get_requirement_by_id_cached_safe,
};

#[get("/<project_id>/requirements?<status_filter>&<verification_filter>&<category_filter>")]
async fn show_requirements(
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

    let selected_project = ProjectService::new(state.inner())
        .get_by_id(project_id)
        .unwrap();

    cookies.add(Cookie::new(
        "selected_project_id",
        project_id.to_string(),
    ));
    ctx["selected_project_id"] = json!(project_id);

    let filtered = RequirementService::new(state.inner()).list_by_project_filtered(project_id, status_filter, verification_filter, category_filter).unwrap_or_default();

    let repo = state.repo_read();
    let decorated = decorate_requirements_with_repo(&*repo, filtered);

    let total_requirements = decorated.len();
    let mut status_totals: HashMap<String, usize> = HashMap::new();

    for requirement in &decorated {
        let key = requirement.req_current_status.trim().to_ascii_lowercase();
        *status_totals.entry(key).or_default() += 1;
    }

    let draft_count = *status_totals.get("draft").unwrap_or(&0);
    let accepted_count = *status_totals.get("accepted").unwrap_or(&0);
    let rejected_count = *status_totals.get("rejected").unwrap_or(&0);

    let coverage_ratio = if total_requirements > 0 {
        accepted_count as f64 / total_requirements as f64
    } else {
        0.0
    };
    let coverage_percent = (coverage_ratio * 100.0).round() as i32;

    // Static lists; all default to empty on error
    let statuses = repo.get_status_all().unwrap_or_default();
    let verifications = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default();
    let categories = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default();

    let status_lookup: HashMap<i32, String> = statuses
        .iter()
        .map(|s| (s.st_id, s.st_title.clone()))
        .collect();
    let verification_lookup: HashMap<i32, String> = verifications
        .iter()
        .map(|v| (v.verification_id, v.verification_name.clone()))
        .collect();
    let category_lookup: HashMap<i32, String> = categories
        .iter()
        .map(|c| (c.cat_id, c.cat_title.clone()))
        .collect();

    let status_label = status_filter.and_then(|id| status_lookup.get(&id).cloned());
    let verification_label =
        verification_filter.and_then(|id| verification_lookup.get(&id).cloned());
    let category_label = category_filter.and_then(|id| category_lookup.get(&id).cloned());

    let mut active_filters = Vec::new();
    if let Some(label) = status_label.clone() {
        active_filters.push(json!({
            "key": "status_filter",
            "label": format!("Status: {label}")
        }));
    }
    if let Some(label) = verification_label.clone() {
        active_filters.push(json!({
            "key": "verification_filter",
            "label": format!("Verification: {label}")
        }));
    }
    if let Some(label) = category_label.clone() {
        active_filters.push(json!({
            "key": "category_filter",
            "label": format!("Category: {label}")
        }));
    }

    ctx["requirements"] = json!(decorated);
    ctx["requirement_metrics"] = json!({
        "total": total_requirements,
        "draft": draft_count,
        "accepted": accepted_count,
        "rejected": rejected_count,
        "coverage": {
            "verified": accepted_count,
            "percent": coverage_percent
        }
    });

    // Filters for template state
    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications);
    ctx["categories"] = json!(categories);
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);
    ctx["active_filters"] = json!(active_filters);
    ctx["project"] = json!({
        "id": selected_project.project_id,
        "name": selected_project.project_name,
        "status": selected_project.project_status,
        "description": selected_project.project_description,
    });

    Ok(Template::render("requirements", ctx))
}

#[get("/<project_id>/requirements/show/<req_id>")]
async fn show_requirement_id(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let service = RequirementService::new(state.inner());

    let requirement = match service.get_by_id(req_id) {
        Ok(req) => req,
        Err(crate::repository::errors::RepoError::NotFound) => {
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're looking for could not be found.",
                "details": "The specified requirement does not exist.",
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
        Err(err) => {
            let ctx = json!({
                "title": "Error Loading Requirement",
                "message": "An error occurred while loading the requirement.",
                "details": format!("{:?}", err),
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

    let reqs = {
        let repo = state.repo_read();
        decorate_requirements_with_repo(&*repo, vec![requirement])
    };

    let linked_tests = service.get_linked_tests(req_id).unwrap_or_default();

    let ctx = json!({
        "requirements": reqs,
        "linked_tests": linked_tests,
        "user": user
    });

    Ok(Template::render("requirement_by_id", ctx))
}

#[get("/<project_id>/requirements/edit/<req_id>")]
async fn get_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let service = RequirementService::new(state.inner());

    let req = match service.get_by_id_decorated(req_id) {
        Ok(req) => req,
        Err(err) => {
            let ctx = json!({
                "title": "Error Loading Requirement",
                "message": "An error occurred while loading the requirement.",
                "details": format!("{:?}", err),
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

    let parent: Option<Requirement> = if req.req_parent_id != 0 {
        match service.get_by_id(req.req_parent_id) {
            Ok(r) => Some(r),
            Err(_) => None,
        }
    } else {
        None
    };

    let repo = state.repo_read();

    // Project-scoped lookups; default to empty on error
    let statuses = repo.get_status_all().unwrap_or_default();
    let categories = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    let verifications = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default();
    let applicability = repo
        .get_applicability_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "req_author_id": req.req_author_id,
        "req_reviewer_id": req.req_reviewer,
        "req_category_id": req.req_category,
        "req_applicability_id": req.req_applicability,
        "req_current_status_id": req.req_current_status,
        "req_verification_id": req.req_verification,
        "req_parent_id": req.req_parent_id,
        "categories": categories,
        "status": statuses,
        "parent": parent,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "user": user,
        "requirement": json!(req)
    });

    #[cfg(debug_assertions)]
    println!("Edit requirement ctx: {:#}", ctx);

    Ok(Template::render("edit_requirement", ctx))
}

#[post("/<project_id>/requirements/edit/<req_id>", data = "<new_req>")]
async fn post_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

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

    let service = RequirementService::new(state.inner());
    match service.update(&user, req_id, new_req.into_inner()) {
        Ok(_) => {}
        Err(crate::repository::errors::RepoError::NotFound) => return Err(Redirect::to(list_url)),
        Err(crate::repository::errors::RepoError::BadInput(_)) => {
            return Err(Redirect::to(edit_url))
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Error editing requirement {} in project {}: {:?}",
                req_id, project_id, _err
            );
            return Err(Redirect::to(list_url));
        }
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/requirements/delete/<req_id>")]
async fn delete_requirement_route(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();
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
    let service = RequirementService::new(state.inner());
    match service.delete(&user, req_id) {
        Ok(_) => {}
        Err(crate::repository::errors::RepoError::NotFound) => {
            return Err(rocket::http::Status::NotFound)
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_requirement({}) failed: {:?}", req_id, _err);
            return Err(rocket::http::Status::InternalServerError);
        }
    }

    // 5) Redirect to this project’s requirements
    Ok(Redirect::to(list_url))
}

#[get("/<project_id>/requirements/new?<error>")]
async fn new_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    _cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let parents = match RequirementService::new(state.inner()).list_by_project(project_id) {
        Ok(reqs) => reqs,
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to load parent requirements for project {}: {:?}",
                project_id, _err
            );
            Vec::new()
        }
    };

    let repo = state.repo_read();
    // Project-scoped lookups; default to empty on error
    let statuses = repo.get_status_all().unwrap_or_default();
    let categories = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    let verifications = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default();
    let applicability = repo
        .get_applicability_by_project(project_id)
        .unwrap_or_default();

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
        "user": user,
        "error": error
    });

    Ok(Template::render("new_requirement", ctx))
}

#[post("/<project_id>/requirements/new", data = "<new_req>")]
async fn post_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    // Reuse these URLs
    let new_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Invalid data provided".to_string())
        )
    );

    // Take ownership and enforce project_id from the route
    let mut req = new_req.into_inner();
    req.project_id = project_id;

    // --- Reference validation / generation ---
    if !req.req_reference.is_empty() {
        // Validate against the category's tag
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
            error = Some("Failed to create requirement".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectMember};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        session_cookie, timestamp, TestAppState,
    };
    use rocket::http::{Cookie, Status};
    use rocket::local::asynchronous::Client;

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
            req_link: "".into(),
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

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1",
            ADMIN_ID,
        )
        .await;
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
    async fn show_requirement_by_id_redirects_on_project_mismatch() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.project_id = 2;
        repo.requirements.insert(1, req);
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/requirements")
        );
    }

    #[rocket::async_test]
    async fn new_requirement_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        assert!(body.contains("Create Requirement"));
    }

    #[rocket::async_test]
    async fn post_requirement_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Test&req_description=Description&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&req_link=&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=&\
             req_justification=Testing&project_id=1",
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
        assert!(reqs[0].req_reference.starts_with("REQ-SYS-"));
    }

    #[rocket::async_test]
    async fn edit_requirement_form_shows_existing_data() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Edit Requirement"));
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
             req_current_status=1&req_author=1&req_reviewer=1&req_link=&\
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

        let response = get_with_session(&client, "/p/1/requirements/tree", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
    }
}
