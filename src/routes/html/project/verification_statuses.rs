// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! HTML routes for project-scoped verification status management.
//! System statuses are shown but not editable or deletable.

use super::helpers::*;
use super::prelude::*;
use crate::services::StatusService;

#[get("/<project_id>/verification_statuses")]
async fn show_verification_statuses(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = StatusService::new(state.inner());
    let statuses = service
        .list_verification_statuses_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "verification_statuses": statuses,
        "page_title": "Verification statuses"
    });

    Ok(Template::render("verification_statuses/verification_statuses", ctx))
}

#[get("/<project_id>/verification_statuses/new")]
async fn new_verification_status(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "page_title": "New verification status"
    });
    Ok(Template::render("verification_statuses/new_verification_status", ctx))
}

#[post("/<project_id>/verification_statuses/new", data = "<form>")]
async fn post_verification_status(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<NewVerificationStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let new_url = uri!("/p", new_verification_status(project_id));
    let show_url = uri!("/p", show_verification_statuses(project_id));

    let mut payload = form.into_inner();
    payload.project_id = project_id;
    payload.tag_color = payload.tag_color.filter(|s| !s.is_empty());

    if let Err(_e) = service.create_verification_status(payload) {
        #[cfg(debug_assertions)]
        eprintln!("create_verification_status error: {:?}", _e);
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/verification_statuses/edit/<status_id>")]
async fn get_edit_verification_status(
    project_access: ProjectAccess,
    project_id: i32,
    status_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let status = service
        .get_verification_status(status_id)
        .map_err(|_| Redirect::to(uri!("/p", show_verification_statuses(project_id))))?;

    if status.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_verification_statuses(project_id = status.project_id)
        )));
    }

    if status.is_system {
        return Err(Redirect::to(uri!("/p", show_verification_statuses(project_id))));
    }

    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "verification_status": status,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "page_title": format!("Edit {} - Verification status", status.title)
    });

    Ok(Template::render("verification_statuses/edit_verification_status", ctx))
}

#[post("/<project_id>/verification_statuses/edit/<status_id>", data = "<form>")]
async fn post_edit_verification_status(
    project_access: ProjectAccess,
    project_id: i32,
    status_id: i32,
    form: Form<NewVerificationStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let edit_url = uri!("/p", get_edit_verification_status(project_id, status_id));
    let show_url = uri!("/p", show_verification_statuses(project_id));

    let mut payload = form.into_inner();
    payload.id = Some(status_id);
    payload.project_id = project_id;
    payload.tag_color = payload.tag_color.filter(|s| !s.is_empty());

    if let Err(_e) = service.update_verification_status(status_id, &payload) {
        #[cfg(debug_assertions)]
        eprintln!("update_verification_status error: {:?}", _e);
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/verification_statuses/delete/<status_id>")]
async fn delete_verification_status_route(
    project_access: ProjectAccess,
    project_id: i32,
    status_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let status = match service.get_verification_status(status_id) {
        Ok(s) => s,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if status.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_verification_statuses(project_id = status.project_id)
        )));
    }

    match service.delete_verification_status(status_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_verification_status error: {:?}", _e);
            Ok(rocket::http::Status::BadRequest)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_verification_statuses,
        new_verification_status,
        post_verification_status,
        get_edit_verification_status,
        post_edit_verification_status,
        delete_verification_status_route,
    ]
}
