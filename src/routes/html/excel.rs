// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]
#![allow(unused_variables)]

use super::helpers::*;
use super::prelude::*;
use crate::services::semantic_search::{IndexingService, SemanticSearchConfig};
use std::path::Path;

/// Queue imported requirements for semantic search indexing.
///
/// This is a best-effort operation - failures are logged but don't affect the import.
fn queue_requirements_for_indexing(
    state: &State<AppState>,
    project_id: i32,
    requirement_ids: &[i32],
) {
    let config = SemanticSearchConfig::global();
    if !config.embeddings_enabled || requirement_ids.is_empty() {
        return;
    }

    let indexing_service = IndexingService::new(state.inner());
    let mut queued = 0;
    for &req_id in requirement_ids {
        if indexing_service
            .queue_for_indexing(req_id, project_id)
            .is_ok()
        {
            queued += 1;
        }
    }

    if queued > 0 {
        eprintln!(
            "📊 Queued {} imported requirements for semantic indexing",
            queued
        );
    }
}

fn render_import_page_html(
    name: &str,
    project_id: i32,
    project_slug: &str,
    error_html: &str,
) -> String {
    format!(
        r#"
    <!doctype html>
    <html lang='en'>
    <head>
        <title>Marreq - Import File</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/marreq.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-8 offset-md-2">
                    <div class="card">
                        <div class="card-header">
                            <h3>Import File</h3>
                        </div>
                        <div class="card-body">
                            <div class="alert alert-info">
                                <strong>Target Project:</strong> {} (ID: {})
                                <br>
                                <small class="text-muted">Requirements and tests will be imported into this project. You can change the project using the dropdown in the navigation bar above.</small>
                            </div>
                            {}
                            <p>Upload a file to import requirements or tests into the selected project.</p>
                            <form action="/{}/import_excel/upload" method="post" enctype="multipart/form-data">
                                <div class="mb-3">
                                    <label for="excel_file" class="form-label">Select File</label>
                                    <input type="file" class="form-control" id="excel_file" name="file" accept=".xlsx,.xls,.csv" required>
                                    <div class="form-text">Supported formats: .xlsx, .xls, .csv</div>
                                </div>
                                <div class="mt-3">
                                    <button type="submit" class="btn btn-primary">Upload File</button>
                                    <a href="/" class="btn btn-secondary">Cancel</a>
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </body>
    </html>
    "#,
        name, project_id, error_html, project_slug
    )
}

#[get("/<namespace>/<project_id>/import_excel?<error>")]
pub fn import_excel_page(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<content::RawHtml<String>, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();

    let project = get_project_by_id_pooled_safe(state, project_id);
    let name = project.name;
    let error_html = error
        .as_ref()
        .map(|message| format!("<div class=\"alert alert-danger\">{}</div>", message))
        .unwrap_or_default();
    let html = render_import_page_html(&name, project_id, &project_slug, &error_html);
    Ok(content::RawHtml(html))
}

#[post("/<namespace>/<project_id>/import_excel/upload", data = "<upload>")]
pub async fn upload_excel_file(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    mut upload: rocket::form::Form<rocket::fs::TempFile<'_>>,
) -> Result<content::RawHtml<String>, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let _project_id = project_access.project_id();
    let _user = project_access.into_user();

    // Save uploaded file temporarily
    let filename = upload.name().unwrap_or("upload");
    let mut extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if extension.is_empty() {
        if let Some(content_type) = upload.content_type() {
            let content_type = content_type.to_string().to_ascii_lowercase();
            if content_type.contains("text/csv") {
                extension = "csv".to_string();
            } else if content_type.contains("application/vnd.ms-excel") {
                extension = "xls".to_string();
            } else if content_type
                .contains("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
            {
                extension = "xlsx".to_string();
            }
        }
    }

    if extension.is_empty() {
        extension = "xlsx".to_string();
    }

    let is_supported = matches!(extension.as_str(), "xlsx" | "xls" | "csv");
    if !is_supported {
        return Err(Redirect::to(format!(
            "/{project_slug}/import_excel?error={}",
            urlencoding::encode("Unsupported file type. Use .xlsx, .xls, or .csv")
        )));
    }

    let temp_path = format!(
        "/tmp/upload_{}.{}",
        chrono::Utc::now().timestamp(),
        extension
    );
    upload.persist_to(&temp_path).await.map_err(|_| {
        Redirect::to(format!(
            "/{project_slug}/import_excel?error={}",
            urlencoding::encode("Failed to store upload. Please try again.")
        ))
    })?;

    // Parse Excel file
    let importer = crate::importers::excel::ExcelImporter::new(&temp_path).map_err(|e| {
        Redirect::to(format!(
            "/{project_slug}/import_excel?error={}",
            urlencoding::encode(&format!("Failed to parse file: {}", e))
        ))
    })?;

    // Create HTML for column mapping
    let _columns_html = importer
        .columns
        .iter()
        .map(|col| format!("<option value=\"{}\">{}</option>", col.name, col.name))
        .collect::<Vec<_>>()
        .join("");

    let available_fields_html = importer
        .get_available_fields()
        .iter()
        .map(|field| format!("<option value=\"{}\">{}</option>", field, field))
        .collect::<Vec<_>>()
        .join("");

    let html = format!(
        r#"
    <!doctype html>
    <html lang='en'>
    <head>
        <title>Marreq - Map Columns</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/marreq.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-10 offset-md-1">
                    <div class="card">
                        <div class="card-header">
                            <h3>Map Columns</h3>
                            <p class="mb-0">Import Type: <strong>{}</strong> | Data Rows: <strong>{}</strong></p>
                        </div>
                        <div class="card-body">
                            <form action="/{}/import_excel/process" method="post" id="mapping-form">
                                <input type="hidden" name="import_type" value="{}">
                                <input type="hidden" name="temp_file" value="{}">
                                <input type="hidden" name="column_mappings" id="column_mappings" value="">
                                
                                <div class="table-responsive">
                                    <table class="table table-bordered">
                                        <thead>
                                            <tr>
                                                <th>Excel Column</th>
                                                <th>Map To Field</th>
                                                <th>Sample Data</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {}
                                        </tbody>
                                    </table>
                                </div>
                                
                                <div class="mt-3">
                                    <button type="submit" class="btn btn-primary">Import Data</button>
                                    <a href="/{}/import_excel" class="btn btn-secondary">Cancel</a>
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <script>
        document.addEventListener('DOMContentLoaded', function() {{
            const form = document.getElementById('mapping-form');
            form.addEventListener('submit', function(e) {{
                e.preventDefault();
                
                const mappings = [];
                const rows = document.querySelectorAll('tbody tr');
                rows.forEach(function(row) {{
                    const column = row.querySelector('td:first-child').textContent.trim();
                    const field = row.querySelector('select[name^="field"]').value;
                    if (field && field !== '') {{
                        mappings.push({{
                            excel_column: column,
                            target_field: field
                        }});
                    }}
                }});
                
                document.getElementById('column_mappings').value = JSON.stringify(mappings);
                form.submit();
            }});
        }});
        </script>
    </body>
    </html>
    "#,
        importer.import_type,
        importer.data.len(),
        project_slug,
        importer.import_type,
        temp_path,
        importer
            .columns
            .iter()
            .map(|col| {
                let sample_data = &col.sample_value;
                format!(
                    r#"<tr>
                    <td>{}</td>
                    <td>
                        <select name="field_{}" class="form-select">
                            <option value="">-- Select Field --</option>
                            {}
                        </select>
                    </td>
                    <td><small class="text-muted">{}</small></td>
                </tr>"#,
                    col.name,
                    col.name.replace(" ", "_"),
                    available_fields_html,
                    sample_data
                )
            })
            .collect::<Vec<_>>()
            .join(""),
        project_slug
    );

    Ok(content::RawHtml(html))
}

#[post(
    "/<namespace>/<project_id>/import_excel/process",
    data = "<mapping_data>"
)]
pub fn process_excel_import(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    mapping_data: Form<crate::models::ImportMappingForm>,
    state: &State<AppState>,
) -> Result<content::RawHtml<String>, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let _user = project_access.into_user();

    eprintln!("Column mappings string: {}", mapping_data.column_mappings);

    // Parse column mappings
    let column_mappings: Vec<crate::importers::excel::ColumnMapping> =
        serde_json::from_str(&mapping_data.column_mappings).map_err(|e| {
            eprintln!("JSON parsing error: {}", e);
            Redirect::to(format!(
                "/{project_slug}/import_excel?error={}",
                urlencoding::encode("Invalid column mapping data.")
            ))
        })?;

    // Create importer and import data
    let importer =
        crate::importers::excel::ExcelImporter::new(&mapping_data.temp_file).map_err(|e| {
            eprintln!("Excel importer creation error: {}", e);
            Redirect::to(format!(
                "/{project_slug}/import_excel?error={}",
                urlencoding::encode("Unable to read uploaded file. Please re-upload.")
            ))
        })?;

    // Create import configuration
    let config = crate::importers::excel::ImportConfig {
        import_type: mapping_data.import_type.clone(),
        column_mappings,
        project_id,
    };

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(format!(
            "/{project_slug}/import_excel?error={}",
            urlencoding::encode("Database connection failed.")
        ))
    })?;
    let result = importer.import_data(&config, connection);

    eprintln!("Import result: {:?}", result);

    let html = match result {
        Ok(import_result) => {
            // Invalidate all caches after successful import since we don't know exactly what was imported
            state.repo_read().cache().clear();

            // Queue imported requirements for semantic search indexing
            queue_requirements_for_indexing(
                state,
                project_id,
                &import_result.imported_requirement_ids,
            );

            // Get project name for display
            let name = get_project_by_id_pooled_safe(state, project_id).name;

            format!(
                r#"
            <!doctype html>
            <html lang='en'>
            <head>
                <title>Marreq - Import Results</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/marreq.css'>
            </head>
            <body>
                <div class="container mt-4">
                    <div class="row">
                        <div class="col-md-8 offset-md-2">
                            <div class="card border-success">
                                <div class="card-header bg-success text-white">
                                    <h3><i class="fas fa-check-circle"></i> Import Successful</h3>
                                </div>
                                <div class="card-body">
                                    <div class="alert alert-success">
                                        <h5>Import completed successfully!</h5>
                                        <p><strong>Records imported:</strong> {}</p>
                                        <p><strong>Import type:</strong> {}</p>
                                        <p><strong>Target project:</strong> {} (ID: {})</p>
                                    </div>
                                    
                                    <div class="mt-3">
                                        <a href="/" class="btn btn-primary">Back to Home</a>
                                        <a href="/{}/import_excel" class="btn btn-outline-primary">Import Another File</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#,
                import_result.imported_count,
                mapping_data.import_type,
                name,
                project_id,
                project_slug
            )
        }
        Err(e) => {
            // Get project name for display
            let name = get_project_by_id_pooled_safe(state, project_id).name;

            format!(
                r#"
            <!doctype html>
            <html lang='en'>
            <head>
                <title>Marreq - Import Error</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/marreq.css'>
            </head>
            <body>
                <div class="container mt-4">
                    <div class="row">
                        <div class="col-md-8 offset-md-2">
                            <div class="card border-danger">
                                <div class="card-header bg-danger text-white">
                                    <h3><i class="fas fa-exclamation-triangle"></i> Import Failed</h3>
                                </div>
                                <div class="card-body">
                                    <div class="alert alert-danger">
                                        <h5>Import failed!</h5>
                                        <p><strong>Error:</strong> {}</p>
                                        <p><strong>Target project:</strong> {} (ID: {})</p>
                                    </div>
                                    
                                    <div class="mt-3">
                                        <a href="/{}/import_excel" class="btn btn-primary">Try Again</a>
                                        <a href="/" class="btn btn-secondary">Back to Home</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#,
                e, name, project_id, project_slug
            )
        }
    };

    Ok(content::RawHtml(html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_page_html_contains_expected_content() {
        let html = render_import_page_html("Test Project", 1, "test-project", "");
        assert!(html.contains("Import File"));
        assert!(html.contains("Target Project"));
        assert!(html.contains("Test Project"));
        assert!(html.contains("/test-project/import_excel/upload"));
        assert!(html.contains(".xlsx,.xls,.csv"));
    }

    #[test]
    fn import_page_html_includes_error_when_provided() {
        let html =
            render_import_page_html("P", 2, "p", "<div class=\"alert alert-danger\">Oops</div>");
        assert!(html.contains("alert-danger"));
        assert!(html.contains("Oops"));
    }
}
