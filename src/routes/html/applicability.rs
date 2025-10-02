use super::helpers::*;
use super::prelude::*;
use rocket::http::Cookie;

#[get("/<project_id>/applicability")]
pub fn show_applicability(
    project_access: ProjectAccess,
    cookies: &CookieJar<'_>,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let mut ctx: serde_json::Value = json!({
        "user": user,
        "selected_project_id": Some(project_id)
    });

    let applicability = state.repo_read().get_applicability_by_project(project_id);

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

#[get("/<project_id>/new_applicability")]
pub fn new_applicability(
    project_access: ProjectAccess,
    cookies: &CookieJar<'_>,
    project_id: i32,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let ctx = json!({
        "user": user,
        "selected_project_id": project_id
    });

    Ok(Template::render("new_applicability", ctx))
}

#[post("/<project_id>/new_applicability", data = "<new_applicability>")]
pub fn post_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    new_applicability: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let mut new_applicability = new_applicability.into_inner();

    if new_applicability.project_id != project_id {
        new_applicability.project_id = project_id;
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_applicability(project_id = project_id)))
    })?;

    match state
        .repo_write()
        .insert_new_applicability(&new_applicability)
    {
        Ok(applicability_id) => {
            let applicability = state
                .repo_read()
                .get_applicability_by_id(applicability_id)
                .expect("Error reading table Applicability");

            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::created(connection, &log_ctx, applicability_id, &applicability);

            Ok(Redirect::to(uri!(show_applicability(
                project_id = project_id
            ))))
        }
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error inserting applicability: {:?}", err);
            Ok(Redirect::to(uri!(new_applicability(
                project_id = project_id
            ))))
        }
    }
}

#[get("/<project_id>/edit_applicability/<app_id>")]
pub fn get_edit_applicability(
    project_access: ProjectAccess,
    cookies: &CookieJar<'_>,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let applicability = get_applicability_by_id_cached(state, app_id);

    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(show_applicability(
            project_id = applicability.project_id
        ))));
    }

    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let ctx = json!({
        "applicability": applicability,
        "user": user,
        "selected_project_id": project_id
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/<project_id>/edit_applicability/<app_id>", data = "<applicability>")]
pub fn post_edit_applicability(
    project_access: ProjectAccess,
    cookies: &CookieJar<'_>,
    project_id: i32,
    app_id: i32,
    applicability: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let applicability_old = get_applicability_by_id_cached(state, app_id);

    if applicability_old.project_id != project_id {
        return Err(Redirect::to(uri!(show_applicability(
            project_id = applicability_old.project_id
        ))));
    }

    let mut applicability = applicability.into_inner();
    if applicability.project_id != project_id {
        applicability.project_id = project_id;
    }

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_applicability(
            project_id = project_id,
            app_id = app_id
        )))
    })?;

    let result = state.repo_write().edit_applicability(&applicability);
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
            cookies.add(Cookie::new("selected_project_id", project_id.to_string()));
            Ok(Redirect::to(uri!(show_applicability(
                project_id = project_id
            ))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_applicability(
                project_id = project_id,
                app_id = app_id
            ))))
        }
    }
}

#[delete("/<project_id>/delete_applicability/<app_id>")]
pub fn delete_applicability_route(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = project_access.into_user();
    let applicability = get_applicability_by_id_cached(state, app_id);

    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(show_applicability(
            project_id = applicability.project_id
        ))));
    }

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_applicability(
                project_id = project_id
            ))));
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
