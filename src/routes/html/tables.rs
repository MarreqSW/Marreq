use super::helpers::*;
use super::prelude::*;
use crate::services::{
    CategoryService, ProjectService, RequirementService, StatusService, TestService, UserService,
};

#[get("/requirements_table?<sort_by>&<sort_order>&<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_requirements_table(
    sort_by: Option<String>,
    sort_order: Option<String>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    user: SessionUser,
    state: &State<AppState>,
) -> Result<Template, rocket::http::Status> {
    use crate::helper_functions::decorators::decorate_requirements;

    let _connection =
        get_db_connection(state).map_err(|_| rocket::http::Status::InternalServerError)?;

    // Get all requirements via the service layer
    let requirement_service = RequirementService::new(state.inner());
    let mut filtered_requirements = requirement_service
        .list_all()
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    // Apply filters
    if let Some(status_id) = status_filter {
        filtered_requirements.retain(|r| r.req_current_status == status_id);
    }
    if let Some(verification_id) = verification_filter {
        filtered_requirements.retain(|r| r.req_verification == verification_id);
    }
    if let Some(category_id) = category_filter {
        filtered_requirements.retain(|r| r.req_category == category_id);
    }

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "req_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    filtered_requirements.sort_by(|a, b| {
        let comparison = match sort_by.as_str() {
            "req_id" => a.req_id.cmp(&b.req_id),
            "req_title" => a.req_title.cmp(&b.req_title),
            "req_current_status" => a.req_current_status.cmp(&b.req_current_status),
            "req_verification" => a.req_verification.cmp(&b.req_verification),
            "req_author" => a.req_author.cmp(&b.req_author),
            "req_reviewer" => a.req_reviewer.cmp(&b.req_reviewer),
            "req_category" => a.req_category.cmp(&b.req_category),
            _ => a.req_id.cmp(&b.req_id),
        };

        if sort_order == "desc" {
            comparison.reverse()
        } else {
            comparison
        }
    });

    // Decorate requirements
    let decorated_requirements = decorate_requirements(filtered_requirements);

    // Get lookup data for dropdowns using dedicated services
    let user_service = UserService::new(state.inner());
    let category_service = CategoryService::new(state.inner());
    let status_service = StatusService::new(state.inner());
    let project_service = ProjectService::new(state.inner());

    let users = user_service.list_all().unwrap_or_default();
    let categories = category_service.list_all().unwrap_or_default();
    let statuses = status_service
        .list_requirement_statuses()
        .unwrap_or_default();
    let verifications = state.repo_read().get_verification_all().unwrap_or_default();

    let mut ctx = json!({
        "user": user.0,
        "projects": project_service.list_all().unwrap_or_default(),
        "selected_project_id": 1
    });
    ctx["requirements"] = json!(decorated_requirements);
    ctx["users"] = json!(users);
    ctx["categories"] = json!(categories);
    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications);
    ctx["sort_by"] = json!(sort_by);
    ctx["sort_order"] = json!(sort_order);
    ctx["status_filter"] = json!(status_filter);
    ctx["verification_filter"] = json!(verification_filter);
    ctx["category_filter"] = json!(category_filter);

    Ok(Template::render("requirements_table", ctx))
}

/// Show tests table view
#[get(
    "/tests_table?<sort_by>&<sort_order>&<status_filter>&<verification_filter>&<category_filter>"
)]
pub fn show_tests_table(
    sort_by: Option<String>,
    sort_order: Option<String>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    user: SessionUser,
    state: &State<AppState>,
) -> Result<Template, rocket::http::Status> {
    use crate::helper_functions::decorators::decorate_tests;

    let _connection =
        get_db_connection(state).map_err(|_| rocket::http::Status::InternalServerError)?;

    // Get all tests via the service layer
    let test_service = TestService::new(state.inner());
    let mut filtered_tests = test_service
        .list_all()
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    // Apply filters
    if let Some(status_id) = status_filter {
        filtered_tests.retain(|t| t.test_status == status_id);
    }
    // Note: Test struct doesn't have verification or category fields
    // These filters are not applicable to tests

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "test_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    filtered_tests.sort_by(|a, b| {
        let comparison = match sort_by.as_str() {
            "test_id" => a.test_id.cmp(&b.test_id),
            "test_name" => a.test_name.cmp(&b.test_name),
            "test_status" => a.test_status.cmp(&b.test_status),
            "test_source" => a.test_source.cmp(&b.test_source),
            "test_reference" => a.test_reference.cmp(&b.test_reference),
            "test_parent" => a.test_parent.cmp(&b.test_parent),
            _ => a.test_id.cmp(&b.test_id),
        };

        if sort_order == "desc" {
            comparison.reverse()
        } else {
            comparison
        }
    });

    // Decorate tests
    let decorated_tests = decorate_tests(filtered_tests);

    // Get lookup data for dropdowns using dedicated services
    let user_service = UserService::new(state.inner());
    let category_service = CategoryService::new(state.inner());
    let status_service = StatusService::new(state.inner());
    let project_service = ProjectService::new(state.inner());

    let users = user_service.list_all().unwrap_or_default();
    let categories = category_service.list_all().unwrap_or_default();
    let statuses = status_service.list_test_statuses().unwrap_or_default();
    let verifications = state.repo_read().get_verification_all().unwrap_or_default();

    let mut ctx = json!({
        "user": user.0,
        "projects": project_service.list_all().unwrap_or_default(),
        "selected_project_id": 1
    });
    ctx["tests"] = json!(decorated_tests);
    ctx["users"] = json!(users);
    ctx["categories"] = json!(categories);
    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications);
    ctx["sort_by"] = json!(sort_by);
    ctx["sort_order"] = json!(sort_order);
    ctx["status_filter"] = json!(status_filter);
    ctx["verification_filter"] = json!(verification_filter);
    ctx["category_filter"] = json!(category_filter);

    Ok(Template::render("tests_table", ctx))
}
