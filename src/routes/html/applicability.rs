use super::helpers::*;
use super::prelude::*;
use rocket::serde::Serialize;
use rocket::http::Status;

#[derive(Serialize)]
struct ApplicabilityCtx<'a> {
    user: &'a User,
    selected_project_id: i32,
    applicability: Option<Vec<Applicability>>,
}

#[get("/<project_id>/applicability")]
pub fn show_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {

    let apps = state
        .repo_read()
        .get_applicability_by_project(project_id)
        .unwrap_or_default();

    let ctx = ApplicabilityCtx {
        user: &project_access.into_user(),
        selected_project_id: project_id,
        applicability: Some(apps),
    };

    Ok(Template::render("applicability", &ctx))
}

#[get("/<project_id>/new_applicability")]
pub fn new_applicability(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<Template, Redirect> {
    let ctx = ApplicabilityCtx {
        user: &project_access.into_user(),
        selected_project_id: project_id,
        applicability: None,
    };

    Ok(Template::render("new_applicability", ctx))
}

#[post("/<project_id>/new_applicability", data = "<form>")]
pub fn post_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let new_url  = uri!("/p", new_applicability(project_id = project_id));
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let new_applicability = NewApplicability {
        project_id,
        ..form.into_inner()
    };

    let applicability_id = match state.repo_write().insert_new_applicability(&new_applicability) {
        Ok(id) => id,
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error inserting applicability: {:?}", err);
            return Ok(Redirect::to(new_url));
        }
    };

    let log_ctx = LogCtx::new(user.user_id);
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(new_url.clone())
    })?;
    let _ = Logger::created(connection, &log_ctx, applicability_id, &new_applicability);

    Ok(Redirect::to(show_url))
}


#[get("/<project_id>/edit_applicability/<app_id>")]
pub fn get_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let applicability = get_applicability_by_id_cached(state, app_id);

    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    let ctx = json!({
        "applicability": applicability,
        "user": user,
        "selected_project_id": project_id
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/<project_id>/edit_applicability/<app_id>", data = "<form>")]
pub fn post_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let edit_url = uri!(
        "/p",
        get_edit_applicability(project_id = project_id, app_id = app_id)
    );
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let old = get_applicability_by_id_cached(state, app_id);
    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = old.project_id)
        )));
    }

    let new = NewApplicability {
        project_id,
        ..form.into_inner()
    };

    if let Err(err) = state.repo_write().edit_applicability(&new) {
        #[cfg(debug_assertions)]
        eprintln!("Error updating applicability {app_id}: {err:?}");
        return Ok(Redirect::to(edit_url));
    }

    let updated = state
        .repo_read()
        .get_applicability_by_id(app_id)
        .expect("Error reading table Applicability after update");

    let log_ctx = LogCtx::new(user.user_id);
    let conn = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(edit_url.clone())
    })?;
    let _ = Logger::updated(conn, &log_ctx, &old, &updated);

    Ok(Redirect::to(show_url))
}


#[delete("/<project_id>/delete_applicability/<app_id>")]
pub fn delete_applicability_route(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Status, Redirect> {
    let user = project_access.into_user();
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let applicability = get_applicability_by_id_cached(state, app_id);
    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    let mut conn = get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(show_url.clone())
    })?;

    let deleted = match state.repo_write().delete_applicability(app_id) {
        Ok(app) => app,
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error deleting applicability {app_id}: {err:?}");
            return Ok(Status::InternalServerError);
        }
    };

    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);

    Ok(Status::Ok)
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
