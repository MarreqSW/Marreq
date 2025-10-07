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

pub fn routes() -> Vec<Route> {
    routes![show_requirements_table, show_tests_table]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{
        Category, Project, ProjectMember, Requirement, RequirementStatus, Test, TestStatus,
        Verification,
    };
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{Cookie, Status};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const USER_ID: i32 = 2;
    const PROJECT_ID: i32 = 1;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn admin_user() -> crate::models::User {
        let mut user = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        user.is_admin = true;
        user.user_name = "Admin User".into();
        user.user_email = "admin@example.com".into();
        user
    }

    fn standard_user() -> crate::models::User {
        let mut user = DieselRepoMock::make_user(USER_ID, "jane", "");
        user.user_name = "Jane Doe".into();
        user.user_email = "jane@example.com".into();
        user
    }

    fn sample_project() -> Project {
        Project {
            project_id: PROJECT_ID,
            project_name: "Test Project".to_string(),
            project_description: Some("Test description".to_string()),
            project_creation_date: Some(timestamp()),
            project_update_date: Some(timestamp()),
            project_status: Some("Active".to_string()),
            project_owner_id: Some(ADMIN_ID),
        }
    }

    fn sample_category(id: i32, title: &str) -> Category {
        Category {
            cat_id: id,
            cat_title: title.to_string(),
            cat_description: format!("{} category", title),
            cat_tag: title.to_ascii_uppercase(),
            project_id: PROJECT_ID,
        }
    }

    fn sample_requirement_status(id: i32, title: &str) -> RequirementStatus {
        RequirementStatus {
            req_st_id: id,
            req_st_title: title.to_string(),
            req_st_description: format!("{} status", title),
            req_st_short_name: title.chars().take(3).collect(),
        }
    }

    fn sample_test_status(id: i32, title: &str) -> TestStatus {
        TestStatus {
            test_st_id: id,
            test_st_title: title.to_string(),
            test_st_description: format!("{} status", title),
            test_st_short_name: title.chars().take(3).collect(),
        }
    }

    fn sample_verification(id: i32, name: &str) -> Verification {
        Verification {
            verification_id: id,
            verification_name: name.to_string(),
            verification_description: format!("{} verification", name),
            project_id: PROJECT_ID,
        }
    }

    fn sample_requirement(id: i32, title: &str, status: i32, category: i32) -> Requirement {
        Requirement {
            req_id: id,
            req_title: title.to_string(),
            req_description: format!("Description for {}", title),
            req_verification: 1,
            req_current_status: status,
            req_author: ADMIN_ID,
            req_reviewer: USER_ID,
            req_link: format!("https://example.com/{}", id),
            req_reference: format!("REQ-{:03}", id),
            req_category: category,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_applicability: 1,
            req_justification: Some("Test justification".to_string()),
            project_id: PROJECT_ID,
        }
    }

    fn sample_test(id: i32, name: &str, status: i32) -> Test {
        Test {
            test_id: id,
            test_name: name.to_string(),
            test_description: format!("Description for {}", name),
            test_source: "Test source".to_string(),
            test_status: status,
            test_reference: format!("TEST-{:03}", id),
            test_parent: 0,
            project_id: PROJECT_ID,
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        repo.users.insert(ADMIN_ID, admin_user());
        repo.users.insert(USER_ID, standard_user());

        repo.projects.insert(PROJECT_ID, sample_project());

        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.requirement_statuses
            .insert(1, sample_requirement_status(1, "Draft"));
        repo.requirement_statuses
            .insert(2, sample_requirement_status(2, "Review"));
        repo.requirement_statuses
            .insert(3, sample_requirement_status(3, "Approved"));

        repo.test_statuses.insert(1, sample_test_status(1, "Draft"));
        repo.test_statuses
            .insert(2, sample_test_status(2, "Active"));
        repo.test_statuses
            .insert(3, sample_test_status(3, "Passed"));

        repo.categories.insert(1, sample_category(1, "Systems"));
        repo.categories.insert(2, sample_category(2, "Software"));

        repo.verifications
            .insert(1, sample_verification(1, "Analysis"));
        repo.verifications
            .insert(2, sample_verification(2, "Testing"));

        repo
    }

    fn repo_with_requirements() -> DieselRepoMock {
        let mut repo = base_repo();

        repo.requirements
            .insert(1, sample_requirement(1, "Req Alpha", 1, 1));
        repo.requirements
            .insert(2, sample_requirement(2, "Req Beta", 2, 1));
        repo.requirements
            .insert(3, sample_requirement(3, "Req Gamma", 3, 2));

        repo
    }

    fn repo_with_tests() -> DieselRepoMock {
        let mut repo = base_repo();

        repo.tests.insert(1, sample_test(1, "Test Alpha", 1));
        repo.tests.insert(2, sample_test(2, "Test Beta", 2));
        repo.tests.insert(3, sample_test(3, "Test Gamma", 3));

        repo
    }

    fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .attach(Template::fairing())
            .mount("/", routes![show_requirements_table, show_tests_table]);
        Client::tracked(rocket).await.expect("client")
    }

    fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie
    }

    async fn get_with_session<'c>(
        client: &'c Client,
        path: &'c str,
        user_id: i32,
    ) -> LocalResponse<'c> {
        client
            .get(path)
            .private_cookie(session_cookie(user_id))
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn show_requirements_table_renders_all_requirements() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(&client, "/requirements_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Req Alpha"));
        assert!(body.contains("Req Beta"));
        assert!(body.contains("Req Gamma"));
        assert!(body.contains("REQ-001"));
    }

    #[rocket::async_test]
    async fn show_requirements_table_filters_by_status() {
        let client = test_client(repo_with_requirements()).await;
        let response =
            get_with_session(&client, "/requirements_table?status_filter=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Req Alpha"));
        assert!(!body.contains("Req Beta"));
        assert!(!body.contains("Req Gamma"));
    }

    #[rocket::async_test]
    async fn show_requirements_table_filters_by_category() {
        let client = test_client(repo_with_requirements()).await;
        let response =
            get_with_session(&client, "/requirements_table?category_filter=2", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(!body.contains("Req Alpha"));
        assert!(!body.contains("Req Beta"));
        assert!(body.contains("Req Gamma"));
    }

    #[rocket::async_test]
    async fn show_requirements_table_sorts_by_title_ascending() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(
            &client,
            "/requirements_table?sort_by=req_title&sort_order=asc",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let alpha_pos = body.find("Req Alpha").unwrap();
        let beta_pos = body.find("Req Beta").unwrap();
        let gamma_pos = body.find("Req Gamma").unwrap();
        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < gamma_pos);
    }

    #[rocket::async_test]
    async fn show_requirements_table_sorts_by_title_descending() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(
            &client,
            "/requirements_table?sort_by=req_title&sort_order=desc",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let alpha_pos = body.find("Req Alpha").unwrap();
        let beta_pos = body.find("Req Beta").unwrap();
        let gamma_pos = body.find("Req Gamma").unwrap();
        assert!(gamma_pos < beta_pos);
        assert!(beta_pos < alpha_pos);
    }

    #[rocket::async_test]
    async fn show_requirements_table_includes_filter_dropdowns() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(&client, "/requirements_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Systems"));
        assert!(body.contains("Software"));
        assert!(body.contains("Draft"));
        assert!(body.contains("Review"));
        assert!(body.contains("Analysis"));
    }

    #[rocket::async_test]
    async fn show_tests_table_renders_all_tests() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/tests_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Test Alpha"));
        assert!(body.contains("Test Beta"));
        assert!(body.contains("Test Gamma"));
        assert!(body.contains("TEST-001"));
    }

    #[rocket::async_test]
    async fn show_tests_table_filters_by_status() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/tests_table?status_filter=2", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(!body.contains("Test Alpha"));
        assert!(body.contains("Test Beta"));
        assert!(!body.contains("Test Gamma"));
    }

    #[rocket::async_test]
    async fn show_tests_table_sorts_by_name_ascending() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(
            &client,
            "/tests_table?sort_by=test_name&sort_order=asc",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let alpha_pos = body.find("Test Alpha").unwrap();
        let beta_pos = body.find("Test Beta").unwrap();
        let gamma_pos = body.find("Test Gamma").unwrap();
        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < gamma_pos);
    }

    #[rocket::async_test]
    async fn show_tests_table_sorts_by_name_descending() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(
            &client,
            "/tests_table?sort_by=test_name&sort_order=desc",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let alpha_pos = body.find("Test Alpha").unwrap();
        let beta_pos = body.find("Test Beta").unwrap();
        let gamma_pos = body.find("Test Gamma").unwrap();
        assert!(gamma_pos < beta_pos);
        assert!(beta_pos < alpha_pos);
    }

    #[rocket::async_test]
    async fn show_tests_table_includes_status_dropdown() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/tests_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Draft"));
        assert!(body.contains("Active"));
        assert!(body.contains("Passed"));
    }

    #[rocket::async_test]
    async fn show_requirements_table_combines_filters_and_sorting() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(
            &client,
            "/requirements_table?status_filter=1&sort_by=req_title&sort_order=asc",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Req Alpha"));
        assert!(!body.contains("Req Beta"));
    }

    #[rocket::async_test]
    async fn show_tests_table_works_with_standard_user() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/tests_table", USER_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Test Alpha"));
    }

    #[rocket::async_test]
    async fn show_requirements_table_defaults_to_id_sort_ascending() {
        let client = test_client(repo_with_requirements()).await;
        let response = get_with_session(&client, "/requirements_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let req1_pos = body.find("REQ-001").unwrap();
        let req2_pos = body.find("REQ-002").unwrap();
        let req3_pos = body.find("REQ-003").unwrap();
        assert!(req1_pos < req2_pos);
        assert!(req2_pos < req3_pos);
    }

    #[rocket::async_test]
    async fn show_tests_table_defaults_to_id_sort_ascending() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/tests_table", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        let test1_pos = body.find("TEST-001").unwrap();
        let test2_pos = body.find("TEST-002").unwrap();
        let test3_pos = body.find("TEST-003").unwrap();
        assert!(test1_pos < test2_pos);
        assert!(test2_pos < test3_pos);
    }
}
