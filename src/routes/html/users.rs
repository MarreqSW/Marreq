use super::helpers::*;
use super::prelude::*;
use rocket::serde::json::Value;

#[derive(FromForm)]
pub struct ProjectMemberForm {
    pub user_id: i32,
    pub role: i32,
}

fn is_project_owner(state: &State<AppState>, project_id: i32, user_id: i32) -> bool {
    if let Ok(members) = state.repo_read().get_members_by_project(project_id) {
        members
            .into_iter()
            .any(|member| member.user_id == user_id && member.role == 1)
    } else {
        false
    }
}

fn can_remove_member(
    can_manage_members: bool,
    owner_count: usize,
    member: &ProjectMember,
    current_user_id: i32,
) -> bool {
    if !can_manage_members {
        return false;
    }

    let is_owner = member.role == 1;
    let is_last_owner = is_owner && owner_count <= 1;
    if is_last_owner {
        return false;
    }

    if member.user_id == current_user_id && is_owner && owner_count <= 1 {
        return false;
    }

    true
}

#[get("/p/<project_id>/members")]
pub fn show_project_members(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let mut ctx = build_context_with_projects(state, user.clone(), cookies);

    let repo = state.repo_read();
    let project = match repo.get_project_by_id(project_id) {
        Ok(project) => project,
        Err(_) => return Err(Redirect::to(uri!(super::projects::show_projects))),
    };

    let memberships = repo.get_members_by_project(project_id).unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    drop(repo);

    let owner_count = memberships.iter().filter(|member| member.role == 1).count();
    let can_manage_members = is_project_owner(state, project_id, user.user_id);

    let user_lookup: HashMap<i32, &User> = users
        .iter()
        .map(|member| (member.user_id, member))
        .collect();

    let decorated_members: Vec<Value> = memberships
        .iter()
        .map(|membership| {
            let (name, username, email, is_admin) = user_lookup
                .get(&membership.user_id)
                .map(|member| {
                    (
                        member.user_name.clone(),
                        member.user_username.clone(),
                        member.user_email.clone(),
                        member.is_admin,
                    )
                })
                .unwrap_or_else(|| {
                    (
                        format!("Unknown User #{}", membership.user_id),
                        "unknown".to_string(),
                        String::new(),
                        false,
                    )
                });

            json!({
                "user_id": membership.user_id,
                "name": name,
                "username": username,
                "email": email,
                "role_id": membership.role,
                "role_label": describe_project_role(membership.role),
                "is_admin": is_admin,
                "can_remove": can_remove_member(
                    can_manage_members,
                    owner_count,
                    membership,
                    user.user_id,
                ),
            })
        })
        .collect();

    let member_count = decorated_members.len();

    let member_ids: HashSet<i32> = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect();

    let available_users: Vec<Value> = if can_manage_members {
        users
            .iter()
            .filter(|candidate| !member_ids.contains(&candidate.user_id))
            .map(|candidate| {
                json!({
                    "user_id": candidate.user_id,
                    "label": format!("{} (@{})", candidate.user_name, candidate.user_username),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let has_available_users = !available_users.is_empty();

    let role_options = vec![
        json!({ "id": 1, "label": describe_project_role(1) }),
        json!({ "id": 2, "label": describe_project_role(2) }),
        json!({ "id": 3, "label": describe_project_role(3) }),
        json!({ "id": 4, "label": describe_project_role(4) }),
    ];

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("project".to_string(), json!(project));
        ctx_obj.insert("members".to_string(), json!(decorated_members));
        ctx_obj.insert("can_manage_members".to_string(), json!(can_manage_members));
        ctx_obj.insert("available_users".to_string(), json!(available_users));
        ctx_obj.insert("role_options".to_string(), json!(role_options));
        ctx_obj.insert("project_id".to_string(), json!(project_id));
        ctx_obj.insert("current_user_id".to_string(), json!(user.user_id));
        ctx_obj.insert("owner_count".to_string(), json!(owner_count));
        ctx_obj.insert("member_count".to_string(), json!(member_count));
        ctx_obj.insert(
            "has_available_users".to_string(),
            json!(has_available_users),
        );
        ctx_obj.insert("selected_project_id".to_string(), json!(project_id));
    }

    Ok(Template::render("members", ctx))
}

#[post("/p/<project_id>/members", data = "<form>")]
pub fn add_project_member(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<ProjectMemberForm>,
    state: &State<AppState>,
) -> Redirect {
    let user = project_access.into_user();

    if !is_project_owner(state, project_id, user.user_id) {
        return Redirect::to(uri!(show_project_members(project_id = project_id)));
    }

    let payload = form.into_inner();
    let new_member = NewProjectMember {
        project_id,
        user_id: payload.user_id,
        role: payload.role,
    };

    if let Err(error) = state.repo_write().add_project_member(&new_member) {
        eprintln!("Error adding project member: {:?}", error);
    }

    Redirect::to(uri!(show_project_members(project_id = project_id)))
}

#[post("/p/<project_id>/members/<member_id>/remove")]
pub fn remove_project_member(
    project_access: ProjectAccess,
    project_id: i32,
    member_id: i32,
    state: &State<AppState>,
) -> Redirect {
    let user = project_access.into_user();

    if !is_project_owner(state, project_id, user.user_id) {
        return Redirect::to(uri!(show_project_members(project_id = project_id)));
    }

    let allow_removal = {
        let repo = state.repo_read();
        let members = repo.get_members_by_project(project_id).unwrap_or_default();
        let owner_count = members.iter().filter(|member| member.role == 1).count();

        !members
            .iter()
            .any(|member| member.user_id == member_id && member.role == 1 && owner_count <= 1)
    };

    if !allow_removal {
        return Redirect::to(uri!(show_project_members(project_id = project_id)));
    }

    if let Err(error) = state
        .repo_write()
        .remove_project_member(project_id, member_id)
    {
        eprintln!("Error removing project member: {:?}", error);
    }

    Redirect::to(uri!(show_project_members(project_id = project_id)))
}

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
        show_project_members,
        add_project_member,
        remove_project_member,
        show_user_id,
        edit_user,
        post_edit_user,
        new_user,
        post_user
    ]
}
