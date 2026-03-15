// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]

use super::helpers::{build_context_with_projects, project_permissions_context};
use super::prelude::*;
use crate::permissions::{has_permission, Permission};
use crate::services::DocumentImportService;
use crate::services::ProjectService;
use rocket::form::FromForm;
use rocket::fs::TempFile;
use rocket::http::ContentType;
use std::path::Path;

#[derive(FromForm)]
pub struct DocumentUpload<'r> {
    pub file: TempFile<'r>,
    pub use_ai: Option<String>,
}

fn requirements_redirect(project_id: i32) -> Redirect {
    Redirect::to(format!("/p/{project_id}/requirements"))
}

fn import_page_redirect(project_id: i32, error: impl Into<String>) -> Redirect {
    let encoded = urlencoding::encode(&error.into()).into_owned();
    Redirect::to(format!("/p/{project_id}/import_document?error={encoded}"))
}

fn infer_upload_extension(content_type: Option<&ContentType>) -> Option<&'static str> {
    let content_type = content_type?.to_string().to_ascii_lowercase();
    if content_type.contains("application/pdf") {
        Some("pdf")
    } else if content_type
        .contains("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    {
        Some("docx")
    } else {
        None
    }
}

fn normalized_upload_filename(upload: &TempFile<'_>) -> String {
    let raw_name = upload.name().map(str::trim).filter(|name| !name.is_empty());
    let inferred_extension = infer_upload_extension(upload.content_type());

    match raw_name {
        Some(name) => {
            let has_extension = Path::new(name)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(str::trim)
                .is_some_and(|ext| !ext.is_empty());
            if has_extension {
                name.to_string()
            } else if let Some(extension) = inferred_extension {
                format!("{name}.{extension}")
            } else {
                name.to_string()
            }
        }
        None => inferred_extension
            .map(|extension| format!("uploaded-document.{extension}"))
            .unwrap_or_else(|| "uploaded-document".to_string()),
    }
}

#[get("/<project_id>/import_document?<error>")]
pub async fn import_document_page(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    error: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::EditRequirements,
    ) {
        return Err(requirements_redirect(project_id));
    }

    let project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let mut ctx = build_context_with_projects(state.inner(), user.clone(), cookies);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "project".into(),
            json!({
                "id": project.id,
                "name": project.name,
            }),
        );
        obj.insert("page_title".into(), json!("Document Import"));
        obj.insert("error".into(), json!(error.unwrap_or_default()));
        obj.insert(
            "supported_formats".into(),
            json!(vec!["PDF (.pdf)", "Word (.docx)"]),
        );
        if let Some(perms) =
            project_permissions_context(state.inner(), &user, project_id).as_object()
        {
            for (key, value) in perms {
                obj.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(Template::render("document_import/upload", ctx))
}

#[post("/<project_id>/import_document/upload", data = "<upload>")]
pub async fn upload_document_import(
    project_access: ProjectAccess,
    project_id: i32,
    upload: Form<DocumentUpload<'_>>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::EditRequirements,
    ) {
        return Err(requirements_redirect(project_id));
    }

    let temp_path = upload.file.path().ok_or_else(|| {
        import_page_redirect(
            project_id,
            "No file was uploaded. Please choose a PDF or DOCX file.",
        )
    })?;
    let bytes = std::fs::read(temp_path).map_err(|_| {
        import_page_redirect(
            project_id,
            "The uploaded file could not be read. Please try again.",
        )
    })?;
    let filename = normalized_upload_filename(&upload.file);
    let service = DocumentImportService::new(state.inner());
    let session = service
        .create_session_from_bytes(
            project_id,
            &user,
            &filename,
            &bytes,
            upload.use_ai.is_some(),
        )
        .await
        .map_err(|err| import_page_redirect(project_id, err.to_string()))?;

    Ok(Redirect::to(format!(
        "/p/{project_id}/import_document/review/{}",
        session.session_id
    )))
}

#[get("/<project_id>/import_document/review/<session_id>")]
pub async fn review_document_import_page(
    project_access: ProjectAccess,
    project_id: i32,
    session_id: &str,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    if !has_permission(
        &*state.repo_read(),
        &user,
        project_id,
        Permission::EditRequirements,
    ) {
        return Err(requirements_redirect(project_id));
    }

    let project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let service = DocumentImportService::new(state.inner());
    let session = service
        .get_session(project_id, user.id, session_id)
        .map_err(|_| {
            import_page_redirect(project_id, "That import session does not exist anymore.")
        })?;

    let mut ctx = build_context_with_projects(state.inner(), user.clone(), cookies);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "project".into(),
            json!({
                "id": project.id,
                "name": project.name,
            }),
        );
        obj.insert(
            "page_title".into(),
            json!(format!("Review Document Import - {}", project.name)),
        );
        obj.insert("session_id".into(), json!(session.session_id));
        obj.insert("filename".into(), json!(session.filename));
        if let Some(perms) =
            project_permissions_context(state.inner(), &user, project_id).as_object()
        {
            for (key, value) in perms {
                obj.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(Template::render("document_import/review", ctx))
}

pub fn routes() -> Vec<Route> {
    routes![
        import_document_page,
        upload_document_import,
        review_document_import_page
    ]
}
