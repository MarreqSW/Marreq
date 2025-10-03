use super::helpers::*;
use super::prelude::*;

#[get("/<project_id>/categories")]
pub fn show_categories(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let categories = state
        .repo_read()
        .get_categories_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "categories": categories,
    });

    Ok(Template::render("categories", ctx))
}

#[get("/<project_id>/categories/new")]
pub fn new_category(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id
    });
    Ok(Template::render("new_category", ctx))
}

#[post("/<project_id>/categories/new", data = "<new_category>")]
pub fn post_category(
    project_access: ProjectAccess,
    project_id: i32,
    new_category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let new_url = uri!("/p", new_category(project_id = project_id));
    let show_url = uri!("/p", show_categories(project_id = project_id));

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(new_url.clone())
    })?;

    let mut category_data = new_category.into_inner();
    category_data.project_id = project_id;

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

            Ok(Redirect::to(show_url))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(new_url))
        }
    }
}

#[get("/<project_id>/categories/edit/<cat_id>")]
pub fn get_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let category = match state.repo_read().get_category_by_id(cat_id) {
        Ok(category) => category,
        Err(_) => {
            return Err(Redirect::to(uri!(
                "/p",
                show_categories(project_id = project_id)
            )))
        }
    };

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let ctx = json!({
        "categories": category,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id
    });
    Ok(Template::render("edit_category", ctx))
}

#[post("/<project_id>/categories/edit/<cat_id>", data = "<category>")]
pub fn post_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let edit_url = uri!(
        "/p",
        get_edit_category(project_id = project_id, cat_id = cat_id)
    );
    let show_url = uri!("/p", show_categories(project_id = project_id));

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(edit_url.clone())
    })?;

    // Get the old values before updating
    let old_category = match state.repo_read().get_category_by_id(cat_id) {
        Ok(category) => category,
        Err(_) => {
            return Err(Redirect::to(show_url));
        }
    };

    if old_category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = old_category.project_id)
        )));
    }

    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(cat_id);
    category_with_id.project_id = project_id;

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

            Ok(Redirect::to(show_url))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(edit_url))
        }
    }
}

#[delete("/<project_id>/categories/delete/<cat_id>")]
pub fn delete_category_route(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = project_access.into_user();
    let show_url = uri!("/p", show_categories(project_id = project_id));

    let category = match state.repo_read().get_category_by_id(cat_id) {
        Ok(category) => category,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(show_url));
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
