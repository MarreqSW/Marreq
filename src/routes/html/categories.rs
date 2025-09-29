use super::helpers::*;
use super::prelude::*;

#[get("/categories")]
pub fn show_categories(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let mut ctx = build_context_with_projects(state, user, cookies);

    // Get selected project ID
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

    match categories {
        Ok(cats) => {
            ctx["categories"] = json!(cats);
        }
        Err(_) => {
            ctx["categories"] = json!([]);
        }
    };

    Ok(Template::render("categories", ctx))
}

#[get("/new_category")]
pub fn new_category(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();

    // Get projects and selected project
    let projects = state.repo_read().get_projects_all().unwrap_or_default();
    let mut selected_project_id = get_selected_project_id(cookies);

    // If no project is selected and there are projects available, select the first one
    if selected_project_id.is_none() && !projects.is_empty() {
        selected_project_id = Some(projects[0].project_id);
        // Set the cookie for the selected project
        cookies.add(Cookie::new(
            "selected_project_id",
            projects[0].project_id.to_string(),
        ));
    }

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Ok(Template::render("new_category", ctx))
}

#[post("/new_category", data = "<new_category>")]
pub fn post_category(
    session_user: SessionUser,
    new_category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();

    // Check if project_id is provided
    if new_category.project_id == 0 {
        return Ok(Redirect::to(uri!(new_category)));
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_category))
    })?;

    let category_data = new_category.into_inner();
    let result = state.repo_write().insert_new_category(&category_data);
    match result {
        Ok(category_id) => {
            // Log the category creation
            let category = state
                .repo_read()
                .get_category_by_id(category_id)
                .expect("Error reading table Categories");
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::created(connection, &log_ctx, category_id, &category);

            Ok(Redirect::to(uri!(show_categories)))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_category)))
        }
    }
}

#[get("/edit_category/<cat_id>")]
pub fn get_edit_category(
    session_user: SessionUser,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let category = get_category_by_id_cached(state, cat_id);
    let ctx = json!({
        "categories": category,
        "user": user
    });
    Ok(Template::render("edit_category", ctx))
}

#[post("/edit_category/<cat_id>", data = "<category>")]
pub fn post_edit_category(
    session_user: SessionUser,
    cat_id: i32,
    category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_category(cat_id)))
    })?;

    // Get the old values before updating
    let old_category = get_category_by_id_cached(state, cat_id);

    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(cat_id);

    let result = state.repo_write().edit_category(&category_with_id);
    match result {
        Ok(_) => {
            // Log the category update
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::updated(
                connection,
                &log_ctx,
                &old_category,
                &state
                    .repo_read()
                    .get_category_by_id(cat_id)
                    .expect("Error reading table Categories after update"),
            );

            Ok(Redirect::to(uri!(show_categories)))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_category(cat_id))))
        }
    }
}

#[delete("/delete_category/<cat_id>")]
pub fn delete_category_route(
    session_user: SessionUser,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = session_user.into_inner();
    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_categories)));
        }
    };

    let result = state.repo_write().delete_category(cat_id);
    match result {
        Ok(category) => {
            // Log the category deletion
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::deleted(connection.as_mut(), &log_ctx, &category);

            Ok(rocket::http::Status::Ok)
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_categories,
        new_category,
        post_category,
        get_edit_category,
        post_edit_category,
        delete_category_route
    ]
}
