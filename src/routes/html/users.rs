use super::helpers::*;
use super::prelude::*;

#[get("/users")]
pub fn show_users(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let repo = state.repo_read();
    let projects = repo.get_projects_all().unwrap_or_default();

    let mut selected_project_id = get_selected_project_id(cookies);
    if selected_project_id.is_none() {
        if let Some(first_project) = projects.first() {
            cookies.add(Cookie::new(
                "selected_project_id",
                first_project.project_id.to_string(),
            ));
            selected_project_id = Some(first_project.project_id);
        }
    }

    let users = if let Some(project_id) = selected_project_id {
        match repo.get_members_by_project(project_id) {
            Ok(members) => {
                let member_ids: HashSet<i32> = members.into_iter().map(|m| m.user_id).collect();

                match repo.get_users_all() {
                    Ok(all_users) => all_users
                        .into_iter()
                        .filter(|u| member_ids.contains(&u.user_id))
                        .collect::<Vec<User>>(),
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let ctx = json!({
        "users": users,
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    });

    Ok(Template::render("users", ctx))
}

#[get("/users/<user_id>")]
pub fn show_user_id(
    session_user: SessionUser,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = session_user.into_inner();
    let user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");
    let ctx = json!({
        "user": current_user,
        "user_name": user.user_name,
        "user_username": user.user_username,
        "user_email": user.user_email,
        "user_id": user.user_id,
        "user_creation_date": user.user_creation_date,
        "user_last_login": user.user_last_login,
        "is_admin": user.is_admin
    });

    Ok(Template::render("user_by_id", ctx))
}

#[get("/edit_user/<user_id>")]
pub fn edit_user(
    session_user: SessionUser,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = session_user.into_inner();
    let user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");
    #[cfg(debug_assertions)]
    println!("USer: {:?}", user);
    let ctx = json!({
        "users": user,
        "user": current_user
    });
    #[cfg(debug_assertions)]
    println!("edit user: {:?}", ctx);
    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/edit_user/<user_id>", data = "<user_form>")]
pub fn post_edit_user(
    session_user: SessionUser,
    user_id: i32,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let current_user = session_user.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(edit_user(user_id)))
    })?;

    // Get the old values before updating
    let old_user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");

    // Create an UpdateUser with the user_id
    let mut user_data = user_form.into_inner();
    user_data.user_id = Some(user_id);

    // Update the user in the database
    match state.repo_write().update_user_without_password(&user_data) {
        Ok(_) => {
            // Log the user update
            let log_ctx = LogCtx::new(current_user.user_id);
            let _ = Logger::updated(
                connection,
                &log_ctx,
                &old_user,
                &state
                    .repo_read()
                    .get_user_by_id(user_id)
                    .expect("Error reading table Users after update"),
            );
            Ok(Redirect::to(uri!(show_user_id(user_id))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(edit_user(user_id))))
        }
    }
}

#[get("/new_user")]
pub fn new_user(session_user: SessionUser, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user
    });
    Ok(Template::render("new_user", ctx))
}

#[post("/new_user", data = "<new_user>")]
pub fn post_user(
    session_user: SessionUser,
    new_user: Form<NewUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_user))
    })?;

    // Hash the password before inserting
    let mut user_with_hashed_password = new_user.into_inner();
    match hash_password(&user_with_hashed_password.user_password) {
        Ok(hashed_password) => {
            user_with_hashed_password.user_password = hashed_password;
            let user_id = state
                .repo_write()
                .insert_user(&user_with_hashed_password)
                .map_err(|e| {
                    eprintln!("Error inserting new user: {:?}", e);
                    Redirect::to(uri!(new_user))
                })?;

            // Log the user creation
            let user = state
                .repo_read()
                .get_user_by_id(user_id)
                .expect("Error reading table Users");
            let log_ctx = LogCtx::new(session_user.into_inner().user_id);
            let _ = Logger::created(connection, &log_ctx, user_id, &user);

            Ok(Redirect::to(uri!(show_user_id(user_id))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_user)))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![show_users, show_user_id, edit_user, post_edit_user, new_user, post_user]
}
