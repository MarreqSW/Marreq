use super::helpers::*;
use super::prelude::*;

#[get("/users/<user_id>")]
pub fn show_user_id(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
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
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
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
    admin: AdminOnly,
    user_id: i32,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let current_user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(edit_user(user_id)))
    })?;

    let old_user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");

    let mut user_data = user_form.into_inner();
    user_data.user_id = Some(user_id);

    match state.repo_write().update_user_without_password(&user_data) {
        Ok(_) => {
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
pub fn new_user(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();
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
    admin: AdminOnly,
    new_user: Form<NewUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_user))
    })?;

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

            let user = state
                .repo_read()
                .get_user_by_id(user_id)
                .expect("Error reading table Users");
            let log_ctx = LogCtx::new(admin.into_inner().user_id);
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
    routes![
        show_user_id,
        edit_user,
        post_edit_user,
        new_user,
        post_user
    ]
}
