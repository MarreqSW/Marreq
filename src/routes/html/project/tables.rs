use super::prelude::*;
use crate::services::{
    CategoryService, DecoratedTestService, ProjectService,
    StatusService, UserService, VerificationService,
};

/// Show tests table view for a specific project
#[get("/<project_id>/tests_table?<sort_by>&<sort_order>&<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_tests_table(
    project_access: ProjectAccess,
    project_id: i32,
    sort_by: Option<String>,
    sort_order: Option<String>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    // Get the selected project
    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;

    // Get tests for this project
    let test_service = DecoratedTestService::new(state.inner());
    let mut filtered_tests = test_service.list_by_project(project_id)?;

    // Apply filters
    if let Some(status_id) = status_filter {
        filtered_tests.retain(|t| t.test_status_id == status_id);
    }

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "test_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    filtered_tests.sort_by(|a, b| {
        let comparison = match sort_by.as_str() {
            "test_id" => a.test_id.cmp(&b.test_id),
            "test_name" => a.test_name.cmp(&b.test_name),
            "test_reference" => a.test_reference.cmp(&b.test_reference),
            "test_description" => a.test_description.cmp(&b.test_description),
            "test_status" => a.test_status_id.cmp(&b.test_status_id),
            "test_source" => a.test_source.cmp(&b.test_source),
            "test_parent" => a.test_parent_id.cmp(&b.test_parent_id),
            _ => a.test_id.cmp(&b.test_id),
        };

        if sort_order == "desc" {
            comparison.reverse()
        } else {
            comparison
        }
    });

    // Get lookup data for dropdowns
    let users = UserService::new(state.inner()).get_by_project(project_id)?;
    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let statuses = StatusService::new(state.inner()).list_test_statuses()?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;

    let ctx = json!({
        "user": user,
        "project": json!({
            "id": selected_project.project_id,
            "name": selected_project.project_name,
        }),
        "selected_project_id": project_id,
        "tests": json!(filtered_tests),
        "users": json!(users),
        "categories": json!(categories),
        "statuses": json!(statuses),
        "verifications": json!(verifications),
        "sort_by": json!(sort_by),
        "sort_order": json!(sort_order),
        "current_status_filter": json!(status_filter),
        "current_verification_filter": json!(verification_filter),
        "current_category_filter": json!(category_filter),
    });

    Ok(Template::render("tests_table", ctx))
}

pub fn routes() -> Vec<Route> {
    routes![show_tests_table]
}
