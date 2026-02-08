//! ReqIF 1.2 export and import routes.

use super::prelude::*;
use crate::reqif::import::ImportConfig;
use crate::services::semantic_search::{IndexingService, SemanticSearchConfig};
use crate::services::{
    ApplicabilityService, CategoryService, ReqIFService, StatusService, UserService,
    VerificationService,
};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::response::content;

#[get("/<project_id>/export_reqif?<baseline_id>")]
pub async fn export_reqif(
    project_access: ProjectAccess,
    project_id: i32,
    baseline_id: Option<i32>,
    state: &State<AppState>,
) -> Result<(ContentType, String), Redirect> {
    let _user = project_access.into_user();
    let service = ReqIFService::new(state.inner());
    let xml = match baseline_id {
        Some(bid) => service.export_baseline(project_id, bid).map_err(|e| {
            eprintln!("ReqIF baseline export error: {:?}", e);
            Redirect::to(format!("/p/{}/requirements", project_id))
        })?,
        None => service.export_project(project_id).map_err(|e| {
            eprintln!("ReqIF export error: {:?}", e);
            Redirect::to(format!("/p/{}/requirements", project_id))
        })?,
    };
    let ct = ContentType::new("application", "xml");
    Ok((ct, xml))
}

#[get("/<project_id>/import_reqif?<error>")]
pub fn import_reqif_page(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<content::RawHtml<String>, Box<Redirect>> {
    let _user = project_access.into_user();
    let project = crate::services::ProjectService::new(state.inner())
        .get_by_id(project_id)
        .map_err(|_| Box::new(Redirect::to(format!("/p/{}", project_id))))?;
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
    <title>ReqMan - Import ReqIF</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
    <link rel="stylesheet" href="/static/reqman.css">
</head>
<body>
    <div class="container mt-4">
        <div class="row">
            <div class="col-md-8 offset-md-2">
                <div class="card">
                    <div class="card-header">
                        <h3>Import ReqIF</h3>
                    </div>
                    <div class="card-body">
                        <div class="alert alert-info">
                            <strong>Target Project:</strong> {} (ID: {})
                            <br>
                            <small class="text-muted">Requirements will be imported from the ReqIF file into this project.</small>
                        </div>
                        {}
                        <p>Upload a ReqIF 1.2 file to import requirements into the selected project.</p>
                        <form action="/p/{}/import_reqif/process" method="post" enctype="multipart/form-data">
                            <div class="mb-3">
                                <label for="reqif_file" class="form-label">Select ReqIF file</label>
                                <input type="file" class="form-control" id="reqif_file" name="file" accept=".reqif,.xml" required>
                                <div class="form-text">Supported: .reqif, .xml (ReqIF 1.2)</div>
                            </div>
                            <div class="mt-3">
                                <button type="submit" class="btn btn-primary">Import</button>
                                <a href="/p/{}/requirements" class="btn btn-secondary">Cancel</a>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    </div>
</body>
</html>"#,
        project.name,
        project_id,
        error
            .as_ref()
            .map(|m| format!("<div class=\"alert alert-danger\">{}</div>", m))
            .unwrap_or_default(),
        project_id,
        project_id
    );
    Ok(content::RawHtml(html))
}

#[derive(FromForm)]
pub struct ReqIFUpload<'r> {
    pub file: TempFile<'r>,
}

#[post("/<project_id>/import_reqif/process", data = "<upload>")]
pub async fn process_reqif_import(
    project_access: ProjectAccess,
    project_id: i32,
    upload: Form<ReqIFUpload<'_>>,
    state: &State<AppState>,
) -> Result<content::RawHtml<String>, Redirect> {
    let user = project_access.into_user();
    let temp_path = upload.file.path().ok_or_else(|| {
        Redirect::to(uri!(import_reqif_page(
            project_id = project_id,
            error = Some("No file uploaded.".to_string())
        )))
    })?;

    let xml_bytes = std::fs::read(temp_path).map_err(|e| {
        eprintln!("ReqIF read error: {}", e);
        Redirect::to(uri!(import_reqif_page(
            project_id = project_id,
            error = Some("Could not read uploaded file.".to_string())
        )))
    })?;

    let status_service = StatusService::new(state.inner());
    let category_service = CategoryService::new(state.inner());
    let applicability_service = ApplicabilityService::new(state.inner());
    let verification_service = VerificationService::new(state.inner());
    let user_service = UserService::new(state.inner());

    let statuses = status_service
        .list_requirement_statuses_by_project(project_id)
        .ok();
    let default_status_id = statuses
        .as_ref()
        .and_then(|s| s.first().map(|st| st.id))
        .unwrap_or(1);
    let categories = category_service.list_by_project(project_id).ok();
    let default_category_id = categories
        .as_ref()
        .and_then(|c| c.first().map(|x| x.id))
        .unwrap_or(1);
    let applicability = applicability_service.list_by_project(project_id).ok();
    let default_applicability_id = applicability
        .as_ref()
        .and_then(|a| a.first().map(|x| x.id))
        .unwrap_or(1);
    let verification = verification_service.list_by_project(project_id).ok();
    let default_verification_id = verification
        .as_ref()
        .and_then(|v| v.first().map(|x| x.id))
        .unwrap_or(1);
    let users = user_service.get_by_project(project_id).ok();
    let reviewer_id = users
        .as_ref()
        .and_then(|u| u.first().map(|x| x.id))
        .unwrap_or(user.id);

    let config = ImportConfig {
        project_id,
        default_status_id,
        default_category_id,
        default_applicability_id,
        default_verification_method_id: default_verification_id,
        author_id: user.id,
        reviewer_id,
    };

    let reqif_service = ReqIFService::new(state.inner());
    let result = reqif_service.import_into_project(&xml_bytes, &config, &user);

    let (success, message, imported_count, errors, imported_ids) = match result {
        Ok(r) => (
            r.success,
            r.message,
            r.imported_count,
            r.errors,
            r.imported_requirement_ids,
        ),
        Err(e) => {
            return Err(Redirect::to(uri!(import_reqif_page(
                project_id = project_id,
                error = Some(e)
            ))));
        }
    };

    state.repo_read().cache().clear();
    let config = SemanticSearchConfig::global();
    if config.embeddings_enabled && !imported_ids.is_empty() {
        let indexing_service = IndexingService::new(state.inner());
        for &req_id in &imported_ids {
            let _ = indexing_service.queue_for_indexing(req_id, project_id);
        }
    }

    let errors_html = if errors.is_empty() {
        String::new()
    } else {
        let list: String = errors.iter().map(|e| format!("<li>{}</li>", e)).collect();
        format!("<ul class=\"list-unstyled\">{}</ul>", list)
    };

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
    <title>ReqMan - ReqIF Import Result</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
    <link rel="stylesheet" href="/static/reqman.css">
</head>
<body>
    <div class="container mt-4">
        <div class="row">
            <div class="col-md-8 offset-md-2">
                <div class="card">
                    <div class="card-header">
                        <h3>ReqIF Import Result</h3>
                    </div>
                    <div class="card-body">
                        <div class="alert alert-{}">
                            <strong>{}</strong>
                            <p class="mb-0">{}</p>
                            <p class="mb-0"><strong>Records imported:</strong> {}</p>
                        </div>
                        {}
                        <a href="/p/{}/requirements" class="btn btn-primary">View requirements</a>
                        <a href="/p/{}/import_reqif" class="btn btn-outline-primary">Import another file</a>
                    </div>
                </div>
            </div>
        </div>
    </div>
</body>
</html>"#,
        if success { "success" } else { "warning" },
        if success {
            "Import completed"
        } else {
            "Import completed with errors"
        },
        message,
        imported_count,
        if errors.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="alert alert-secondary"><strong>Errors:</strong>{}</div>"#,
                errors_html
            )
        },
        project_id,
        project_id
    );
    Ok(content::RawHtml(html))
}

pub fn routes() -> Vec<Route> {
    routes![export_reqif, import_reqif_page, process_reqif_import]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Method;

    #[test]
    fn routes_returns_three_routes() {
        let r = routes();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn routes_include_export_and_import() {
        let r = routes();
        let paths: Vec<String> = r.iter().map(|route| route.uri.to_string()).collect();
        let has_export = paths.iter().any(|p| p.contains("export_reqif"));
        let has_import = paths.iter().any(|p| p.contains("import_reqif"));
        assert!(
            has_export,
            "expected a route containing export_reqif, got {:?}",
            paths
        );
        assert!(
            has_import,
            "expected a route containing import_reqif, got {:?}",
            paths
        );
    }

    #[test]
    fn export_route_is_get() {
        let r = routes();
        let export_route = r
            .iter()
            .find(|route| route.uri.to_string().contains("export_reqif"));
        assert!(export_route.is_some());
        assert_eq!(export_route.unwrap().method, Method::Get);
    }

    #[test]
    fn process_import_route_is_post() {
        let r = routes();
        let process_route = r
            .iter()
            .find(|route| route.uri.to_string().contains("process"));
        assert!(process_route.is_some());
        assert_eq!(process_route.unwrap().method, Method::Post);
    }

    #[test]
    fn import_reqif_route_has_error_query_param() {
        let r = routes();
        let import_page_route = r.iter().find(|route| {
            let u = route.uri.to_string();
            u.contains("import_reqif") && !u.contains("process")
        });
        assert!(
            import_page_route.is_some(),
            "import_reqif page route should exist"
        );
        let uri = import_page_route.unwrap().uri.to_string();
        assert!(
            uri.contains("error"),
            "import_reqif page should accept optional error query param, got uri: {}",
            uri
        );
    }

    #[test]
    fn reqif_upload_struct_used_in_process_route() {
        // ReqIFUpload is the form data type for process_reqif_import; ensure it's present in module
        let _: Option<ReqIFUpload<'_>> = None;
    }
}
