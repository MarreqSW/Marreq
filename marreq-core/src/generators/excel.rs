// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::generators::GeneratorError as WorkbookError;
use crate::helper_functions::decorators;
use crate::repository::{DieselRepo, Repository};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Scratch file for xlsxwriter, which can only write to a path. The name is unique per
/// process and call so concurrent exports of the same project cannot clobber each other,
/// and the file is removed even when workbook generation fails part way through.
struct TempWorkbook {
    path: PathBuf,
}

impl TempWorkbook {
    fn new(kind: &str, project_id: i32) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "marreq-{kind}-{project_id}-{}-{unique}.xlsx",
            std::process::id()
        ));
        Self { path }
    }

    fn path_str(&self) -> Result<&str, WorkbookError> {
        self.path.to_str().ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid temp path",
            )) as WorkbookError
        })
    }

    fn read(&self) -> Result<Vec<u8>, WorkbookError> {
        Ok(fs::read(&self.path)?)
    }
}

impl Drop for TempWorkbook {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn create_matrix_workbook(
    project_id: i32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let repo = DieselRepo::new().map_err(|e| format!("Database connection error: {}", e))?;
    matrix_workbook_with_repo(&repo, project_id)
}

/// Build the traceability matrix workbook using an explicitly provided repository.
pub fn matrix_workbook_with_repo<R: Repository>(
    repo: &R,
    project_id: i32,
) -> Result<Vec<u8>, WorkbookError> {
    let all_reqs = repo
        .get_requirements_by_project(project_id)
        .map_err(|e| format!("Error querying requirements by project: {:?}", e))?;

    let all_tests = repo
        .get_verifications_by_project(project_id)
        .map_err(|e| format!("Error querying tests by project: {:?}", e))?;

    let links = repo
        .get_matrix_by_project(project_id)
        .map_err(|e| format!("Error querying matrix links: {:?}", e))?;
    let linked: HashSet<(i32, i32)> = links
        .iter()
        .map(|link| (link.req_id, link.verification_id))
        .collect();

    // Decorate requirements and tests to get real names
    let mut decorated_reqs = decorators::decorate_requirements_with_repo(repo, all_reqs);
    let mut decorated_tests = decorators::decorate_verifications_with_repo(repo, all_tests);

    // Sort requirements by ID
    decorated_reqs.sort_by_key(|req| req.id);

    // Sort tests by ID
    decorated_tests.sort_by_key(|test| test.id);

    let temp = TempWorkbook::new("matrix", project_id);
    let workbook = xlsxwriter::Workbook::new(temp.path_str()?)?;
    let mut sheet1 = workbook.add_worksheet(None)?;

    // Write headers
    // First column headers (requirement info)
    sheet1.write_string(0, 0, "Title", None)?;
    sheet1.write_string(0, 1, "Reference", None)?;
    sheet1.write_string(0, 2, "Category", None)?;
    sheet1.write_string(0, 3, "Status", None)?;

    // Test headers starting from column 4
    for (col_idx, test) in decorated_tests.iter().enumerate() {
        let col = (col_idx + 4) as u16;
        let header = format!("Test #{} ({})", test.id, test.name);
        sheet1.write_string(0, col, &header, None)?;
    }

    // Write requirement rows
    for (row_idx, req) in decorated_reqs.iter().enumerate() {
        let row = (row_idx + 1) as u32;

        // Write requirement info
        sheet1.write_string(row, 0, &req.title, None)?;
        sheet1.write_string(row, 1, &req.reference_code, None)?;
        sheet1.write_string(row, 2, &req.category_id, None)?;
        sheet1.write_string(row, 3, &req.status_id, None)?;

        // Mark the cell when this requirement is linked to this test
        for (col_idx, test) in decorated_tests.iter().enumerate() {
            let col = (col_idx + 4) as u16;
            if linked.contains(&(req.id, test.id)) {
                sheet1.write_string(row, col, "Yes", None)?;
            }
            // Leave cell empty if no link exists
        }
    }

    workbook.close()?;
    temp.read()
}

pub fn create_requirements_workbook(pid: i32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let repo = DieselRepo::new()?;
    requirements_workbook_with_repo(&repo, pid).map_err(|e| -> Box<dyn std::error::Error> { e })
}

/// Build the requirements workbook using an explicitly provided repository.
pub fn requirements_workbook_with_repo<R: Repository>(
    repo: &R,
    pid: i32,
) -> Result<Vec<u8>, WorkbookError> {
    let all_requirements = repo
        .get_requirements_by_project(pid)
        .map_err(|e| format!("Error querying requirements by project: {:?}", e))?;

    let custom_defs = repo
        .list_custom_field_definitions_by_project(pid)
        .map_err(|e| format!("Error querying custom field definitions: {:?}", e))?;

    // Decorate requirements to get real names instead of IDs
    let decorated_requirements =
        decorators::decorate_requirements_with_repo(repo, all_requirements.clone());

    let temp = TempWorkbook::new("requirements", pid);
    let workbook = xlsxwriter::Workbook::new(temp.path_str()?)?;
    let mut worksheet = workbook.add_worksheet(Some("Requirements"))?;

    let base_cols = 14u16;
    // Headers: standard columns then custom field labels
    worksheet.write_string(0, 0, "ID", None)?;
    worksheet.write_string(0, 1, "Title", None)?;
    worksheet.write_string(0, 2, "Description", None)?;
    worksheet.write_string(0, 3, "Reference", None)?;
    worksheet.write_string(0, 4, "Category", None)?;
    worksheet.write_string(0, 5, "Applicability", None)?;
    worksheet.write_string(0, 6, "Status", None)?;
    worksheet.write_string(0, 7, "Verification", None)?;
    worksheet.write_string(0, 8, "Author", None)?;
    worksheet.write_string(0, 9, "Reviewer", None)?;
    worksheet.write_string(0, 10, "Creation Date", None)?;
    worksheet.write_string(0, 11, "Update Date", None)?;
    worksheet.write_string(0, 12, "Deadline Date", None)?;
    worksheet.write_string(0, 13, "Justification", None)?;
    for (col_off, def) in custom_defs.iter().enumerate() {
        worksheet.write_string(0, base_cols + col_off as u16, &def.label, None)?;
    }

    for (i, req) in decorated_requirements.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, req.id as f64, None)?;
        worksheet.write_string(row, 1, &req.title, None)?;
        worksheet.write_string(row, 2, &req.description, None)?;
        worksheet.write_string(row, 3, &req.reference_code, None)?;
        worksheet.write_string(row, 4, &req.category_id, None)?;
        worksheet.write_string(row, 5, &req.applicability_id, None)?;
        worksheet.write_string(row, 6, &req.status_id, None)?;
        worksheet.write_string(row, 7, &req.verification_method_id, None)?;
        worksheet.write_string(row, 8, &req.author_id, None)?;
        worksheet.write_string(row, 9, &req.reviewer_id, None)?;
        worksheet.write_string(row, 10, &req.creation_date, None)?;
        worksheet.write_string(row, 11, &req.update_date, None)?;
        worksheet.write_string(row, 12, &req.deadline_date, None)?;
        worksheet.write_string(row, 13, req.justification.as_deref().unwrap_or(""), None)?;

        let raw_req = &all_requirements[i];
        if let Some(version_id) = raw_req.current_version_id {
            let values = repo
                .get_custom_field_values_for_version(version_id)
                .unwrap_or_default();
            let value_map: std::collections::HashMap<i32, String> = values
                .into_iter()
                .map(|v| (v.field_id, v.value.unwrap_or_default()))
                .collect();
            for (col_off, def) in custom_defs.iter().enumerate() {
                let val = value_map.get(&def.id).cloned().unwrap_or_default();
                worksheet.write_string(row, base_cols + col_off as u16, &val, None)?;
            }
        }
    }

    // Comments sheet: requirement_id, version_id, author, created_at, body
    let all_req_ids: Vec<i32> = all_requirements.iter().map(|r| r.id).collect();
    let mut all_comments: Vec<(crate::models::RequirementComment, String)> = Vec::new();
    for req_id in &all_req_ids {
        let comments = repo
            .list_comments_by_requirement(*req_id, None)
            .unwrap_or_default();
        for c in comments {
            let author_name = repo
                .get_user_by_id(c.author_id)
                .ok()
                .map(|u| u.name)
                .unwrap_or_else(|| format!("User#{}", c.author_id));
            all_comments.push((c, author_name));
        }
    }
    all_comments.sort_by_key(|a| a.0.created_at);
    let mut comments_sheet = workbook.add_worksheet(Some("Comments"))?;
    comments_sheet.write_string(0, 0, "Requirement ID", None)?;
    comments_sheet.write_string(0, 1, "Version ID", None)?;
    comments_sheet.write_string(0, 2, "Author", None)?;
    comments_sheet.write_string(0, 3, "Created At", None)?;
    comments_sheet.write_string(0, 4, "Body", None)?;
    for (i, (c, author_name)) in all_comments.iter().enumerate() {
        let row = (i + 1) as u32;
        comments_sheet.write_number(row, 0, c.requirement_id as f64, None)?;
        comments_sheet.write_string(
            row,
            1,
            &c.requirement_version_id
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
            None,
        )?;
        comments_sheet.write_string(row, 2, author_name, None)?;
        comments_sheet.write_string(
            row,
            3,
            &c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            None,
        )?;
        comments_sheet.write_string(row, 4, &c.body, None)?;
    }

    workbook.close()?;
    temp.read()
}

pub fn create_tests_workbook(pid: i32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let repo = DieselRepo::new()?;
    verifications_workbook_with_repo(&repo, pid).map_err(|e| -> Box<dyn std::error::Error> { e })
}

/// Build the verifications workbook using an explicitly provided repository.
pub fn verifications_workbook_with_repo<R: Repository>(
    repo: &R,
    pid: i32,
) -> Result<Vec<u8>, WorkbookError> {
    let all_tests = repo
        .get_verifications_by_project(pid)
        .map_err(|e| format!("Error querying verifications by project: {:?}", e))?;

    // Decorate tests to get real names instead of IDs
    let decorated_tests = decorators::decorate_verifications_with_repo(repo, all_tests);

    let temp = TempWorkbook::new("verifications", pid);
    let workbook = xlsxwriter::Workbook::new(temp.path_str()?)?;
    let mut worksheet = workbook.add_worksheet(Some("Tests"))?;

    // Write headers
    worksheet.write_string(0, 0, "ID", None)?;
    worksheet.write_string(0, 1, "Name", None)?;
    worksheet.write_string(0, 2, "Description", None)?;
    worksheet.write_string(0, 3, "Source", None)?;
    worksheet.write_string(0, 4, "Reference", None)?;
    worksheet.write_string(0, 5, "Status", None)?;
    worksheet.write_string(0, 6, "Parent", None)?;

    // Write data
    for (i, test) in decorated_tests.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, test.id as f64, None)?;
        worksheet.write_string(row, 1, &test.name, None)?;
        worksheet.write_string(row, 2, &test.description, None)?;
        worksheet.write_string(row, 3, &test.source, None)?;
        worksheet.write_string(row, 4, &test.reference_code, None)?;
        worksheet.write_string(row, 5, &test.status_id, None)?;
        worksheet.write_string(row, 6, &test.verification_parent_title, None)?;
    }

    workbook.close()?;
    temp.read()
}
