use super::helpers::*;
use super::prelude::*;

#[get("/import_excel")]
pub fn import_excel_page(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<content::RawHtml<String>, Redirect> {
    let _user = session_user.into_inner();

    // Get selected project ID and name
    let selected_project_id = get_selected_project_id(cookies);
    let (project_id, name) = if let Some(pid) = selected_project_id {
        let project = get_project_by_id_pooled_safe(state, pid);
        (pid, project.name)
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            (first_project.project_id, first_project.name.clone())
        } else {
            (1, "Default Project".to_string())
        }
    };

    let html = format!(
        r#"
    <!doctype html>
    <html lang='en'>
    <head>
        <title>ReqMan - Import Excel</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/reqman.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-8 offset-md-2">
                    <div class="card">
                        <div class="card-header">
                            <h3>Import Excel File</h3>
                        </div>
                        <div class="card-body">
                            <div class="alert alert-info">
                                <strong>Target Project:</strong> {} (ID: {})
                                <br>
                                <small class="text-muted">Requirements and tests will be imported into this project. You can change the project using the dropdown in the navigation bar above.</small>
                            </div>
                            <p>Upload an Excel file to import requirements or tests into the selected project.</p>
                            <form action="/import_excel/upload" method="post" enctype="multipart/form-data">
                                <div class="mb-3">
                                    <label for="excel_file" class="form-label">Select Excel File</label>
                                    <input type="file" class="form-control" id="excel_file" name="file" accept=".xlsx,.xls" required>
                                    <div class="form-text">Supported formats: .xlsx, .xls</div>
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
        name, project_id
    );

    Ok(content::RawHtml(html))
}

#[post("/import_excel/upload", data = "<upload>")]
pub async fn upload_excel_file(
    session_user: SessionUser,
    mut upload: rocket::form::Form<rocket::fs::TempFile<'_>>,
) -> Result<content::RawHtml<String>, Redirect> {
    let _user = session_user.into_inner();

    // Save uploaded file temporarily
    let temp_path = format!("/tmp/upload_{}.xlsx", chrono::Utc::now().timestamp());
    upload
        .persist_to(&temp_path)
        .await
        .map_err(|_| Redirect::to(uri!(import_excel_page)))?;

    // Parse Excel file
    let importer = crate::importers::excel::ExcelImporter::new(&temp_path)
        .map_err(|_| Redirect::to(uri!(import_excel_page)))?;

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
        <title>ReqMan - Map Excel Columns</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/reqman.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-10 offset-md-1">
                    <div class="card">
                        <div class="card-header">
                            <h3>Map Excel Columns</h3>
                            <p class="mb-0">Import Type: <strong>{}</strong> | Data Rows: <strong>{}</strong></p>
                        </div>
                        <div class="card-body">
                            <form action="/import_excel/process" method="post" id="mapping-form">
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
                                    <a href="/import_excel" class="btn btn-secondary">Cancel</a>
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
            .join("")
    );

    Ok(content::RawHtml(html))
}

#[post("/import_excel/process", data = "<mapping_data>")]
pub fn process_excel_import(
    session_user: SessionUser,
    mapping_data: Form<crate::models::ImportMappingForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<content::RawHtml<String>, Redirect> {
    let _user = session_user.into_inner();

    eprintln!("Column mappings string: {}", mapping_data.column_mappings);

    // Parse column mappings
    let column_mappings: Vec<crate::importers::excel::ColumnMapping> =
        serde_json::from_str(&mapping_data.column_mappings).map_err(|e| {
            eprintln!("JSON parsing error: {}", e);
            Redirect::to(uri!(import_excel_page))
        })?;

    // Create importer and import data
    let importer =
        crate::importers::excel::ExcelImporter::new(&mapping_data.temp_file).map_err(|e| {
            eprintln!("Excel importer creation error: {}", e);
            Redirect::to(uri!(import_excel_page))
        })?;

    // Get selected project ID from cookies
    let selected_project_id = get_selected_project_id(cookies);
    let project_id = if let Some(pid) = selected_project_id {
        pid
    } else {
        // Default to the first project if no project is selected
        let projects = state.repo_read().get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            first_project.project_id
        } else {
            1 // Fallback to project 1 if no projects exist
        }
    };

    // Create import configuration
    let config = crate::importers::excel::ImportConfig {
        import_type: mapping_data.import_type.clone(),
        column_mappings,
        project_id,
    };

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(import_excel_page))
    })?;
    let result = importer.import_data(&config, connection);

    eprintln!("Import result: {:?}", result);

    let html = match result {
        Ok(import_result) => {
            // Invalidate all caches after successful import since we don't know exactly what was imported
            state.repo_read().cache().clear();

            // Get project name for display
            let name = get_project_by_id_pooled_safe(state, project_id).name;

            format!(
                r#"
            <!doctype html>
            <html lang='en'>
            <head>
                <title>ReqMan - Import Results</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/reqman.css'>
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
                                        <a href="/import_excel" class="btn btn-outline-primary">Import Another File</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#,
                import_result.imported_count, mapping_data.import_type, name, project_id
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
                <title>ReqMan - Import Error</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/reqman.css'>
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
                                        <a href="/import_excel" class="btn btn-primary">Try Again</a>
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
                e, name, project_id
            )
        }
    };

    Ok(content::RawHtml(html))
}
