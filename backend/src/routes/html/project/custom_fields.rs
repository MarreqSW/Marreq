// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! HTML routes for project-scoped custom field definitions (admin UI).

use super::helpers::*;
use super::prelude::*;
use crate::models::CustomFieldDefinitionPayload;
use crate::permissions::{has_permission, Permission};
use crate::services::CustomFieldService;
use rocket::form::FromForm;

#[derive(FromForm)]
pub struct CustomFieldForm {
    pub label: String,
    pub field_type: String,
    /// Comma- or newline-separated values for enum type.
    pub enum_values: Option<String>,
    pub sort_order: Option<i32>,
}

fn form_to_payload(form: CustomFieldForm) -> CustomFieldDefinitionPayload {
    let enum_values = form.enum_values.and_then(|s| {
        let v: Vec<String> = s
            .split([',', '\n'])
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    });
    CustomFieldDefinitionPayload {
        label: form.label,
        field_type: form.field_type,
        enum_values,
        sort_order: form.sort_order,
    }
}

fn list_url(project_slug: &str) -> String {
    format!("/p/{project_slug}/custom_fields")
}

#[get("/<project_id>/custom_fields?<error>&<count>")]
async fn show_custom_fields(
    project_access: HtmlProjectAccess,
    project_id: String,
    error: Option<&str>,
    count: Option<i64>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = CustomFieldService::new(state.inner());
    let custom_fields = service.list_by_project(project_id).unwrap_or_default();
    let in_use_counts: std::collections::HashMap<i32, i64> = custom_fields
        .iter()
        .filter_map(|def| {
            service
                .count_versions_using_field(def.id)
                .ok()
                .map(|c| (def.id, c))
        })
        .collect();

    let delete_error_message = if error == Some("in_use") {
        count.map(|n| format!("Cannot delete: field is in use by {} requirement version(s). Remove or update those values first.", n))
    } else {
        None
    };

    let mut ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "custom_fields": custom_fields,
        "in_use_counts": in_use_counts,
        "delete_error_message": delete_error_message,
        "page_title": "Custom fields"
    });
    if let Some(ctx_obj) = ctx.as_object_mut() {
        let perms = super::helpers::project_permissions_context(state, &user, project_id);
        if let Some(perms_obj) = perms.as_object() {
            for (k, v) in perms_obj {
                ctx_obj.insert(k.clone(), v.clone());
            }
        }
    }

    Ok(Template::render("custom_fields/custom_fields", ctx))
}

#[get("/<project_id>/custom_fields/new")]
async fn new_custom_field(
    project_access: HtmlProjectAccess,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::ManageCustomFields,
    ) {
        return Err(Redirect::to(list_url(&project_slug)));
    }
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": "New Custom Field"
    });
    Ok(Template::render("custom_fields/new_custom_field", ctx))
}

#[post("/<project_id>/custom_fields/new", data = "<form>")]
async fn post_custom_field(
    project_access: HtmlProjectAccess,
    project_id: String,
    form: Form<CustomFieldForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::ManageCustomFields,
    ) {
        return Ok(Redirect::to(list_url(&project_slug)));
    }
    let service = CustomFieldService::new(state.inner());

    let new_url = format!("/p/{project_slug}/custom_fields/new");
    let show_url = list_url(&project_slug);

    let payload = form_to_payload(form.into_inner());
    if let Err(_e) = service.create(project_id, payload) {
        #[cfg(debug_assertions)]
        eprintln!("create_custom_field error: {:?}", _e);
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url.clone()))
}

#[get("/<project_id>/custom_fields/edit/<field_id>")]
async fn get_edit_custom_field(
    project_access: HtmlProjectAccess,
    project_id: String,
    field_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::ManageCustomFields,
    ) {
        return Err(Redirect::to(list_url(&project_slug)));
    }
    let service = CustomFieldService::new(state.inner());

    let custom_field = service
        .get_by_id(field_id)
        .map_err(|_| Redirect::to(list_url(&project_slug)))?;

    if custom_field.project_id != project_id {
        let custom_field_project_slug =
            get_project_slug_by_id_pooled_safe(state, custom_field.project_id);
        return Err(Redirect::to(list_url(&custom_field_project_slug)));
    }

    let in_use_count = service.count_versions_using_field(field_id).unwrap_or(0);
    let projects = get_accessible_projects(state, &user);
    let enum_values_string: String = custom_field
        .enum_values
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .map(|v| v.join("\n"))
        .unwrap_or_default();

    let ctx = json!({
        "custom_field": custom_field,
        "enum_values_string": enum_values_string,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "in_use_count": in_use_count,
        "page_title": format!("Edit {} - Custom Field", custom_field.label)
    });

    Ok(Template::render("custom_fields/edit_custom_field", ctx))
}

#[post("/<project_id>/custom_fields/edit/<field_id>", data = "<form>")]
async fn post_edit_custom_field(
    project_access: HtmlProjectAccess,
    project_id: String,
    field_id: i32,
    form: Form<CustomFieldForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::ManageCustomFields,
    ) {
        return Ok(Redirect::to(list_url(&project_slug)));
    }
    let service = CustomFieldService::new(state.inner());

    let edit_url = format!("/p/{project_slug}/custom_fields/edit/{field_id}");
    let show_url = list_url(&project_slug);

    let old = service
        .get_by_id(field_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;

    if old.project_id != project_id {
        let old_project_slug = get_project_slug_by_id_pooled_safe(state, old.project_id);
        return Err(Redirect::to(list_url(&old_project_slug)));
    }

    let payload = form_to_payload(form.into_inner());
    if let Err(_e) = service.update(field_id, payload) {
        #[cfg(debug_assertions)]
        eprintln!("update_custom_field error: {:?}", _e);
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/custom_fields/delete/<field_id>")]
async fn delete_custom_field_route(
    project_access: HtmlProjectAccess,
    project_id: String,
    field_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, DeleteCustomFieldError> {
    let _project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::ManageCustomFields,
    ) {
        return Ok(rocket::http::Status::Forbidden);
    }
    let service = CustomFieldService::new(state.inner());

    let def = match service.get_by_id(field_id) {
        Ok(d) => d,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if def.project_id != project_id {
        let def_project_slug = get_project_slug_by_id_pooled_safe(state, def.project_id);
        return Err(DeleteCustomFieldError::Redirect(Box::new(Redirect::to(
            list_url(&def_project_slug),
        ))));
    }

    // Delete the definition; DB ON DELETE CASCADE removes all custom_field_values for this field.
    match service.delete(field_id) {
        Ok(()) => Ok(rocket::http::Status::Ok),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_custom_field error: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

#[derive(rocket::response::Responder)]
pub enum DeleteCustomFieldError {
    Redirect(Box<Redirect>),
}

pub fn routes() -> Vec<Route> {
    routes![
        show_custom_fields,
        new_custom_field,
        post_custom_field,
        get_edit_custom_field,
        post_edit_custom_field,
        delete_custom_field_route
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_url_format() {
        assert_eq!(list_url("my-project"), "/p/my-project/custom_fields");
        assert_eq!(list_url("orbiter"), "/p/orbiter/custom_fields");
    }

    #[test]
    fn form_to_payload_label_and_type() {
        let form = CustomFieldForm {
            label: "Priority".to_string(),
            field_type: "text".to_string(),
            enum_values: None,
            sort_order: None,
        };
        let payload = form_to_payload(form);
        assert_eq!(payload.label, "Priority");
        assert_eq!(payload.field_type, "text");
        assert!(payload.enum_values.is_none());
        assert!(payload.sort_order.is_none());
    }

    #[test]
    fn form_to_payload_enum_comma_separated() {
        let form = CustomFieldForm {
            label: "Status".to_string(),
            field_type: "enum".to_string(),
            enum_values: Some("Low, Medium, High".to_string()),
            sort_order: Some(0),
        };
        let payload = form_to_payload(form);
        assert_eq!(payload.enum_values.as_ref().map(|v| v.len()), Some(3));
        assert_eq!(payload.sort_order, Some(0));
    }

    #[test]
    fn form_to_payload_enum_newline_separated() {
        let form = CustomFieldForm {
            label: "X".to_string(),
            field_type: "enum".to_string(),
            enum_values: Some("A\nB\nC".to_string()),
            sort_order: None,
        };
        let payload = form_to_payload(form);
        assert_eq!(
            payload.enum_values.as_deref(),
            Some(vec!["A".to_string(), "B".to_string(), "C".to_string()].as_slice())
        );
    }

    #[test]
    fn routes_count() {
        let r = routes();
        assert_eq!(r.len(), 6);
    }
}
