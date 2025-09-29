use super::helpers::*;
use super::prelude::*;

#[get("/projects")]
pub fn show_projects(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let (projects, selected_project_id) = get_user_projects_and_selection(state, &user, cookies);
    let decorated_projects = decorate_projects_for_listing(state, &user, &projects);

    let ctx = json!({
        "projects": decorated_projects,
        "user": user,
        "selected_project_id": selected_project_id
    });

    Ok(Template::render("projects", ctx))
}

#[get("/projects/<project_id>")]
pub fn show_project_id(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    if !user.is_admin {
        let memberships = state
            .repo_read()
            .get_projects_for_user(user.user_id)
            .unwrap_or_default();

        let has_access = memberships
            .iter()
            .any(|membership| membership.project_id == project_id);

        if !has_access {
            return Err(Redirect::to(uri!(show_projects)));
        }
    }
    let project = get_project_by_id_pooled_safe(state, project_id);

    let members = state
        .repo_read()
        .get_members_by_project(project_id)
        .unwrap_or_default();

    let user_map: HashMap<i32, User> = state
        .repo_read()
        .get_users_all()
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.user_id, u))
        .collect();

    let decorated_members: Vec<_> = members
        .into_iter()
        .map(|membership| {
            let role_label = describe_project_role(membership.role).to_string();
            if let Some(user) = user_map.get(&membership.user_id) {
                json!({
                    "user_id": user.user_id,
                    "user_name": user.user_name,
                    "user_username": user.user_username,
                    "user_email": user.user_email,
                    "role_label": role_label,
                    "role_id": membership.role,
                    "is_admin": user.is_admin
                })
            } else {
                json!({
                    "user_id": membership.user_id,
                    "user_name": format!("Unknown User #{}", membership.user_id),
                    "user_username": "unknown",
                    "user_email": "",
                    "role_label": role_label,
                    "role_id": membership.role,
                    "is_admin": false
                })
            }
        })
        .collect();

    let mut ctx = build_context_with_projects(state, user.clone(), cookies);
    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("project".to_string(), json!(project));
        ctx_obj.insert("members".to_string(), json!(decorated_members));
        ctx_obj.insert("user".to_string(), json!(user));
    }

    Ok(Template::render("project_detail", ctx))
}

#[get("/new_project")]
pub fn new_project(admin: AdminOnly, state: &State<AppState>) -> Template {
    let user = admin.into_inner();

    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "users": users,
        "user": user
    });
    Template::render("new_project", ctx)
}

#[post("/new_project", data = "<new_project>")]
pub fn post_project(
    admin: AdminOnly,
    new_project: Form<NewProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_project))
    })?;

    let project_data = new_project.into_inner();
    let result = state.repo_write().insert_new_project(&project_data);
    match result {
        Ok(project_id) => {
            let project = state
                .repo_read()
                .get_project_by_id(project_id)
                .expect("Error reading table Projects");
            // Log the project creation
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::created(connection, &log_ctx, project_id, &project);

            Ok(Redirect::to(uri!(show_projects)))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_project)))
        }
    }
}

#[get("/edit_project/<project_id>")]
pub fn get_edit_project(admin: AdminOnly, project_id: i32, state: &State<AppState>) -> Template {
    let user = admin.into_inner();

    let project = get_project_by_id_pooled_safe(state, project_id);
    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "project": project,
        "users": users,
        "user": user
    });
    Template::render("edit_project", ctx)
}

#[post("/edit_project/<project_id>", data = "<project>")]
pub fn post_edit_project(
    admin: AdminOnly,
    project_id: i32,
    project: Form<UpdateProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_project(project_id)))
    })?;

    // Get the old values before updating
    let old_project = get_project_by_id_cached(state, project_id);

    let result = state.repo_write().edit_project(project_id, &project);
    match result {
        Ok(_) => {
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::updated(
                connection,
                &log_ctx,
                &old_project,
                &state
                    .repo_read()
                    .get_project_by_id(project_id)
                    .expect("Error reading table Projects after update"),
            );

            Ok(Redirect::to(uri!(show_projects)))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_project(project_id))))
        }
    }
}

#[delete("/delete_project/<project_id>")]
pub fn delete_project_route(
    admin: AdminOnly,
    project_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = admin.into_inner();

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_projects)));
        }
    };

    let result = state.repo_write().delete_project(project_id);
    match result {
        Ok(project) => {
            // Log the project deletion
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::deleted(connection.as_mut(), &log_ctx, &project);

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
        show_projects,
        show_project_id,
        new_project,
        post_project,
        get_edit_project,
        post_edit_project,
        delete_project_route
    ]
}
