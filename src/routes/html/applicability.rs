use super::helpers::*;
use super::prelude::*;


fn has_access(state: &State<AppState>, user: &User, project_id: i32) -> bool {
    get_accessible_projects(state, &user)
        .iter()
        .any(|project| project.project_id == project_id)
}

#[get("/applicability")]
pub fn show_applicability(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let project_id = get_selected_project_id(cookies).expect("Project must exist!");

    if !has_access(state, &user, project_id) {
        //log_unauthorized_attempt();
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }

    let mut ctx: serde_json::Value = json!({
        "user": user,
        "selected_project_id": Some(project_id)
    });

    let applicability = state
        .repo_read()
        .get_applicability_by_project(project_id);

    match applicability {
        Ok(apps) => {
            ctx["applicability"] = json!(apps);
        }
        Err(_) => {
            ctx["applicability"] = json!([]);
        }
    };

    Ok(Template::render("applicability", ctx))
}

#[get("/new_applicability")]
pub fn new_applicability(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let project_id = get_selected_project_id(cookies).expect("Project must exist!");

    if !has_access(state, &user, project_id) {
        //log_unauthorized_attempt();
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }

    let ctx = json!({
        "user": user,
        "selected_project_id": project_id
    });

    Ok(Template::render("new_applicability", ctx))
}

#[post("/new_applicability", data = "<new_applicability>")]
pub fn post_applicability(
    session_user: SessionUser,
    new_applicability: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();

    if !has_access(state, &user, new_applicability.project_id) {
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_applicability))
    })?;

    match state.repo_write().insert_new_applicability(&new_applicability.into_inner()) {
        Ok(applicability_id) => {
            let applicability = state
                .repo_read()
                .get_applicability_by_id(applicability_id)
                .expect("Error reading table Applicability");

            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::created(connection, &log_ctx, applicability_id, &applicability);

            Ok(Redirect::to(uri!(show_applicability)))
        }
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error inserting applicability: {:?}", err);
            Ok(Redirect::to(uri!(new_applicability)))
        }
    }
}


#[get("/edit_applicability/<app_id>")]
pub fn get_edit_applicability(
    session_user: SessionUser,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let applicability = get_applicability_by_id_cached(state, app_id);
    let project_id = applicability.project_id;

    if !has_access(state, &user, project_id) {
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }

    let ctx = json!({
        "applicability": applicability,
        "user": user
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/edit_applicability/<app_id>", data = "<applicability>")]
pub fn post_edit_applicability(
    session_user: SessionUser,
    app_id: i32,
    applicability: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();
    let applicability_old = get_applicability_by_id_cached(state, app_id);
    let project_id = applicability.project_id;

    if !has_access(state, &user, project_id) {
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_applicability(app_id)))
    })?;

    let result = state
        .repo_write()
        .edit_applicability(&applicability);
    match result {
        Ok(_) => {
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::updated(
                connection,
                &log_ctx,
                &applicability_old,
                &state
                    .repo_read()
                    .get_applicability_by_id(app_id)
                    .expect("Error reading table Applicability after update"),
            );
            Ok(Redirect::to(uri!(show_applicability)))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_applicability(app_id))))
        }
    }
}

#[delete("/delete_applicability/<app_id>")]
pub fn delete_applicability_route(
    session_user: SessionUser,
    app_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = session_user.into_inner();
    let applicability = get_applicability_by_id_cached(state, app_id);
    let project_id = applicability.project_id;

    if !has_access(state, &user, project_id) {
        return Err(Redirect::to(uri!(crate::routes::html::dashboard::index)));
    }
    
    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_applicability)));
        }
    };

    let result = state.repo_write().delete_applicability(app_id);
    match result {
        Ok(applicability) => {
            // Log the applicability deletion
            let log_ctx = LogCtx::new(user.user_id);

            let _ = Logger::deleted(connection.as_mut(), &log_ctx, &applicability);

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
        show_applicability,
        new_applicability,
        post_applicability,
        get_edit_applicability,
        post_edit_applicability,
        delete_applicability_route
    ]
}
