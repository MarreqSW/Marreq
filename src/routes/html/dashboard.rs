use super::helpers::{get_user_projects_and_selection, project_status_badge};
use super::prelude::*;
use crate::services::{RequirementService, StatusService, TestService};

#[get("/")]
pub fn index(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();

    let (projects, selected_project_id) = get_user_projects_and_selection(state, &user, cookies);

    let selected_project_name = selected_project_id
        .and_then(|project_id| {
            projects
                .iter()
                .find(|project| project.project_id == project_id)
                .map(|project| project.project_name.clone())
        })
        .unwrap_or_else(|| "Requirements Manager".to_string());

    let requirement_service = RequirementService::new(state.inner());
    let test_service = TestService::new(state.inner());

    let requirements_count = selected_project_id
        .map(|project_id| {
            requirement_service
                .list_by_project(project_id)
                .map(|reqs| reqs.len())
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let tests_count = selected_project_id
        .map(|project_id| {
            test_service
                .list_by_project(project_id)
                .map(|tests| tests.len())
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let user_memberships = state
        .repo_read()
        .get_projects_for_user(user.user_id)
        .unwrap_or_default();

    let membership_map: HashMap<i32, ProjectMember> = user_memberships
        .into_iter()
        .map(|membership| (membership.project_id, membership))
        .collect();

    let user_projects: Vec<_> = projects
        .iter()
        .filter_map(|project| {
            membership_map.get(&project.project_id).map(|membership| {
                let project_status_label = project
                    .project_status
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let status_class = project_status_badge(&project_status_label).to_string();
                let role_label = super::helpers::describe_project_role(membership.role).to_string();
                let role_id = membership.role;

                json!({
                    "project_id": project.project_id,
                    "project_name": project.project_name.clone(),
                    "project_description": project.project_description.clone(),
                    "project_status": project_status_label,
                    "status_class": status_class,
                    "role_label": role_label,
                    "role_id": role_id,
                })
            })
        })
        .collect();

    let user_project_count = user_projects.len();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id,
        "title": "Main",
        "selected_project_name": selected_project_name,
        "requirements_count": requirements_count,
        "tests_count": tests_count,
        "user_projects": user_projects,
        "user_project_count": user_project_count
    });

    Ok(Template::render("index", ctx))
}

#[get("/status")]
pub fn show_status(state: &State<AppState>) -> content::RawHtml<String> {
    let mut out_str = print_header();
    let status_service = StatusService::new(state.inner());

    let all_status = match status_service.list_requirement_statuses() {
        Ok(status_list) => status_list,
        Err(e) => {
            eprintln!("Database query error: {}", e);
            return content::RawHtml("Error: Failed to load status data".to_string());
        }
    };

    for st in all_status.iter() {
        out_str = format!(
            "{}
        <div class='AllStatus'>
            <div>Id: {}</div>
            <div>Title: {}</div>
            <div>Description: {}</div>
        </div>",
            out_str, st.req_st_id, st.req_st_title, st.req_st_description
        );
    }

    out_str = format!("{} {}", out_str, print_footer());
    content::RawHtml(out_str)
}

pub fn routes() -> Vec<Route> {
    routes![index, show_status]
}
