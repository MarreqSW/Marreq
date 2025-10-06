use super::helpers::*;
use super::prelude::*;
use crate::services::project_service::ProjectService;

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
    let project_service = ProjectService::new(state.inner());

    match project_service.create(&user, new_project.into_inner()) {
        Ok(_) => Ok(Redirect::to(uri!(show_projects))),
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to create project: {_err:?}");
            Ok(Redirect::to(uri!(new_project)))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![show_projects, new_project, post_project]
}
