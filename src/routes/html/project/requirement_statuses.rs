// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! HTML routes for project-scoped requirement status management.
//! System statuses are shown but not editable or deletable.

use super::helpers::*;
use super::prelude::*;
use crate::services::StatusService;

#[get("/<namespace>/<project_id>/requirement_statuses")]
async fn show_requirement_statuses(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = get_accessible_projects(state, &user)
        .iter()
        .map(|project| project_to_template_value(state, project))
        .collect();
    let service = StatusService::new(state.inner());
    let statuses = service
        .list_requirement_statuses_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "requirement_statuses": statuses,
        "page_title": "Requirement statuses"
    });

    Ok(Template::render(
        "requirement_statuses/requirement_statuses",
        ctx,
    ))
}

#[get("/<namespace>/<project_id>/requirement_statuses/new")]
async fn new_requirement_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = get_accessible_projects(state, &user)
        .iter()
        .map(|project| project_to_template_value(state, project))
        .collect();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": "New requirement status"
    });
    Ok(Template::render(
        "requirement_statuses/new_requirement_status",
        ctx,
    ))
}

#[post("/<namespace>/<project_id>/requirement_statuses/new", data = "<form>")]
async fn post_requirement_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    form: Form<NewRequirementStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let new_url = format!("/{project_slug}/requirement_statuses/new");
    let show_url = format!("/{project_slug}/requirement_statuses");

    let mut payload = form.into_inner();
    payload.project_id = project_id;
    payload.tag_color = payload.tag_color.filter(|s| !s.is_empty());

    if let Err(_e) = service.create_requirement_status(payload) {
        #[cfg(debug_assertions)]
        eprintln!("create_requirement_status error: {:?}", _e);
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<namespace>/<project_id>/requirement_statuses/edit/<status_id>")]
async fn get_edit_requirement_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    status_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let status = service
        .get_requirement_status(status_id)
        .map_err(|_| Redirect::to(format!("/{project_slug}/requirement_statuses")))?;

    if status.project_id != project_id {
        let status_project_slug = get_project_slug_by_id_pooled_safe(state, status.project_id);
        return Err(Redirect::to(format!(
            "/{status_project_slug}/requirement_statuses"
        )));
    }

    if status.is_system {
        return Err(Redirect::to(format!(
            "/{project_slug}/requirement_statuses"
        )));
    }

    let projects: Vec<_> = get_accessible_projects(state, &user)
        .iter()
        .map(|project| project_to_template_value(state, project))
        .collect();

    let ctx = json!({
        "requirement_status": status,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": format!("Edit {} - Requirement status", status.title)
    });

    Ok(Template::render(
        "requirement_statuses/edit_requirement_status",
        ctx,
    ))
}

#[post(
    "/<namespace>/<project_id>/requirement_statuses/edit/<status_id>",
    data = "<form>"
)]
async fn post_edit_requirement_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    status_id: i32,
    form: Form<NewRequirementStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let edit_url = format!("/{project_slug}/requirement_statuses/edit/{status_id}");
    let show_url = format!("/{project_slug}/requirement_statuses");

    let mut payload = form.into_inner();
    payload.id = Some(status_id);
    payload.project_id = project_id;
    payload.tag_color = payload.tag_color.filter(|s| !s.is_empty());

    if let Err(_e) = service.update_requirement_status(status_id, &payload) {
        #[cfg(debug_assertions)]
        eprintln!("update_requirement_status error: {:?}", _e);
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<namespace>/<project_id>/requirement_statuses/delete/<status_id>")]
async fn delete_requirement_status_route(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    status_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let project_id = {
        let _project_slug = project_id;
        project_access.project_id()
    };
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let status = match service.get_requirement_status(status_id) {
        Ok(s) => s,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if status.project_id != project_id {
        let status_project_slug = get_project_slug_by_id_pooled_safe(state, status.project_id);
        return Err(Redirect::to(format!(
            "/{status_project_slug}/requirement_statuses"
        )));
    }

    match service.delete_requirement_status(status_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_requirement_status error: {:?}", _e);
            Ok(rocket::http::Status::BadRequest)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_requirement_statuses,
        new_requirement_status,
        post_requirement_status,
        get_edit_requirement_status,
        post_edit_requirement_status,
        delete_requirement_status_route,
    ]
}
