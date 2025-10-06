use super::helpers::*;
use super::prelude::*;
use crate::repository::errors::RepoError;
use crate::services::project_service::ProjectService;
use rocket::serde::json::Value;

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

#[get("/new_project")]
pub fn new_project(admin: AdminOnly, state: &State<AppState>) -> Template {
    let user = admin.into_inner();
    render_new_project_form(state, &user, default_new_project_form(), None)
}

#[post("/new_project", data = "<new_project>")]
pub fn post_project(
    admin: AdminOnly,
    new_project: Form<NewProject>,
    state: &State<AppState>,
) -> Result<Redirect, Template> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let submitted = new_project.into_inner();
    let form_state = snapshot_new_project_form(&submitted);

    match project_service.create(&user, submitted) {
        Ok(_) => Ok(Redirect::to(uri!(show_projects))),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to create project: {err:?}");
            let message = match err {
                RepoError::BadInput(reason) => reason,
                _ => "Failed to create project. Please try again.".to_string(),
            };

            Err(render_new_project_form(
                state,
                &user,
                form_state,
                Some(message),
            ))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![show_projects, new_project, post_project]
}

fn render_new_project_form(
    state: &State<AppState>,
    user: &User,
    form: Value,
    error: Option<String>,
) -> Template {
    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "users": users,
        "user": user,
        "form": form,
        "error": error,
    });

    Template::render("new_project", ctx)
}

fn default_new_project_form() -> Value {
    json!({
        "project_name": "",
        "project_description": "",
        "project_status": "active",
        "project_owner_id": null,
    })
}

fn snapshot_new_project_form(project: &NewProject) -> Value {
    json!({
        "project_name": project.project_name.clone(),
        "project_description": project
            .project_description
            .clone()
            .unwrap_or_default(),
        "project_status": project.project_status.clone(),
        "project_owner_id": project.project_owner_id,
    })
}
