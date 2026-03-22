// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(unused_variables)]

//! HTML routes for project-scoped verification status management.
//! System statuses are shown but not editable or deletable.

use super::helpers::*;
use super::prelude::*;
use crate::services::StatusService;

#[get("/<namespace>/<project_id>/verification_statuses")]
async fn show_verification_statuses(
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
        .list_verification_statuses_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "verification_statuses": statuses,
        "page_title": "Verification statuses"
    });

    Ok(Template::render(
        "verification_statuses/verification_statuses",
        ctx,
    ))
}

#[get("/<namespace>/<project_id>/verification_statuses/new")]
async fn new_verification_status(
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
        "page_title": "New verification status"
    });
    Ok(Template::render(
        "verification_statuses/new_verification_status",
        ctx,
    ))
}

#[post("/<namespace>/<project_id>/verification_statuses/new", data = "<form>")]
async fn post_verification_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    form: Form<NewVerificationStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let new_url = format!("/{project_slug}/verification_statuses/new");
    let show_url = format!("/{project_slug}/verification_statuses");

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

#[get("/<namespace>/<project_id>/verification_statuses/edit/<status_id>")]
async fn get_edit_verification_status(
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
        .get_verification_status(status_id)
        .map_err(|_| Redirect::to(format!("/{project_slug}/verification_statuses")))?;

    if status.project_id != project_id {
        let status_project_slug = get_project_slug_by_id_pooled_safe(state, status.project_id);
        return Err(Redirect::to(format!(
            "/{status_project_slug}/verification_statuses"
        )));
    }

    if status.is_system {
        return Err(Redirect::to(format!(
            "/{project_slug}/verification_statuses"
        )));
    }

    let projects: Vec<_> = get_accessible_projects(state, &user)
        .iter()
        .map(|project| project_to_template_value(state, project))
        .collect();

    let ctx = json!({
        "verification_status": status,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": format!("Edit {} - Verification status", status.title)
    });

    Ok(Template::render(
        "verification_statuses/edit_verification_status",
        ctx,
    ))
}

#[post(
    "/<namespace>/<project_id>/verification_statuses/edit/<status_id>",
    data = "<form>"
)]
async fn post_edit_verification_status(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    status_id: i32,
    form: Form<NewVerificationStatus>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let edit_url = format!("/{project_slug}/verification_statuses/edit/{status_id}");
    let show_url = format!("/{project_slug}/verification_statuses");

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

#[delete("/<namespace>/<project_id>/verification_statuses/delete/<status_id>")]
async fn delete_verification_status_route(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    status_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let service = StatusService::new(state.inner());

    let status = match service.get_verification_status(status_id) {
        Ok(s) => s,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if status.project_id != project_id {
        let status_project_slug = get_project_slug_by_id_pooled_safe(state, status.project_id);
        return Err(Redirect::to(format!(
            "/{status_project_slug}/verification_statuses"
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
