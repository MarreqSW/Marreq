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
    let user_id = project_access.into_user().user_id;

    let new_url  = uri!("/p", new_category(project_id));
    let show_url = uri!("/p", show_categories(project_id));

    let mut category = new_category.into_inner();
    category.project_id = project_id;

    let category_id = state
        .repo_write()
        .insert_new_category(&category)
        .map_err(|e| {
            #[cfg(debug_assertions)]
            eprintln!("insert_new_category error: {:?}", e);
            Redirect::to(new_url.clone())
        })?;

    if let Ok(mut conn) = get_db_connection(state) {
        if let Ok(full) = state.repo_read().get_category_by_id(category_id) {
            let log_ctx = LogCtx::new(user_id);
            let _ = Logger::created(&mut conn, &log_ctx, category_id, &full);
        }
    }

    Ok(Redirect::to(show_url))
}


#[get("/<project_id>/categories/edit/<cat_id>")]
pub fn get_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let category = state
        .repo_read()
        .get_category_by_id(cat_id)
        .map_err(|_| Redirect::to(uri!("/p", show_categories(project_id))))?;

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let projects = get_accessible_projects(state, &user);

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
    let user_id = project_access.into_user().user_id;

    let edit_url = uri!("/p", get_edit_category(project_id, cat_id));
    let show_url = uri!("/p", show_categories(project_id));

    let old = state
        .repo_read()
        .get_category_by_id(cat_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;

    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = old.project_id)
        )));
    }

    let mut edited = category.into_inner();
    edited.cat_id = Some(cat_id);
    edited.project_id = project_id;

    state
        .repo_write()
        .edit_category(&edited)
        .map_err(|e| {
            #[cfg(debug_assertions)]
            eprintln!("edit_category error: {:?}", e);
            Redirect::to(edit_url.clone())
        })?;

    if let Ok(mut conn) = get_db_connection(state) {
        if let Ok(new_row) = state.repo_read().get_category_by_id(cat_id) {
            let log_ctx = LogCtx::new(user_id);
            let _ = Logger::updated(&mut conn, &log_ctx, &old, &new_row);
        }
    }

    Ok(Redirect::to(show_url))
}


#[delete("/<project_id>/categories/delete/<cat_id>")]
pub fn delete_category_route(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user_id = project_access.into_user().user_id;

    let category = match state.repo_read().get_category_by_id(cat_id) {
        Ok(c) => c,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let deleted = match state.repo_write().delete_category(cat_id) {
        Ok(c) => c,
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_category error: {:?}", e);
            return Ok(rocket::http::Status::InternalServerError);
        }
    };

    if let Ok(mut conn) = get_db_connection(state) {
        let log_ctx = LogCtx::new(user_id);
        let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);
    }

    Ok(rocket::http::Status::Ok)
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
