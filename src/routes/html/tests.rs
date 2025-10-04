use super::helpers::*;
use super::prelude::*;

#[get("/<project_id>/tests?<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_tests(
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
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);

    // Fetch and process tests
    let tests = repo.get_tests_by_project(project_id).unwrap_or_default();

    let tests = decorate_tests(filter_tests(
        tests,
        status_filter,
        verification_filter,
        category_filter,
    ));
    ctx["tests"] = json!(tests);

    // Common data lookups
    ctx["statuses"] = json!(repo.get_status_all().unwrap_or_default());
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
pub fn show_test_id(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();

    let test = match get_test_by_id_cached_safe(state, test_id) {
        Ok(t) => t,
        Err(details) => {
            let ctx = json!({
                "title": "Test Not Found",
                "message": "The test you're looking for could not be found.",
                "details": details,
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    let decorated = decorate_tests(vec![test]);
    let test = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();

    let ctx = json!({
        "project_id": project_id,
        "test": test,
        "linked_requirements": linked_requirements,
        "user": user
    });

    Ok(Template::render("test_by_id", ctx))
}

#[get("/<project_id>/tests/new")]
pub fn new_test(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let repo = state.repo_read();

    let ctx = json!({
        "categories": repo.get_categories_by_project(project_id).unwrap_or_default(),
        "status": repo.get_status_all().unwrap_or_default(),
        "parents": repo.get_tests_by_project(project_id).unwrap_or_default(),
        "users": repo.get_users_all().unwrap_or_default(),
        "requirements": repo.get_requirements_by_project(project_id).unwrap_or_default(),
        "user": user
    });

    Ok(Template::render("new_test", ctx))
}

#[post("/<project_id>/tests/new", data = "<new_test>")]
pub fn post_test(
    project_access: ProjectAccess,
    project_id: i32,
    new_test: Form<NewTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!("/p", new_test(project_id)))
    })?;
    let my_new_test = NewTest {
        test_id: None,
        test_name: new_test.test_name.clone(),
        test_description: new_test.test_description.clone(),
        test_source: new_test.test_source.clone(),
        test_status: new_test.test_status,
        test_reference: format!("TEST-{}", chrono::Utc::now().timestamp()),
        test_parent: new_test.test_parent,
        project_id: new_test.project_id,
    };
    let test_id = state.repo_write().insert_test(&my_new_test).map_err(|e| {
        eprintln!("Error inserting new test: {:?}", e);
        Redirect::to(uri!(
            "/p",
            show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
        ))
    })?;

    let test = state
        .repo_read()
        .get_test_by_id(test_id)
        .expect("Error reading table Tests");

    // Log the test creation
    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::created(connection, &log_ctx, test_id, &test);

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
pub fn get_edit_test(
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

    let decorated = decorate_tests(vec![test]);
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
pub fn post_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    edit_test_form: Form<EditTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let to_list = || {
        Redirect::to(uri!(
            "/p",
            show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
        ))
    };
    let to_edit = || Redirect::to(uri!("/p", get_edit_test(project_id, test_id)));

    let mut conn = get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {e}");
        to_edit()
    })?;

    let repo_r = state.repo_read();
    let old_test = repo_r
        .get_test_by_id(test_id)
        .expect("Error reading table Tests");

    // Own the form to avoid cloning strings
    let f = edit_test_form.into_inner();

    let new_test = NewTest {
        test_id: Some(f.test_id),
        test_name: f.test_name,
        test_description: f.test_description,
        test_source: f.test_source,
        test_status: f.test_status,
        test_reference: old_test.test_reference.clone(),
        test_parent: f.test_parent,
        project_id: f.project_id,
    };

    state.repo_write().edit_test(&new_test).map_err(|e| {
        eprintln!("Error editing test: {e:?}");
        to_list()
    })?;

    let log_ctx = LogCtx::new(user.user_id);
    let updated = state
        .repo_read()
        .get_test_by_id(test_id)
        .expect("Error reading table Tests after update");
    let _ = Logger::updated(conn.as_mut(), &log_ctx, &old_test, &updated);

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
pub fn delete_test_route(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    use rocket::http::Status;

    let user = project_access.into_user();

    let mut conn = get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {e}");
        Status::InternalServerError
    })?;

    let test = get_test_by_id_cached_safe(state, test_id).map_err(|_| Status::NotFound)?;

    // allow only Draft(1) or Proposal(2) unless admin
    if test.test_status > 2 && !user.is_admin {
        return Err(Status::Forbidden);
    }

    let deleted = state
        .repo_write()
        .delete_test(test_id)
        .map_err(|e| match e {
            crate::repository::errors::RepoError::NotFound => Status::NotFound,
            _ => Status::InternalServerError,
        })?;

    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);

    Ok(Redirect::to(uri!(
        "/p",
        show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)
    )))
}

#[get("/<project_id>/matrix?<sort_by>&<sort_order>&<test_status_filter>")]
pub fn get_matrix(
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
pub async fn get_matrix_xls(
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
pub async fn get_requirements_xls(
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
pub async fn get_tests_xls(
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
