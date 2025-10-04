use super::helpers::*;
use super::prelude::*;

#[delete("/<project_id>/tests/delete/<test_id>")]
pub fn delete_test_route(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        rocket::http::Status::InternalServerError
    })?;

    // Get the test details before deleting
    let test = match get_test_by_id_cached_safe(state, test_id) {
        Ok(t) => t,
        Err(_) => {
            // Test not found
            return Err(rocket::http::Status::NotFound);
        }
    };

    // Check if user can delete this test
    // Only allow deletion if status is Draft (1) or Proposal (2), or if user is admin
    if test.test_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    match state.repo_write().delete_test(test_id) {
        Ok(test) => {
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::deleted(connection.as_mut(), &log_ctx, &test);

            // Redirect to tests list page
            Ok(Redirect::to(uri!("/p", show_tests(
                project_id,
                None::<i32>,
                None::<i32>,
                None::<i32>
            ))))
        }
        Err(crate::repository::errors::RepoError::NotFound) => Err(rocket::http::Status::NotFound),
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting test: {:?}", _e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

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
    let user = project_access.into_user();
    let mut ctx = build_context_with_projects(state, user, cookies);

    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);

    let tests = if let Some(project_id) = selected_project_id {
        state.repo_read().get_tests_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_tests_by_project(first_project.project_id)
        } else {
            state.repo_read().get_tests_all()
        }
    };

    let tests_data = tests.unwrap_or_default();
    // Apply filters
    let filtered_tests = filter_tests(
        tests_data,
        status_filter,
        verification_filter,
        category_filter,
    );
    let tests_decorate = decorate_tests(filtered_tests);
    ctx["tests"] = json!(tests_decorate);

    // Add filter data to context for the template
    let statuses = state.repo_read().get_status_all().unwrap_or_default();
    let verifications = if let Some(project_id) = selected_project_id {
        state.repo_read().get_verification_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_verification_by_project(first_project.project_id)
        } else {
            state.repo_read().get_verification_all()
        }
    };

    // Get categories filtered by selected project
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };

    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications.unwrap_or_default());
    ctx["categories"] = json!(categories.unwrap_or_default());
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);

    Ok(Template::render("tests", ctx))
}

#[get("/<project_id>/tests/<test_id_param>")]
pub fn show_test_id(
    project_access: ProjectAccess,
    project_id: i32,
    test_id_param: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    // Use the safe function that returns a Result
    match get_test_by_id_cached_safe(state, test_id_param) {
        Ok(test) => {
            let test_decorate = decorate_tests(vec![test]);

            // Get linked requirements for this test
            let linked_requirements =
                get_requirements_for_test_cached(state, test_id_param).unwrap_or_default();
            let linked_requirements_json = json!(linked_requirements);

            let decorated_test = &test_decorate[0];
            let ctx = json!({
                "test_id": decorated_test.test_id,
                "test_name": decorated_test.test_name,
                "test_description": decorated_test.test_description,
                "test_source": decorated_test.test_source,
                "test_status": decorated_test.test_status,
                "test_parent_id": decorated_test.test_parent_id,
                "test_parent_title": decorated_test.test_parent_title,
                "linked_requirements": linked_requirements_json,
                "user": user
            });

            Ok(Template::render("test_by_id", ctx))
        }
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Test Not Found",
                "message": "The test you're looking for could not be found.",
                "details": error_msg,
                "user": user
            });

            Ok(Template::render("error", ctx))
        }
    }
}

#[get("/<project_id>/new_test")]
pub fn new_test(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent tests filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        state.repo_read().get_tests_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_tests_by_project(first_project.project_id)
        } else {
            state.repo_read().get_tests_all()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = state.repo_read().get_users_all().unwrap_or_default();
    let users_json = json!(users);

    // Get requirements filtered by project
    let requirements = if let Some(project_id) = selected_project_id {
        state.repo_read().get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_requirements_by_project(first_project.project_id)
        } else {
            state.repo_read().get_requirements_all()
        }
    };
    let requirements_json = json!(requirements.unwrap_or_default());

    let ctx = json!({
        "categories": categories_json,
        "status": status_json,
        "parents": parents_json,
        "users": users_json,
        "requirements": requirements_json,
        "user": user
    });

    Ok(Template::render("new_test", ctx))
}

#[get("/<project_id>/edit_test/<test_id>")]
pub fn get_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let test = state
        .repo_read()
        .get_test_by_id(test_id)
        .expect("Error reading table Tests");
    let test_decorate = decorate_tests(vec![test]);
    let test_decorate_json = json!(test_decorate[0]);

    let status = state.repo_read().get_test_status_all().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        state.repo_read().get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_categories_by_project(first_project.project_id)
        } else {
            state.repo_read().get_categories_all()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent tests filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        state.repo_read().get_tests_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_tests_by_project(first_project.project_id)
        } else {
            state.repo_read().get_tests_all()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = state.repo_read().get_users_all().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        state.repo_read().get_verification_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_verification_by_project(first_project.project_id)
        } else {
            state.repo_read().get_verification_all()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get linked requirements for this test
    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let linked_requirements_json = json!(linked_requirements);

    // Create a simple array of linked requirement IDs for template checking
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.req_id).collect();
    let linked_req_ids_json = json!(linked_req_ids);

    // Get all requirements for the multi-select (filtered by project)
    let all_requirements = if let Some(project_id) = selected_project_id {
        state.repo_read().get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            state
                .repo_read()
                .get_requirements_by_project(first_project.project_id)
        } else {
            state.repo_read().get_requirements_all()
        }
    };
    let all_requirements_json = json!(all_requirements.unwrap_or_default());

    let ctx = json!({
        "tests": test_decorate_json,
        "test_status_id": test_decorate[0].test_status_id,
        "categories": categories_json,
        "status": status_json,
        "parent": parents_json,
        "users": users_json,
        "verification": verification_json,
        "linked_requirements": linked_requirements_json,
        "linked_req_ids": linked_req_ids_json,
        "requirements": all_requirements_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);
    Ok(Template::render("edit_test_by_id", ctx))
}

#[post("/<project_id>/edit_test/<test_id>", data = "<edit_test_form>")]
pub fn post_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    edit_test_form: Form<EditTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!("/p", get_edit_test(project_id, test_id)))
    })?;

    // Get the old values before updating
    let old_test = state
        .repo_read()
        .get_test_by_id(test_id)
        .expect("Error reading table Tests");

    // First, update the test details
    let new_test = NewTest {
        test_id: Some(edit_test_form.test_id),
        test_name: edit_test_form.test_name.clone(),
        test_description: edit_test_form.test_description.clone(),
        test_source: edit_test_form.test_source.clone(),
        test_status: edit_test_form.test_status,
        test_reference: old_test.test_reference.clone(),
        test_parent: edit_test_form.test_parent,
        project_id: edit_test_form.project_id,
    };

    state.repo_write().edit_test(&new_test).map_err(|e| {
        eprintln!("Error editing test: {:?}", e);
        Redirect::to(uri!("/p", show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)))
    })?;

    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::updated(
        connection,
        &log_ctx,
        &old_test,
        &state
            .repo_read()
            .get_test_by_id(test_id)
            .expect("Error reading table Tests after update"),
    );

    // Then, update the requirement links
    state
        .repo_write()
        .update_test_requirement_links(edit_test_form.test_id, &edit_test_form.linked_requirements)
        .map_err(|e| {
            eprintln!("Error updating test requirement links: {:?}", e);
            Redirect::to(uri!("/p", show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)))
        })?;

    Ok(Redirect::to(uri!("/p", show_test_id(project_id, edit_test_form.test_id))))
}

#[post("/<project_id>/new_test", data = "<new_test>")]
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
        Redirect::to(uri!("/p", show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)))
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
                Redirect::to(uri!("/p", show_tests(project_id, None::<i32>, None::<i32>, None::<i32>)))
            })?;
    }

    Ok(Redirect::to(uri!("/p", show_test_id(project_id, test_id))))
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
    let user = project_access.into_user();
    use crate::schema::matrix::dsl::*;
    use crate::schema::requirements::dsl::*;
    use crate::schema::tests::dsl::*;

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
        }
    };

    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);

    let mut all_reqs = if let Some(selected_pid) = selected_project_id {
        requirements
            .filter(crate::schema::requirements::project_id.eq(selected_pid))
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying requirements from the database".to_string()
            })
            .expect("Error getting matrix table")
    } else {
        requirements
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying page views from the database".to_string()
            })
            .expect("Error getting matrix table")
    };

    let mut all_tests = if let Some(selected_pid) = selected_project_id {
        tests
            .filter(crate::schema::tests::project_id.eq(selected_pid))
            .load::<Test>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying tests from the database".to_string()
            })
            .expect("Error getting tests")
    } else {
        tests
            .load::<Test>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying tests from the database".to_string()
            })
            .expect("Error getting tests")
    };

    // Always sort tests by test_id (number)
    all_tests.sort_by(|a, b| a.test_id.cmp(&b.test_id));

    // Filter tests by status if filter is provided
    if let Some(status_filter) = test_status_filter {
        all_tests.retain(|test| test.test_status == status_filter);
    }

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "req_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    // Check if sorting by test column
    if sort_by.starts_with("test_") {
        // Extract test ID from sort_by (e.g., "test_1" -> test_id = 1)
        if let Ok(target_test_id) = sort_by.trim_start_matches("test_").parse::<i32>() {
            // Sort requirements based on their link status to the specified test
            if sort_order == "desc" {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    b_has_link.cmp(&a_has_link)
                });
            } else {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    a_has_link.cmp(&b_has_link)
                });
            }
        }
    } else {
        // Sort requirements by requirement fields
        match sort_by.as_str() {
            "req_id" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_id.cmp(&a.req_id));
                } else {
                    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
                }
            }
            "req_title" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_title.cmp(&a.req_title));
                } else {
                    all_reqs.sort_by(|a, b| a.req_title.cmp(&b.req_title));
                }
            }
            "req_reference" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_reference.cmp(&a.req_reference));
                } else {
                    all_reqs.sort_by(|a, b| a.req_reference.cmp(&b.req_reference));
                }
            }
            _ => {
                // Default sort by req_id ascending
                all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
            }
        }
    }

    let total_tests = all_tests.len() as i32;
    let total_requirements = all_reqs.len() as i32;

    // Create matrix data structure
    let mut total_links = 0;
    let mut requirements_with_matrix = Vec::new();

    for req in &all_reqs {
        let mut req_matrix = Vec::new();

        for test in &all_tests {
            let test_present: i64 = matrix
                .filter(matrix_req_id.eq(req.req_id))
                .filter(matrix_test_id.eq(test.test_id))
                .count()
                .get_result(connection.as_mut())
                .unwrap();

            if test_present > 0 {
                req_matrix.push(json!({
                    "linked": true,
                    "test_status": test.test_status
                }));
                total_links += 1;
            } else {
                req_matrix.push(json!({
                    "linked": false,
                    "test_status": null
                }));
            }
        }

        requirements_with_matrix.push(json!({
            "req_id": req.req_id,
            "req_title": req.req_title,
            "req_reference": req.req_reference,
            "matrix": req_matrix
        }));
    }

    // Prepare tests with status names
    let mut tests_with_status = Vec::new();
    for test in all_tests {
        let test_status_name = get_status_name_by_id_cached(state, test.test_status);
        tests_with_status.push(json!({
            "test_id": test.test_id,
            "test_name": test.test_name,
            "test_status": test_status_name
        }));
    }

    // Get all statuses for the filter dropdown
    let all_statuses = state.repo_read().get_status_all().unwrap_or_default();
    let statuses_json = json!(all_statuses);

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["requirements"] = json!(requirements_with_matrix);
    ctx["tests"] = json!(tests_with_status);
    ctx["total_tests"] = json!(total_tests);
    ctx["total_requirements"] = json!(total_requirements);
    ctx["total_links"] = json!(total_links);
    ctx["current_sort_by"] = json!(sort_by);
    ctx["current_sort_order"] = json!(sort_order);
    ctx["test_status_filter"] = json!(test_status_filter);
    ctx["statuses"] = json!(statuses_json);

    Ok(Template::render("matrix", ctx))
}

#[get("/<project_id>/matrix.xls")]
pub async fn get_matrix_xls(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = project_access.into_user();

    match excel::create_matrix_workbook(cookies) {
        Ok(_) => {
            let path_to_file = path::Path::new("target/matrix.xls");
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
                Err(error) => {
                    eprintln!("Error opening matrix file: {:?}", error);
                    Err(Redirect::to("/matrix"))
                }
            }
        }
        Err(e) => {
            eprintln!("Error creating matrix workbook: {:?}", e);
            Err(Redirect::to("/matrix"))
        }
    }
}

#[get("/<project_id>/requirements.xls")]
pub async fn get_requirements_xls(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = project_access.into_user();
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
    let _user = project_access.into_user();
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
