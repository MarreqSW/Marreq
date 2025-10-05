use super::helpers::*;
use super::prelude::*;
use crate::services::TestService;

#[get("/<project_id>/tests?<status_filter>&<verification_filter>&<category_filter>")]
async fn show_tests(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);

    // Fetch and process tests
    let tests = service.list_by_project(project_id).unwrap_or_default();

    let tests = decorate_tests_cached(
        state,
        filter_tests(tests, status_filter, verification_filter, category_filter),
    );
    ctx["tests"] = json!(tests);

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

    Ok(Template::render("tests", ctx))
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

    let mut ctx_map = serde_json::Map::new();
    ctx_map.insert("project_id".into(), json!(project_id));
    ctx_map.insert("selected_project_id".into(), json!(project_id));
    ctx_map.insert("linked_requirements".into(), json!(linked_requirements));
    ctx_map.insert("user".into(), json!(user));

    if let Ok(serde_json::Value::Object(test_obj)) = serde_json::to_value(&test) {
        for (key, value) in test_obj {
            ctx_map.insert(key, value);
        }
    }

    Ok(Template::render(
        "test_by_id",
        serde_json::Value::Object(ctx_map),
    ))
}

#[get("/<project_id>/tests/new")]
async fn new_test(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
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

    Ok(Template::render("new_test", ctx))
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
            show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
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
                    show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
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

    Ok(Template::render("edit_test_by_id", ctx))
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
    let to_list = || {
        Redirect::to(uri!(
            "/p",
            show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
        ))
    };

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

    // allow only Draft(1) or Proposal(2) unless admin
    if test.test_status > 2 && !user.is_admin {
        return Err(Status::Forbidden);
    }

    service.delete(&user, test_id).map_err(|e| match e {
        crate::repository::errors::RepoError::NotFound => Status::NotFound,
        _ => Status::InternalServerError,
    })?;

    Ok(Redirect::to(uri!(
        "/p",
        show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
    )))
}

#[get("/<project_id>/matrix?<sort_by>&<sort_order>&<test_status_filter>")]
async fn get_matrix(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    test_status_filter: Option<i32>,
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

    // Load requirements & tests for the project in one go.
    let mut all_reqs: Vec<Requirement> = requirements::dsl::requirements
        .filter(requirements::project_id.eq(project_id))
        .load(conn.as_mut())
        .map_err(|e| {
            eprintln!("DB error loading requirements: {e}");
            Redirect::to(uri!(crate::routes::html::dashboard::index))
        })?;

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

    // Build matrix cells + counts.
    let mut total_links = 0;
    let requirements_with_matrix: Vec<_> = all_reqs
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
                "test_status": get_status_name_by_id_cached(state, t.test_status)
            })
        })
        .collect();

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["requirements"] = json!(requirements_with_matrix);
    ctx["tests"] = json!(tests_with_status);
    ctx["total_tests"] = json!(all_tests.len() as i32);
    ctx["total_requirements"] = json!(all_reqs.len() as i32);
    ctx["total_links"] = json!(total_links);
    ctx["current_sort_by"] = json!(sort_by);
    ctx["current_sort_order"] = json!(if desc { "desc" } else { "asc" });
    ctx["test_status_filter"] = json!(test_status_filter);
    ctx["statuses"] = json!(state.repo_read().get_status_all().unwrap_or_default());

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
            req_link: String::new(),
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
        assert!(body.contains("TEST-001"));
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
        assert_eq!(response.headers().get_one("Location"), Some("/p/1/tests"));

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
}
