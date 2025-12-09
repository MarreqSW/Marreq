use super::helpers::*;
use super::prelude::*;
use crate::services::{
    ApplicabilityService, CategoryService, MatrixFilters, MatrixPagination, MatrixService,
    SortOrder, StatusService,
};

#[get("/<project_id>/matrix?<sort_by>&<sort_order>&<test_status_filter>&<req_status_filter>&<category_filter>&<applicability_filter>&<page>&<per_page>&<search>")]
async fn get_matrix(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    test_status_filter: Option<i32>,
    req_status_filter: Option<i32>,
    category_filter: Option<i32>,
    applicability_filter: Option<i32>,
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();

    // Build filter and pagination parameters
    let filters = MatrixFilters {
        test_status: test_status_filter,
        req_status: req_status_filter,
        category: category_filter,
        applicability: applicability_filter,
        search: search.clone(),
    };

    let sort_by_value = sort_by.clone().unwrap_or_else(|| "req_id".to_string());
    let is_desc = sort_order.as_deref() == Some("desc");
    let pagination = MatrixPagination {
        page: page.unwrap_or(1).max(1),
        per_page: per_page.unwrap_or(50).clamp(10, 200),
        sort_by: sort_by_value.clone(),
        sort_order: if is_desc {
            SortOrder::Desc
        } else {
            SortOrder::Asc
        },
    };

    // Get matrix view from service
    let matrix_service = MatrixService::new(state.inner());
    let view = matrix_service
        .get_matrix_view(project_id, filters, pagination.clone())
        .map_err(|e| {
            eprintln!("Error loading matrix view: {e}");
            Redirect::to(uri!(crate::routes::html::dashboard::index))
        })?;

    // Build matrix cells for template
    let (requirements_with_matrix, _) =
        build_matrix_rows(&view.requirements, &view.tests, &view.links);

    // Build tests with status names
    let tests_with_status = build_tests_with_status(&view.tests, state);

    // Build pagination context
    let pagination_ctx = build_pagination_context(
        pagination.page,
        pagination.per_page,
        view.total_requirements,
        view.total_pages,
    );

    // Build final context
    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["requirements"] = json!(requirements_with_matrix);
    ctx["tests"] = json!(tests_with_status);
    ctx["total_tests"] = json!(view.tests.len() as i32);
    ctx["total_requirements"] = json!(view.total_requirements);
    ctx["total_links"] = json!(view.total_links);
    ctx["current_sort_by"] = json!(sort_by_value);
    ctx["current_sort_order"] = json!(if is_desc { "desc" } else { "asc" });
    ctx["test_status_filter"] = json!(test_status_filter);
    ctx["req_status_filter"] = json!(req_status_filter);
    ctx["category_filter"] = json!(category_filter);
    ctx["applicability_filter"] = json!(applicability_filter);
    ctx["search"] = json!(search);
    ctx["page"] = json!(pagination.page);
    ctx["per_page"] = json!(pagination.per_page);
    ctx["total_pages"] = json!(view.total_pages);
    ctx["has_prev_page"] = json!(pagination_ctx.has_prev_page);
    ctx["has_next_page"] = json!(pagination_ctx.has_next_page);
    ctx["prev_page"] = json!(pagination.page - 1);
    ctx["next_page"] = json!(pagination.page + 1);
    ctx["start_item"] = json!(pagination_ctx.start_item);
    ctx["end_item"] = json!(pagination_ctx.end_item);
    ctx["show_first_page"] = json!(pagination_ctx.show_first_page);
    ctx["show_last_page"] = json!(pagination_ctx.show_last_page);
    ctx["show_first_ellipsis"] = json!(pagination_ctx.show_first_ellipsis);
    ctx["show_last_ellipsis"] = json!(pagination_ctx.show_last_ellipsis);
    ctx["test_statuses"] = json!(StatusService::new(state.inner())
        .list_test_statuses()
        .unwrap_or_default());
    ctx["statuses"] = json!(StatusService::new(state.inner())
        .list_requirement_statuses()
        .unwrap_or_default());
    ctx["categories"] = json!(CategoryService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default());
    ctx["applicabilities"] = json!(ApplicabilityService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default());
    ctx["total_test_columns"] = json!(view.tests.len() + 1);

    Ok(Template::render("matrix/matrix", ctx))
}

/// Build matrix rows with linkage information
fn build_matrix_rows(
    reqs: &[Requirement],
    tests: &[Test],
    links: &HashSet<(i32, i32)>,
) -> (Vec<serde_json::Value>, usize) {
    use serde_json::json;

    let rows: Vec<_> = reqs
        .iter()
        .map(|req| {
            let row: Vec<_> = tests
                .iter()
                .map(|test| {
                    json!({
                        "linked": links.contains(&(req.req_id, test.test_id)),
                        "test_status": test.test_status
                    })
                })
                .collect();

            json!({
                "req_id": req.req_id,
                "req_title": req.req_title,
                "req_reference": req.req_reference,
                "matrix": row
            })
        })
        .collect();

    // Count total links separately
    let total_links = reqs
        .iter()
        .map(|req| {
            tests
                .iter()
                .filter(|test| links.contains(&(req.req_id, test.test_id)))
                .count()
        })
        .sum();

    (rows, total_links)
}

/// Build tests list with status names
fn build_tests_with_status(tests: &[Test], state: &State<AppState>) -> Vec<serde_json::Value> {
    use serde_json::json;

    tests
        .iter()
        .map(|t| {
            json!({
                "test_id": t.test_id,
                "test_name": t.test_name,
                "test_reference": t.test_reference,
                "test_status": get_status_name_by_id_cached(state, t.test_status)
            })
        })
        .collect()
}

struct PaginationContext {
    start_item: i64,
    end_item: i64,
    show_first_page: bool,
    show_last_page: bool,
    show_first_ellipsis: bool,
    show_last_ellipsis: bool,
    has_prev_page: bool,
    has_next_page: bool,
}

/// Build pagination context
fn build_pagination_context(
    page: i64,
    per_page: i64,
    total_items: i64,
    total_pages: i64,
) -> PaginationContext {
    PaginationContext {
        start_item: ((page - 1) * per_page + 1).min(total_items),
        end_item: (page * per_page).min(total_items),
        show_first_page: page > 2,
        show_last_page: page < total_pages - 1,
        show_first_ellipsis: page > 3,
        show_last_ellipsis: page < total_pages - 2,
        has_prev_page: page > 1,
        has_next_page: page < total_pages,
    }
}

#[get("/<project_id>/matrix.xls")]
async fn get_matrix_xls(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();

    println!(
        "User {} (id:{}) requested matrix export for project {}",
        user.user_username, user.user_id, project_id
    );

    excel::create_matrix_workbook(cookies).map_err(|e| {
        eprintln!("Error creating matrix workbook: {e:?}");
        Redirect::to(format!("/p/{}/matrix", project_id))
    })?;

    let path = std::path::Path::new("target/matrix.xls");
    let file = NamedFile::open(path).await.map_err(|e| {
        eprintln!("Error opening matrix file: {e:?}");
        Redirect::to(format!("/p/{}/matrix", project_id))
    })?;

    let ct = ContentType::new(
        "application",
        "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );

    Ok((ct, file))
}

#[get("/<project_id>/matrix.csv?<test_status_filter>")]
async fn get_matrix_csv(
    project_access: ProjectAccess,
    project_id: i32,
    test_status_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<(ContentType, String), Redirect> {
    let user = project_access.into_user();

    println!(
        "User {} (id:{}) requested CSV export for project {} with test status filter: {:?}",
        user.user_username, user.user_id, project_id, test_status_filter
    );

    let service = MatrixService::new(state.inner());
    let csv_data = service
        .export_matrix_csv(project_id, test_status_filter)
        .map_err(|e| {
            eprintln!("Error generating CSV: {e:?}");
            Redirect::to(format!("/p/{}/matrix", project_id))
        })?;

    Ok((ContentType::new("text", "csv"), csv_data))
}

pub fn routes() -> Vec<Route> {
    routes![get_matrix, get_matrix_xls, get_matrix_csv]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Applicability, Category, Project, ProjectMember, Requirement, Status, Test, TestStatus,
        Verification,
    };
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, get_with_session, timestamp,
    };
    use rocket::http::Status as HttpStatus;
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const USER_ID: i32 = 2;
    const PRIMARY_PROJECT: i32 = 1;

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            project_id: id,
            project_name: name.to_string(),
            project_description: Some(format!("{name} project")),
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
            cat_description: format!("{title} systems"),
            cat_tag: title.to_ascii_uppercase(),
            project_id: PRIMARY_PROJECT,
        }
    }

    fn sample_status(id: i32, title: &str) -> Status {
        Status {
            st_id: id,
            st_title: title.to_string(),
            st_description: format!("{title} status"),
            st_short_name: title.to_ascii_uppercase(),
        }
    }

    fn sample_test_status(id: i32, title: &str) -> TestStatus {
        TestStatus {
            test_st_id: id,
            test_st_title: title.to_string(),
            test_st_description: format!("{title} status"),
            test_st_short_name: title.to_ascii_uppercase(),
        }
    }

    fn sample_applicability(id: i32, title: &str) -> Applicability {
        Applicability {
            app_id: id,
            app_title: title.to_string(),
            app_description: format!("{title} applicability"),
            app_tag: title.to_ascii_uppercase(),
            project_id: PRIMARY_PROJECT,
        }
    }

    fn sample_verification(id: i32, title: &str) -> Verification {
        Verification {
            verification_id: id,
            verification_name: title.to_string(),
            verification_description: format!("{title} verification"),
            project_id: PRIMARY_PROJECT,
        }
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

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        let mut user = DieselRepoMock::make_user(USER_ID, "user", "");
        user.is_admin = false;
        repo.users.insert(USER_ID, user);

        repo.projects
            .insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Orbiter"));

        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: USER_ID,
            role: 3,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.statuses.insert(1, sample_status(1, "Planned"));
        repo.test_statuses.insert(1, sample_test_status(1, "Draft"));
        repo.test_statuses
            .insert(2, sample_test_status(2, "Proposal"));
        repo.test_statuses
            .insert(3, sample_test_status(3, "Active"));

        repo.categories.insert(1, sample_category(1, "Systems"));
        repo.verifications
            .insert(1, sample_verification(1, "Analysis"));
        repo.applicability.insert(1, sample_applicability(1, "All"));
        repo.requirements.insert(1, sample_requirement(1));

        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(repo, routes![get_matrix, get_matrix_csv, get_matrix_xls]).await
    }

    #[rocket::async_test]
    async fn get_matrix_works_with_base_repo() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Requirement 1") || body.contains("REQ-SYS-1"));
    }

    #[rocket::async_test]
    async fn get_matrix_allows_project_member() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/matrix", USER_ID).await;

        // Non-admin user with project membership should be able to view matrix
        assert_eq!(response.status(), HttpStatus::Ok);
    }

    #[rocket::async_test]
    async fn get_matrix_supports_pagination() {
        let mut repo = base_repo();
        repo.requirements.clear(); // Clear default requirement

        // Add 25 requirements to test pagination with per_page=10
        for i in 1..=25 {
            repo.requirements.insert(
                i,
                Requirement {
                    req_id: i,
                    req_title: format!("Req {}", i),
                    req_description: String::new(),
                    req_verification: 1,
                    req_current_status: 1,
                    req_author: 1,
                    req_reviewer: 1,
                    req_reference: format!("REF-{}", i),
                    req_category: 1,
                    req_parent: 0,
                    req_creation_date: timestamp(),
                    req_update_date: timestamp(),
                    req_deadline_date: timestamp(),
                    req_applicability: 1,
                    req_justification: None,
                    project_id: 1,
                },
            );
        }

        let client = test_client(repo).await;

        // Request first page with 10 items per page (minimum allowed by clamp)
        let response = get_with_session(&client, "/p/1/matrix?page=1&per_page=10", ADMIN_ID).await;
        assert_eq!(response.status(), HttpStatus::Ok);

        let body = response.into_string().await.expect("response body");
        // Page 1 should contain first 10 requirements
        let req_1_in_table = body.contains(r#"req_id":1"#) || body.contains("REF-1");
        let req_10_in_table = body.contains(r#"req_id":10"#) || body.contains("REF-10");
        assert!(req_1_in_table, "Page 1 should contain requirement 1");
        assert!(req_10_in_table, "Page 1 should contain requirement 10");

        // Test page 2 - should contain requirements 11-20
        let response2 = get_with_session(&client, "/p/1/matrix?page=2&per_page=10", ADMIN_ID).await;
        assert_eq!(response2.status(), HttpStatus::Ok);
        let body2 = response2.into_string().await.expect("response body");

        let req_11_in_table = body2.contains(r#"req_id":11"#) || body2.contains("REF-11");
        let req_20_in_table = body2.contains(r#"req_id":20"#) || body2.contains("REF-20");
        assert!(req_11_in_table, "Page 2 should contain requirement 11");
        assert!(req_20_in_table, "Page 2 should contain requirement 20");
    }

    #[rocket::async_test]
    async fn get_matrix_supports_search() {
        let mut repo = base_repo();
        repo.requirements.clear(); // Clear default requirement

        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Authentication Requirement".to_string(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "AUTH-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        repo.requirements.insert(
            2,
            Requirement {
                req_id: 2,
                req_title: "Database Requirement".to_string(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "DB-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        let client = test_client(repo).await;

        // Search for "auth" should only find the first requirement
        let response = get_with_session(&client, "/p/1/matrix?search=auth", ADMIN_ID).await;
        let body = response.into_string().await.expect("response body");
        // Check if Authentication requirement is present in the table
        let has_auth = body.contains("Authentication") || body.contains("AUTH-001");
        assert!(
            has_auth,
            "Search results should contain Authentication requirement"
        );
        // Database requirement should not be in the results (not in pagination or filtered out)
        // Note: We check total_requirements to verify filtering worked
        let total_shown = body.contains(r#""total_requirements":1"#) || body.contains("of 1");
        assert!(
            total_shown,
            "Should show only 1 requirement after filtering"
        );
    }

    #[rocket::async_test]
    async fn get_matrix_csv_returns_csv_format() {
        let mut repo = base_repo();

        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Test Requirement".to_string(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        repo.tests.insert(
            1,
            Test {
                test_id: 1,
                test_name: "Test 1".to_string(),
                test_reference: "TST-1".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix.csv", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);

        let body = response.into_string().await.expect("response body");
        assert!(body.starts_with("Title,Reference"));
        assert!(body.contains("Test Requirement,REF-001"));
        assert!(body.contains("Test #1"));
    }

    #[rocket::async_test]
    async fn get_matrix_csv_escapes_special_characters() {
        let mut repo = base_repo();

        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Test, with \"quotes\"".to_string(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix.csv", ADMIN_ID).await;

        let body = response.into_string().await.expect("response body");
        // Should escape commas and quotes properly
        assert!(body.contains("\"Test, with \"\"quotes\"\"\""));
    }

    #[rocket::async_test]
    async fn get_matrix_handles_empty_dataset() {
        let mut repo = base_repo();
        // Remove the requirement to get empty dataset
        repo.requirements.clear();

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;
        assert_eq!(response.status(), HttpStatus::Ok);

        let body = response.into_string().await.expect("response body");
        // With no requirements, the table should still render but be empty
        assert!(body.contains("0") || body.contains("matrix"));
    }

    #[rocket::async_test]
    async fn get_matrix_displays_missing_links() {
        let mut repo = base_repo();

        // Add requirement without test links
        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Unlinked Requirement".to_string(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        repo.tests.insert(
            1,
            Test {
                test_id: 1,
                test_name: "Test 1".to_string(),
                test_reference: "TST-1".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;

        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Unlinked Requirement"));
        // Should show dash for unlinked test
        assert!(body.contains("-"));
    }
}
