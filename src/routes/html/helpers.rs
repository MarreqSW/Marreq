use std::collections::HashMap;

use chrono::Utc;
use rocket::http::CookieJar;
use rocket::serde::json::{json, Value};
use rocket::State;

use crate::app::AppState;
use crate::helper_functions::{
    decorators::{decorate_tests_with_repo, get_linked_tests_for_requirement_with_repo},
    get_selected_project_id,
};
use crate::models::{Category, DecoratedTest, Project, ProjectMember, Requirement, Test, User};
use crate::repository::errors::RepoError;
use crate::repository::PooledConnectionWrapper;
use crate::repository::{
    LookupRepository, ProjectMembersRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};
use crate::services::project_service::ProjectService;

/// Helper function to get a database connection with proper error handling
pub(crate) fn get_db_connection(
    state: &AppState,
) -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    state
        .repo_read()
        .inner_repo()
        .get_conn()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

pub(crate) fn get_accessible_projects(state: &AppState, user: &User) -> Vec<Project> {
    let repo = state.repo_read();

    if user.is_admin {
        let mut projects = repo.get_projects_all().unwrap_or_default();
        projects.sort_by(|a, b| {
            a.project_name
                .to_lowercase()
                .cmp(&b.project_name.to_lowercase())
        });
        return projects;
    }

    let memberships = repo.get_projects_for_user(user.user_id).unwrap_or_default();

    if memberships.is_empty() {
        return Vec::new();
    }

    let mut projects: Vec<Project> = memberships
        .into_iter()
        .filter_map(|membership| repo.get_project_by_id(membership.project_id).ok())
        .collect();

    projects.sort_by(|a, b| {
        a.project_name
            .to_lowercase()
            .cmp(&b.project_name.to_lowercase())
    });
    projects
}

pub(crate) fn resolve_selected_project_id(
    requested: Option<i32>,
    projects: &[Project],
) -> Option<i32> {
    match requested {
        Some(project_id)
            if projects
                .iter()
                .any(|project| project.project_id == project_id) =>
        {
            Some(project_id)
        }
        _ => projects.first().map(|project| project.project_id),
    }
}

pub(crate) fn get_user_projects_and_selection(
    state: &AppState,
    user: &User,
    cookies: &CookieJar<'_>,
) -> (Vec<Project>, Option<i32>) {
    let projects = get_accessible_projects(state, user);
    let requested = get_selected_project_id(cookies);
    let selected_project_id = resolve_selected_project_id(requested, &projects);
    (projects, selected_project_id)
}

pub(crate) fn build_context_with_projects(
    state: &AppState,
    user: User,
    cookies: &CookieJar<'_>,
) -> rocket::serde::json::Value {
    let (projects, selected_project_id) = get_user_projects_and_selection(state, &user, cookies);

    json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    })
}

pub(crate) fn decorate_projects_for_listing(
    state: &AppState,
    user: &User,
    projects: &[Project],
) -> Vec<Value> {
    let repo = state.repo_read();

    let membership_by_project: HashMap<i32, ProjectMember> = repo
        .get_projects_for_user(user.user_id)
        .unwrap_or_default()
        .into_iter()
        .map(|membership| (membership.project_id, membership))
        .collect();

    let owner_lookup: HashMap<i32, String> = repo
        .get_users_all()
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.user_id, u.user_name))
        .collect();

    let mut decorated: Vec<Value> = Vec::with_capacity(projects.len());

    for project in projects {
        if !user.is_admin && !membership_by_project.contains_key(&project.project_id) {
            continue;
        }

        let role_label = membership_by_project
            .get(&project.project_id)
            .map(|membership| describe_project_role(membership.role).to_string())
            .or_else(|| {
                if user.is_admin {
                    Some("Administrator".to_string())
                } else {
                    None
                }
            });

        let role_id = membership_by_project
            .get(&project.project_id)
            .map(|membership| membership.role);

        let owner_name = project
            .project_owner_id
            .and_then(|owner_id| owner_lookup.get(&owner_id).cloned());

        let status_original = project
            .project_status
            .as_ref()
            .map(|status| status.trim().to_string());
        let status_display = status_original
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        let status_normalized = status_original
            .as_ref()
            .map(|status| status.to_ascii_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let status_badge = project_status_badge(status_display.as_str());

        decorated.push(json!({
            "project_id": project.project_id,
            "project_name": project.project_name,
            "project_description": project.project_description,
            "project_creation_date": project
                .project_creation_date
                .map(|dt| dt.format("%Y-%m-%d").to_string()),
            "project_update_date": project
                .project_update_date
                .map(|dt| dt.format("%Y-%m-%d").to_string()),
            "project_status": status_display,
            "project_status_normalized": status_normalized,
            "project_status_badge": status_badge,
            "project_owner_id": project.project_owner_id,
            "project_owner_name": owner_name,
            "role_label": role_label,
            "role_id": role_id
        }));
    }

    decorated.sort_by(|a, b| {
        let a_name = a
            .get("project_name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_lowercase();
        let b_name = b
            .get("project_name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_lowercase();
        a_name.cmp(&b_name)
    });

    decorated
}

pub(crate) fn decorate_tests_cached(state: &AppState, tests: Vec<Test>) -> Vec<DecoratedTest> {
    let repo = state.repo_read();
    decorate_tests_with_repo(&*repo, tests)
}

pub(crate) fn describe_project_role(role: i32) -> &'static str {
    match role {
        1 => "Owner",
        2 => "Manager",
        3 => "Contributor",
        4 => "Viewer",
        _ => "Member",
    }
}

pub(crate) fn project_status_badge(status: &str) -> &'static str {
    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "active" => "bg-success",
        "archived" | "inactive" => "bg-secondary",
        "on hold" | "paused" | "maintenance" => "bg-warning",
        _ => "bg-secondary",
    }
}

pub(crate) fn get_requirement_by_id_cached_safe(
    state: &AppState,
    id: i32,
) -> Result<Requirement, String> {
    state
        .repo_read()
        .get_requirement_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Requirement with ID {} not found", id),
            _ => e.to_string(),
        })
}

pub(crate) fn get_category_by_id_cached(state: &AppState, id: i32) -> Category {
    state
        .repo_read()
        .get_category_by_id(id)
        .unwrap_or_else(|_| Category {
            cat_id: id,
            cat_title: format!("Unknown Category ({})", id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub(crate) fn get_status_name_by_id_cached(state: &AppState, id: i32) -> String {
    state
        .repo_read()
        .get_status_by_id(id)
        .map(|s| s.st_title)
        .unwrap_or_else(|_| "[Status Not Found]".to_string())
}

pub(crate) fn get_linked_tests_for_requirement_cached(
    state: &AppState,
    req_id: i32,
) -> Result<Vec<DecoratedTest>, String> {
    let repo = state.repo_read();
    get_linked_tests_for_requirement_with_repo(&*repo, req_id).map_err(|e| e.to_string())
}

pub(crate) fn get_requirements_for_test_cached(
    state: &AppState,
    test_id: i32,
) -> Result<Vec<Requirement>, String> {
    state
        .repo_read()
        .get_requirements_for_test(test_id)
        .map_err(|e| e.to_string())
}

/// Get project by ID with safe fallback using the repository.
pub(crate) fn get_project_by_id_pooled_safe(state: &State<AppState>, project_id: i32) -> Project {
    ProjectService::new(state.inner())
        .get_by_id(project_id)
        .unwrap_or(Project {
            project_id: 0,
            project_name: "Unknown Project".to_string(),
            project_description: Some("Unknown project".to_string()),
            project_creation_date: Some(Utc::now().naive_utc()),
            project_update_date: Some(Utc::now().naive_utc()),
            project_status: Some("Unknown".to_string()),
            project_owner_id: Some(0),
        })
}
