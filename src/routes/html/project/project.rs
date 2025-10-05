use super::helpers::*;
use super::prelude::*;
use crate::services::project_service::ProjectService;
use chrono::Utc;

#[get("/<project_id>")]
pub fn show_project_id(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let project_service = ProjectService::new(state.inner());
    if !user.is_admin {
        let memberships = state
            .repo_read()
            .get_projects_for_user(user.user_id)
            .unwrap_or_default();

        let has_access = memberships
            .iter()
            .any(|membership| membership.project_id == project_id);

        if !has_access {
            return Err(Redirect::to(uri!("/projects")));
        }
    }
    let project = project_service
        .get_by_id(project_id)
        .unwrap_or_else(|_| fallback_project());

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

#[get("/<project_id>/edit")]
pub fn get_edit_project(admin: AdminOnly, project_id: i32, state: &State<AppState>) -> Template {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let project = project_service
        .get_by_id(project_id)
        .unwrap_or_else(|_| fallback_project());
    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "project": project,
        "users": users,
        "user": user
    });
    Template::render("edit_project", ctx)
}

#[post("/<project_id>/edit", data = "<project>")]
pub fn post_edit_project(
    admin: AdminOnly,
    project_id: i32,
    project: Form<UpdateProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());

    match project_service.update(&user, project_id, project.into_inner()) {
        Ok(_) => Ok(Redirect::to(uri!("/projects"))),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to update project {project_id}: {err:?}");
            Ok(Redirect::to(uri!(get_edit_project(project_id))))
        }
    }
}

#[delete("/<project_id>/delete")]
pub fn delete_project_route(
    admin: AdminOnly,
    project_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());

    match project_service.delete(&user, project_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to delete project {project_id}: {err:?}");
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_project_id,
        get_edit_project,
        post_edit_project,
        delete_project_route
    ]
}

fn fallback_project() -> Project {
    Project {
        project_id: 0,
        project_name: "Unknown Project".to_string(),
        project_description: Some("Unknown project".to_string()),
        project_creation_date: Some(Utc::now().naive_utc()),
        project_update_date: Some(Utc::now().naive_utc()),
        project_status: Some("Unknown".to_string()),
        project_owner_id: Some(0),
    }
}
