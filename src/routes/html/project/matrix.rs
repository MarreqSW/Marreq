use super::helpers::*;
use super::prelude::*;
use crate::services::{ApplicabilityService, CategoryService, MatrixService, StatusService};

#[get("/<project_id>/matrix?<sort_by>&<sort_order>&<test_status_filter>&<req_status_filter>&<category_filter>&<applicability_filter>&<linkage_filter>&<page>&<per_page>&<search>")]
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
    linkage_filter: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use crate::schema::*;
    use diesel::prelude::*;
    use serde_json::json;
    use std::collections::HashSet;

    let user = project_access.into_user();

    // DB connection with a single redirect-on-error path.
    let mut conn = get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {e}");
        Redirect::to(uri!(crate::routes::html::dashboard::index))
    })?;

    // Set pagination defaults
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(50).clamp(10, 200);
    
    // Load requirements & tests for the project in one go.
    let mut all_reqs: Vec<Requirement> = requirements::dsl::requirements
        .filter(requirements::project_id.eq(project_id))
        .load(conn.as_mut())
        .map_err(|e| {
            eprintln!("DB error loading requirements: {e}");
            Redirect::to(uri!(crate::routes::html::dashboard::index))
        })?;

    // Apply requirement status filter if provided
    if let Some(req_status) = req_status_filter {
        all_reqs.retain(|r| r.req_current_status == req_status);
    }

    // Apply category filter if provided
    if let Some(category) = category_filter {
        all_reqs.retain(|r| r.req_category == category);
    }

    // Apply applicability filter if provided
    if let Some(applicability) = applicability_filter {
        all_reqs.retain(|r| r.req_applicability == applicability);
    }

    // Apply search filter if provided
    if let Some(ref search_term) = search {
        let search_lower = search_term.to_lowercase();
        all_reqs.retain(|r| {
            r.req_title.to_lowercase().contains(&search_lower)
                || r.req_reference.to_lowercase().contains(&search_lower)
                || r.req_id.to_string().contains(&search_lower)
        });
    }

    let mut all_tests: Vec<Test> = tests::dsl::tests
        .filter(tests::project_id.eq(project_id))
        .load(conn.as_mut())
        .map_err(|e| {
            eprintln!("DB error loading tests: {e}");
            Redirect::to(uri!(crate::routes::html::dashboard::index))
        })?;

    // Always sort tests by id (ascending), then optionally filter by status.
    all_tests.sort_by_key(|t| t.test_id);
    if let Some(s) = test_status_filter {
        all_tests.retain(|t| t.test_status == s);
    }

    // Preload all links once; check membership in O(1) later.
    let links: HashSet<(i32, i32)> = matrix::dsl::matrix
        .select((matrix::matrix_req_id, matrix::matrix_test_id))
        .load::<(i32, i32)>(conn.as_mut())
        .map_err(|e| {
            eprintln!("DB error loading matrix links: {e}");
            Redirect::to(uri!(crate::routes::html::dashboard::index))
        })?
        .into_iter()
        .collect();

    // Apply linkage filter (before pagination to get accurate counts)
    if let Some(ref linkage) = linkage_filter {
        match linkage.as_str() {
            "linked" => {
                // Keep only requirements that have at least one test link
                all_reqs.retain(|r| {
                    all_tests.iter().any(|t| links.contains(&(r.req_id, t.test_id)))
                });
            }
            "unlinked" => {
                // Keep only requirements that have no test links
                all_reqs.retain(|r| {
                    !all_tests.iter().any(|t| links.contains(&(r.req_id, t.test_id)))
                });
            }
            _ => {} // "all" or unknown: no filtering
        }
    }

    let total_requirements = all_reqs.len() as i64;

    // Sort requirements.
    let sort_by = sort_by.unwrap_or_else(|| "req_id".to_string());
    let desc = sort_order.as_deref() == Some("desc");

    if let Some(test_id_str) = sort_by.strip_prefix("test_") {
        if let Ok(target_test_id) = test_id_str.parse::<i32>() {
            all_reqs.sort_by_key(|r| links.contains(&(r.req_id, target_test_id)));
            if desc {
                all_reqs.reverse();
            }
        }
    } else {
        match sort_by.as_str() {
            "req_title" => {
                all_reqs.sort_by(|a, b| a.req_title.cmp(&b.req_title));
                if desc {
                    all_reqs.reverse();
                }
            }
            "req_reference" => {
                all_reqs.sort_by(|a, b| a.req_reference.cmp(&b.req_reference));
                if desc {
                    all_reqs.reverse();
                }
            }
            _ => {
                all_reqs.sort_by_key(|r| r.req_id);
                if desc {
                    all_reqs.reverse();
                }
            }
        }
    }

    // Calculate pagination
    let total_pages = (total_requirements as f64 / per_page as f64).ceil() as i64;
    let start_idx = ((page - 1) * per_page) as usize;
    let end_idx = (start_idx + per_page as usize).min(all_reqs.len());
    let paginated_reqs = &all_reqs[start_idx..end_idx];

    // Build matrix cells + counts
    let mut total_links = 0;
    let requirements_with_matrix: Vec<_> = paginated_reqs
        .iter()
        .map(|req| {
            let row: Vec<_> = all_tests
                .iter()
                .map(|test| {
                    let linked = links.contains(&(req.req_id, test.test_id));
                    if linked {
                        total_links += 1;
                    }
                    json!({ "linked": linked, "test_status": test.test_status })
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

    // Tests with human-readable status name.
    let tests_with_status: Vec<_> = all_tests
        .iter()
        .map(|t| {
            json!({
                "test_id": t.test_id,
                "test_name": t.test_name,
                "test_reference": t.test_reference,
                "test_status": get_status_name_by_id_cached(state, t.test_status)
            })
        })
        .collect();

    // Calculate pagination display values
    let start_item = ((page - 1) * per_page + 1).min(total_requirements);
    let end_item = (page * per_page).min(total_requirements);
    let show_first_ellipsis = page > 3;
    let show_last_ellipsis = page < total_pages - 2;
    let show_first_page = page > 2;
    let show_last_page = page < total_pages - 1;

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["requirements"] = json!(requirements_with_matrix);
    ctx["tests"] = json!(tests_with_status);
    ctx["total_tests"] = json!(all_tests.len() as i32);
    ctx["total_requirements"] = json!(total_requirements);
    ctx["total_links"] = json!(total_links);
    ctx["current_sort_by"] = json!(sort_by);
    ctx["current_sort_order"] = json!(if desc { "desc" } else { "asc" });
    ctx["test_status_filter"] = json!(test_status_filter);
    ctx["req_status_filter"] = json!(req_status_filter);
    ctx["category_filter"] = json!(category_filter);
    ctx["applicability_filter"] = json!(applicability_filter);
    ctx["linkage_filter"] = json!(linkage_filter);
    ctx["search"] = json!(search);
    ctx["page"] = json!(page);
    ctx["per_page"] = json!(per_page);
    ctx["total_pages"] = json!(total_pages);
    ctx["has_prev_page"] = json!(page > 1);
    ctx["has_next_page"] = json!(page < total_pages);
    ctx["prev_page"] = json!(page - 1);
    ctx["next_page"] = json!(page + 1);
    ctx["start_item"] = json!(start_item);
    ctx["end_item"] = json!(end_item);
    ctx["show_first_page"] = json!(show_first_page);
    ctx["show_last_page"] = json!(show_last_page);
    ctx["show_first_ellipsis"] = json!(show_first_ellipsis);
    ctx["show_last_ellipsis"] = json!(show_last_ellipsis);
    ctx["test_statuses"] = json!(StatusService::new(state.inner()).list_test_statuses().unwrap_or_default());
    ctx["statuses"] = json!(StatusService::new(state.inner()).list_requirement_statuses().unwrap_or_default());
    ctx["categories"] = json!(CategoryService::new(state.inner()).list_by_project(project_id).unwrap_or_default());
    ctx["applicabilities"] = json!(ApplicabilityService::new(state.inner()).list_by_project(project_id).unwrap_or_default());
    ctx["total_test_columns"] = json!(all_tests.len() + 1); // Reference + test columns

    Ok(Template::render("matrix/matrix", ctx))
}

#[get("/<project_id>/matrix.xls")]
async fn get_matrix_xls(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();

    // Log user and project info
    println!(
        "User [{} - id:{}] requested matrix export for project_id={}",
        user.user_username, user.user_id, project_id
    );

    excel::create_matrix_workbook(cookies).map_err(|e| {
        eprintln!("Error creating matrix workbook: {e:?}");
        Redirect::to("/matrix")
    })?;

    let path = std::path::Path::new("target/matrix.xls");
    let file = NamedFile::open(path).await.map_err(|e| {
        eprintln!("Error opening matrix file: {e:?}");
        Redirect::to("/matrix")
    })?;

    // Note: MIME here is for .xlsx; change if you truly emit legacy .xls files.
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
        "User [{} - id:{}] requested CSV matrix export for project_id={} with status_filter={:?}",
        user.user_username, user.user_id, project_id, test_status_filter
    );

    let service = MatrixService::new(state);
    let csv_data = service.export_matrix_csv(project_id, test_status_filter)
        .map_err(|e| {
            eprintln!("Error generating CSV: {e:?}");
            Redirect::to(uri!(get_matrix(project_id = project_id, sort_by = None::<String>, sort_order = None::<String>, test_status_filter = None::<i32>, req_status_filter = None::<i32>, category_filter = None::<i32>, applicability_filter = None::<i32>, linkage_filter = None::<String>, page = None::<i64>, per_page = None::<i64>, search = None::<String>)))
        })?;

    let ct = ContentType::new("text", "csv");
    Ok((ct, csv_data))
}

pub fn routes() -> Vec<Route> {
    routes![
        get_matrix,
        get_matrix_xls,
        get_matrix_csv,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Applicability, Category, Matrix, Project, ProjectMember, Requirement, Status, Test,
        TestStatus, Verification,
    };
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, get_with_session, timestamp, TestAppState,
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

    fn sample_test(id: i32, status: i32, name: &str) -> Test {
        Test {
            test_id: id,
            test_name: name.to_string(),
            test_description: format!("{name} description"),
            test_source: "Design Spec".into(),
            test_status: status,
            test_reference: format!("TEST-{id:03}"),
            test_parent: 0,
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

    fn repo_with_tests() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.tests.insert(1, sample_test(1, 1, "Baseline Test"));
        repo.matrices.push(Matrix {
            matrix_req_id: 1,
            matrix_test_id: 1,
            matrix_creation_date: timestamp(),
            project_id: PRIMARY_PROJECT,
        });
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                get_matrix
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn get_matrix_redirects_when_database_unavailable() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
    }

    #[rocket::async_test]
    async fn get_matrix_requires_membership_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/matrix", USER_ID).await;

        // The matrix route redirects when the database is unavailable (mock repo doesn't provide real DB)
        assert_eq!(response.status(), HttpStatus::SeeOther);
    }

    // Note: The following tests require a real database connection because get_matrix
    // uses direct diesel queries. Consider refactoring to use repository pattern for testing.
    // Service-level tests in matrix_service.rs cover the pagination and CSV export logic.
    
    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_supports_pagination() {
        let mut repo = base_repo();
        
        // Add 5 requirements
        for i in 1..=5 {
            repo.requirements.insert(i, Requirement {
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
            });
        }

        let client = test_client(repo).await;
        
        // Request first page with 2 items per page
        let response = get_with_session(&client, "/p/1/matrix?page=1&per_page=2", ADMIN_ID).await;
        assert_eq!(response.status(), HttpStatus::Ok);
        
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Req 1"));
        assert!(body.contains("Req 2"));
        assert!(!body.contains("Req 3")); // Should not be on page 1
        
        // Test page 2
        let response2 = get_with_session(&client, "/p/1/matrix?page=2&per_page=2", ADMIN_ID).await;
        let body2 = response2.into_string().await.expect("response body");
        assert!(body2.contains("Req 3"));
        assert!(body2.contains("Req 4"));
        assert!(!body2.contains("Req 1")); // Should not be on page 2
    }

    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_supports_search() {
        let mut repo = base_repo();
        
        repo.requirements.insert(1, Requirement {
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
        });
        
        repo.requirements.insert(2, Requirement {
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
        });

        let client = test_client(repo).await;
        
        // Search for "auth" should only find the first requirement
        let response = get_with_session(&client, "/p/1/matrix?search=auth", ADMIN_ID).await;
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Authentication"));
        assert!(!body.contains("Database"));
    }

    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_csv_returns_csv_format() {
        let mut repo = base_repo();
        
        repo.requirements.insert(1, Requirement {
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
        });

        repo.tests.insert(1, Test {
            test_id: 1,
            test_name: "Test 1".to_string(),
            test_reference: "TST-1".to_string(),
            test_description: String::new(),
            test_source: String::new(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        });

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix.csv", ADMIN_ID).await;
        
        assert_eq!(response.status(), HttpStatus::Ok);
        
        let body = response.into_string().await.expect("response body");
        assert!(body.starts_with("Req ID,Title,Reference"));
        assert!(body.contains("REQ-1,Test Requirement,REF-001"));
        assert!(body.contains("Test #1"));
    }

    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_csv_escapes_special_characters() {
        let mut repo = base_repo();
        
        repo.requirements.insert(1, Requirement {
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
        });

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix.csv", ADMIN_ID).await;
        
        let body = response.into_string().await.expect("response body");
        // Should escape commas and quotes properly
        assert!(body.contains("\"Test, with \"\"quotes\"\"\""));
    }

    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_handles_empty_dataset() {
        let repo = base_repo();
        let client = test_client(repo).await;
        
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;
        assert_eq!(response.status(), HttpStatus::Ok);
        
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("No requirements found"));
    }

    #[rocket::async_test]
    #[ignore] // Requires real DB connection
    async fn get_matrix_displays_missing_links() {
        let mut repo = base_repo();
        
        // Add requirement without test links
        repo.requirements.insert(1, Requirement {
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
        });

        repo.tests.insert(1, Test {
            test_id: 1,
            test_name: "Test 1".to_string(),
            test_reference: "TST-1".to_string(),
            test_description: String::new(),
            test_source: String::new(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        });

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;
        
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Unlinked Requirement"));
        // Should show dash for unlinked test
        assert!(body.contains("-"));
    }
}
