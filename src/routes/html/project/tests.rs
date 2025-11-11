use super::helpers::*;
use super::prelude::*;
use crate::helper_functions::decorators::decorate_requirements_with_repo;
use crate::services::{MatrixService, TestService};
use crate::status_enums::TestStatusEnum;

#[get("/<project_id>/tests?<status_filter>&<verification_filter>&<category_filter>&<search>")]
async fn show_tests(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    search: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let is_admin = user.is_admin;
    let service = TestService::new(state.inner());
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);

    // Get project info
    let project = repo.get_project_by_id(project_id).ok();
    if let Some(ref proj) = project {
        ctx["project"] = json!({
            "id": proj.project_id,
            "name": proj.project_name,
        });
    }

    // Fetch and process tests
    let all_tests = service.list_by_project(project_id).unwrap_or_default();

    // Calculate metrics before filtering
    // Using enum definitions for test statuses: Passed=1, Failed=2, Pending=3, InProgress=4
    let total = all_tests.len();
    let passed = all_tests
        .iter()
        .filter(|t| t.test_status == TestStatusEnum::Passed.id())
        .count();
    let failed = all_tests
        .iter()
        .filter(|t| t.test_status == TestStatusEnum::Failed.id())
        .count();
    let pending = all_tests
        .iter()
        .filter(|t| t.test_status == TestStatusEnum::Pending.id())
        .count();
    let in_progress = all_tests
        .iter()
        .filter(|t| t.test_status == TestStatusEnum::InProgress.id())
        .count();
    let pass_rate_percent = if total > 0 { (passed * 100) / total } else { 0 };

    // Apply filters
    let mut tests = filter_tests(
        all_tests,
        status_filter,
        verification_filter,
        category_filter,
    );

    // Apply search filter
    if let Some(ref query) = search {
        let query_lower = query.to_lowercase();
        tests.retain(|t| {
            t.test_name.to_lowercase().contains(&query_lower)
                || t.test_description.to_lowercase().contains(&query_lower)
                || t.test_reference.to_lowercase().contains(&query_lower)
        });
    }

    let tests = decorate_tests_cached(state, tests);
    ctx["tests"] = json!(tests);

    // Add metrics
    ctx["test_metrics"] = json!({
        "total": total,
        "passed": passed,
        "failed": failed,
        "pending": pending,
        "in_progress": in_progress,
        "pass_rate": {
            "percent": pass_rate_percent,
            "passed": passed
        }
    });

    // Common data lookups
    ctx["statuses"] = json!(repo.get_test_status_all().unwrap_or_default());
    ctx["verifications"] = json!(repo
        .get_verification_by_project(project_id)
        .unwrap_or_default());
    ctx["categories"] = json!(repo
        .get_categories_by_project(project_id)
        .unwrap_or_default());

    // Active filter values
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);
    ctx["search_query"] = json!(search.unwrap_or_default());

    // User info for admin checks
    ctx["is_admin"] = json!(is_admin);

    Ok(Template::render("tests/tests", ctx))
}

#[get("/<project_id>/tests/show/<test_id>")]
async fn show_test_id(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let test = match service.get_by_id(test_id) {
        Ok(t) => t,
        Err(details) => {
            let ctx = json!({
                "title": "Test Not Found",
                "message": "The test you're looking for could not be found.",
                "details": details.to_string(),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    let decorated = decorate_tests_cached(state, vec![test]);
    let test = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let repo = state.repo_read();
    let decorated_requirements = decorate_requirements_with_repo(&*repo, linked_requirements);

    let mut ctx_map = serde_json::Map::new();
    ctx_map.insert("project_id".into(), json!(project_id));
    ctx_map.insert("selected_project_id".into(), json!(project_id));
    ctx_map.insert("linked_requirements".into(), json!(decorated_requirements));
    ctx_map.insert("user".into(), json!(user));

    if let Ok(serde_json::Value::Object(test_obj)) = serde_json::to_value(&test) {
        for (key, value) in test_obj {
            ctx_map.insert(key, value);
        }
    }

    Ok(Template::render(
        "tests/test",
        serde_json::Value::Object(ctx_map),
    ))
}

#[get("/<project_id>/tests/new?<error>")]
async fn new_test(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["categories"] = json!(repo
        .get_categories_by_project(project_id)
        .unwrap_or_default());
    ctx["status"] = json!(repo.get_test_status_all().unwrap_or_default());
    ctx["parents"] = json!(repo.get_tests_by_project(project_id).unwrap_or_default());
    ctx["users"] = json!(repo.get_users_all().unwrap_or_default());
    ctx["requirements"] = json!(repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default());
    ctx["project_id"] = json!(project_id);
    ctx["selected_project_id"] = json!(project_id);
    ctx["error"] = json!(error);

    Ok(Template::render("tests/new_test", ctx))
}

#[post("/<project_id>/tests/new", data = "<new_test>")]
async fn post_test(
    project_access: ProjectAccess,
    project_id: i32,
    new_test: Form<NewTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let my_new_test = NewTest {
        test_id: None,
        test_name: new_test.test_name.clone(),
        test_description: new_test.test_description.clone(),
        test_source: new_test.test_source.clone(),
        test_status: new_test.test_status,
        test_reference: new_test.test_reference.clone(),
        test_parent: new_test.test_parent,
        project_id: project_id,
    };

    let test_id = service.create(&user, my_new_test).map_err(|e| {
        eprintln!("Error inserting new test: {:?}", e);
        Redirect::to(uri!(
            "/p",
            new_test(
                project_id = project_id,
                error = Some("Failed to create test".to_string())
            )
        ))
    })?;

    // Link requirements
    #[cfg(debug_assertions)]
    println!("NewTestForm requirements: {:#?}", new_test.test_req);
    for req in new_test.test_req.iter() {
        let matrix_item = NewMatrix {
            matrix_req_id: *req,
            matrix_test_id: test_id,
            project_id: new_test.project_id,
        };
        state
            .repo_write()
            .insert_new_matrix_item(&matrix_item)
            .map_err(|e| {
                eprintln!("Error inserting matrix item: {:?}", e);
                Redirect::to(uri!(
                    "/p",
                    new_test(
                        project_id = project_id,
                        error = Some("Failed to link requirements".to_string())
                    )
                ))
            })?;
    }

    Ok(Redirect::to(uri!("/p", show_test_id(project_id, test_id))))
}

#[get("/<project_id>/tests/edit/<test_id>")]
async fn get_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let repo = state.repo_read();

    let test = repo
        .get_test_by_id(test_id)
        .expect("Error reading table Tests");

    let decorated = decorate_tests_cached(state, vec![test]);
    let test0 = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.req_id).collect();

    let ctx = json!({
        "tests": test0,
        "test_status_id": test0.test_status_id,
        "categories": repo.get_categories_by_project(project_id).unwrap_or_default(),
        "status": repo.get_test_status_all().unwrap_or_default(),
        "parent": repo.get_tests_by_project(project_id).unwrap_or_default(),
        "users": repo.get_users_all().unwrap_or_default(),
        "verification": repo.get_verification_by_project(project_id).unwrap_or_default(),
        "linked_requirements": linked_requirements,
        "linked_req_ids": linked_req_ids,
        "requirements": repo.get_requirements_by_project(project_id).unwrap_or_default(),
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);

    Ok(Template::render("tests/edit_test", ctx))
}

#[post("/<project_id>/tests/edit/<test_id>", data = "<edit_test_form>")]
async fn post_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    edit_test_form: Form<EditTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = TestService::new(state.inner());
    let to_list = || Redirect::to(format!("/p/{}/tests", project_id));

    // Own the form to avoid cloning strings
    let f = edit_test_form.into_inner();

    let new_test = NewTest {
        test_id: Some(f.test_id),
        test_name: f.test_name,
        test_description: f.test_description,
        test_source: f.test_source,
        test_status: f.test_status,
        test_reference: f.test_reference,
        test_parent: f.test_parent,
        project_id: f.project_id,
    };

    service.update(&user, test_id, new_test).map_err(|e| {
        eprintln!("Error editing test: {e:?}");
        to_list()
    })?;

    state
        .repo_write()
        .update_test_requirement_links(f.test_id, &f.linked_requirements)
        .map_err(|e| {
            eprintln!("Error updating test requirement links: {e:?}");
            to_list()
        })?;

    Ok(Redirect::to(uri!(
        "/p",
        show_test_id(project_id, f.test_id)
    )))
}

#[delete("/<project_id>/tests/delete/<test_id>")]
async fn delete_test_route(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    use rocket::http::Status;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let test = service.get_by_id(test_id).map_err(|_| Status::NotFound)?;

    // Permission gate: only allow deletion of tests in Passed or Failed status, or if admin
    // Using enum to check if the test is in a deletable state
    let is_deletable = TestStatusEnum::from_id(test.test_status)
        .map(|status| matches!(status, TestStatusEnum::Passed | TestStatusEnum::Failed))
        .unwrap_or(false);

    if !is_deletable && !user.is_admin {
        return Err(Status::Forbidden);
    }

    service.delete(&user, test_id).map_err(|e| match e {
        crate::repository::errors::RepoError::NotFound => Status::NotFound,
        _ => Status::InternalServerError,
    })?;

    Ok(Redirect::to(format!("/p/{}/tests", project_id)))
}

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
    ctx["test_statuses"] = json!(state.repo_read().get_test_status_all().unwrap_or_default());
    ctx["req_statuses"] = json!(state.repo_read().get_status_all().unwrap_or_default());
    ctx["categories"] = json!(state.repo_read().get_categories_by_project(project_id).unwrap_or_default());
    ctx["applicabilities"] = json!(state.repo_read().get_applicability_all().unwrap_or_default());
    ctx["total_test_columns"] = json!(all_tests.len() + 1); // Reference + test columns

    Ok(Template::render("matrix", ctx))
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

#[get("/<project_id>/requirements.xls")]
async fn get_requirements_xls(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested requirements export for project_id={}",
        user.user_username, user.user_id, project_id
    );

    let _file = excel::create_requirements_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/requirements.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/<project_id>/tests.xls")]
async fn get_tests_xls(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested requirements export for project_id={}",
        user.user_username, user.user_id, project_id
    );
    let _file = excel::create_tests_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/tests.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        delete_test_route,
        show_tests,
        show_test_id,
        new_test,
        get_edit_test,
        post_edit_test,
        post_test,
        get_matrix,
        get_matrix_xls,
        get_matrix_csv,
        get_requirements_xls,
        get_tests_xls
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
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        timestamp, TestAppState,
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

    fn repo_with_active_test() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.tests
            .insert(1, sample_test(1, 3, "Qualification Test"));
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                show_tests,
                show_test_id,
                new_test,
                post_test,
                get_edit_test,
                post_edit_test,
                delete_test_route,
                get_matrix
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_tests_lists_known_items() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn show_test_id_displays_details() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
        assert!(body.contains("description"));
    }

    #[rocket::async_test]
    async fn show_test_id_returns_error_when_missing() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests/show/42", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Test Not Found"));
    }

    #[rocket::async_test]
    async fn new_test_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests/new", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("New Test"));
        assert!(body.contains("Create Test"));
    }

    #[rocket::async_test]
    async fn post_test_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/tests/new",
            concat!(
                "test_name=Thermal+Check&test_reference=TEST-002&test_description=Thermal+validation&",
                "test_source=Spec&test_status=1&test_parent=0&test_req=1&project_id=1"
            ),
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/tests/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.tests.get(&1).expect("inserted test");
        assert_eq!(test.test_name, "Thermal Check");
        assert_eq!(test.test_status, 1);

        let links: Vec<_> = inner
            .matrices
            .iter()
            .filter(|m| m.matrix_test_id == 1)
            .collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].matrix_req_id, 1);
    }

    #[rocket::async_test]
    async fn get_edit_test_renders_existing_data() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests/edit/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Edit Test"));
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn post_edit_test_updates_entry() {
        let client = test_client(repo_with_tests()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/tests/edit/1",
            concat!(
                "test_id=1&test_reference=TEST-001&test_name=Updated+Test&test_description=Updated+desc&",
                "test_source=Updated&test_status=2&test_parent=0&linked_requirements=1&project_id=1"
            ),
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/tests/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.tests.get(&1).expect("existing test");
        assert_eq!(test.test_name, "Updated Test");
        assert_eq!(test.test_status, 2);

        let links: Vec<_> = inner
            .matrices
            .iter()
            .filter(|m| m.matrix_test_id == 1)
            .collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].matrix_req_id, 1);
    }

    #[rocket::async_test]
    async fn delete_test_route_removes_draft() {
        let client = test_client(repo_with_tests()).await;
        let response = delete_with_session(&client, "/p/1/tests/delete/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        let location = response.headers().get_one("Location");
        assert!(location.is_some());
        assert!(location.unwrap().contains("/p/1/tests"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.inner_repo().tests.is_empty());
    }

    #[rocket::async_test]
    async fn delete_test_route_forbids_non_admin_when_status_high() {
        let client = test_client(repo_with_active_test()).await;
        let response = delete_with_session(&client, "/p/1/tests/delete/1", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Forbidden);
    }

    #[rocket::async_test]
    async fn get_matrix_redirects_when_database_unavailable() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/matrix", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
    }

    #[rocket::async_test]
    async fn show_tests_requires_membership_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
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
