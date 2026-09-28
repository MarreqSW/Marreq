// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::generators::GeneratorError as WorkbookError;
use crate::helper_functions::decorators;
use crate::repository::{DieselRepo, Repository};
use rust_xlsxwriter::Workbook;
use std::collections::{HashMap, HashSet};

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

    let mut workbook = Workbook::new();
    let sheet1 = workbook.add_worksheet();

    // Write headers
    // First column headers (requirement info)
    sheet1.write_string(0, 0, "Title")?;
    sheet1.write_string(0, 1, "Reference")?;
    sheet1.write_string(0, 2, "Category")?;
    sheet1.write_string(0, 3, "Status")?;

    // Test headers starting from column 4
    for (col_idx, test) in decorated_tests.iter().enumerate() {
        let col = (col_idx + 4) as u16;
        let header = format!("Test #{} ({})", test.id, test.name);
        sheet1.write_string(0, col, &header)?;
    }

    // Write requirement rows
    for (row_idx, req) in decorated_reqs.iter().enumerate() {
        let row = (row_idx + 1) as u32;

        // Write requirement info
        sheet1.write_string(row, 0, &req.title)?;
        sheet1.write_string(row, 1, &req.reference_code)?;
        sheet1.write_string(row, 2, &req.category_id)?;
        sheet1.write_string(row, 3, &req.status_id)?;

        // Mark the cell when this requirement is linked to this test
        for (col_idx, test) in decorated_tests.iter().enumerate() {
            let col = (col_idx + 4) as u16;
            if linked.contains(&(req.id, test.id)) {
                sheet1.write_string(row, col, "Yes")?;
            }
            // Leave cell empty if no link exists
        }
    }

    Ok(workbook.save_to_buffer()?)
}

/// Two-column workbook (requirement_code, verification_code) matching matrix-links import.
pub fn matrix_links_workbook_with_repo<R: Repository>(
    repo: &R,
    project_id: i32,
) -> Result<Vec<u8>, WorkbookError> {
    let reqs = repo
        .get_requirements_by_project(project_id)
        .map_err(|e| format!("Error querying requirements by project: {:?}", e))?;
    let vers = repo
        .get_verifications_by_project(project_id)
        .map_err(|e| format!("Error querying tests by project: {:?}", e))?;
    let links = repo
        .get_matrix_by_project(project_id)
        .map_err(|e| format!("Error querying matrix links: {:?}", e))?;

    let req_codes: HashMap<i32, String> =
        reqs.into_iter().map(|r| (r.id, r.reference_code)).collect();
    let ver_codes: HashMap<i32, String> =
        vers.into_iter().map(|v| (v.id, v.reference_code)).collect();

    let mut rows: Vec<(String, String)> = Vec::new();
    for link in links {
        let Some(req_code) = req_codes.get(&link.req_id) else {
            continue;
        };
        let Some(ver_code) = ver_codes.get(&link.verification_id) else {
            continue;
        };
        if req_code.trim().is_empty() || ver_code.trim().is_empty() {
            continue;
        }
        rows.push((req_code.clone(), ver_code.clone()));
    }
    rows.sort();

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.write_string(0, 0, "requirement_code")?;
    sheet.write_string(0, 1, "verification_code")?;
    for (idx, (req_code, ver_code)) in rows.iter().enumerate() {
        let row = (idx + 1) as u32;
        sheet.write_string(row, 0, req_code)?;
        sheet.write_string(row, 1, ver_code)?;
    }
    Ok(workbook.save_to_buffer()?)
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

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Requirements")?;

    let base_cols = 14u16;
    // Headers: standard columns then custom field labels
    worksheet.write_string(0, 0, "ID")?;
    worksheet.write_string(0, 1, "Title")?;
    worksheet.write_string(0, 2, "Description")?;
    worksheet.write_string(0, 3, "Reference")?;
    worksheet.write_string(0, 4, "Category")?;
    worksheet.write_string(0, 5, "Applicability")?;
    worksheet.write_string(0, 6, "Status")?;
    worksheet.write_string(0, 7, "Verification")?;
    worksheet.write_string(0, 8, "Author")?;
    worksheet.write_string(0, 9, "Reviewer")?;
    worksheet.write_string(0, 10, "Creation Date")?;
    worksheet.write_string(0, 11, "Update Date")?;
    worksheet.write_string(0, 12, "Deadline Date")?;
    worksheet.write_string(0, 13, "Justification")?;
    for (col_off, def) in custom_defs.iter().enumerate() {
        worksheet.write_string(0, base_cols + col_off as u16, &def.label)?;
    }

    for (i, req) in decorated_requirements.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, req.id as f64)?;
        worksheet.write_string(row, 1, &req.title)?;
        worksheet.write_string(row, 2, &req.description)?;
        worksheet.write_string(row, 3, &req.reference_code)?;
        worksheet.write_string(row, 4, &req.category_id)?;
        worksheet.write_string(row, 5, &req.applicability_id)?;
        worksheet.write_string(row, 6, &req.status_id)?;
        worksheet.write_string(row, 7, &req.verification_method_id)?;
        worksheet.write_string(row, 8, &req.author_id)?;
        worksheet.write_string(row, 9, &req.reviewer_id)?;
        worksheet.write_string(row, 10, &req.creation_date)?;
        worksheet.write_string(row, 11, &req.update_date)?;
        worksheet.write_string(row, 12, &req.deadline_date)?;
        worksheet.write_string(row, 13, req.justification.as_deref().unwrap_or(""))?;

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
                worksheet.write_string(row, base_cols + col_off as u16, &val)?;
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
    let comments_sheet = workbook.add_worksheet().set_name("Comments")?;
    comments_sheet.write_string(0, 0, "Requirement ID")?;
    comments_sheet.write_string(0, 1, "Version ID")?;
    comments_sheet.write_string(0, 2, "Author")?;
    comments_sheet.write_string(0, 3, "Created At")?;
    comments_sheet.write_string(0, 4, "Body")?;
    for (i, (c, author_name)) in all_comments.iter().enumerate() {
        let row = (i + 1) as u32;
        comments_sheet.write_number(row, 0, c.requirement_id as f64)?;
        comments_sheet.write_string(
            row,
            1,
            c.requirement_version_id
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
        )?;
        comments_sheet.write_string(row, 2, author_name)?;
        comments_sheet.write_string(row, 3, c.created_at.format("%Y-%m-%d %H:%M").to_string())?;
        comments_sheet.write_string(row, 4, &c.body)?;
    }

    Ok(workbook.save_to_buffer()?)
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

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Tests")?;

    // Write headers
    worksheet.write_string(0, 0, "ID")?;
    worksheet.write_string(0, 1, "Name")?;
    worksheet.write_string(0, 2, "Description")?;
    worksheet.write_string(0, 3, "Source")?;
    worksheet.write_string(0, 4, "Reference")?;
    worksheet.write_string(0, 5, "Status")?;
    worksheet.write_string(0, 6, "Parent")?;

    // Write data
    for (i, test) in decorated_tests.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, test.id as f64)?;
        worksheet.write_string(row, 1, &test.name)?;
        worksheet.write_string(row, 2, &test.description)?;
        worksheet.write_string(row, 3, &test.source)?;
        worksheet.write_string(row, 4, &test.reference_code)?;
        worksheet.write_string(row, 5, &test.status_id)?;
        worksheet.write_string(row, 6, &test.verification_parent_title)?;
    }

    Ok(workbook.save_to_buffer()?)
}

/// Short marker for a link type in DSM cells.
fn dsm_link_letter(link_type: &str) -> &'static str {
    match link_type {
        "DERIVES_FROM" => "D",
        "REFINES" => "R",
        "DEPENDS_ON" => "P",
        "SATISFIES" => "S",
        "RELATES_TO" => "~",
        _ => "?",
    }
}

/// Dependency Structure Matrix workbook (issue #328): sheet "DSM" with the
/// ordered matrix (row depends on column), plus "Loops" and "Legend".
pub fn dsm_workbook(dsm: &crate::services::dsm_service::Dsm) -> Result<Vec<u8>, WorkbookError> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

    const FIRST_COL: u16 = 4;
    let header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE2E5ED));
    let col_header = header
        .clone()
        .set_rotation(90)
        .set_align(FormatAlign::Center)
        .set_font_color(Color::RGB(0x1A237E));
    let diagonal = Format::new().set_background_color(Color::RGB(0x3C4043));
    let mark = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_font_color(Color::RGB(0x1A237E));
    let loop_mark = mark.clone().set_background_color(Color::RGB(0xFDE8C4));
    let changed = mark
        .clone()
        .set_border(FormatBorder::Medium)
        .set_border_color(Color::RGB(0xC5221F));
    let changed_loop = changed.clone().set_background_color(Color::RGB(0xFDE8C4));
    let group_band = Format::new().set_background_color(Color::RGB(0xF3F4F8));

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("DSM")?;
    sheet.write_string_with_format(0, 0, "#", &header)?;
    sheet.write_string_with_format(0, 1, "Reference", &header)?;
    sheet.write_string_with_format(0, 2, "Title", &header)?;
    sheet.write_string_with_format(0, 3, "Group", &header)?;
    sheet.set_row_height(0, 90)?;
    sheet.set_column_width(0, 5)?;
    sheet.set_column_width(1, 16)?;
    sheet.set_column_width(2, 40)?;
    sheet.set_column_width(3, 16)?;

    let group_of = |index: usize| {
        dsm.groups
            .iter()
            .position(|g| (g.start..=g.end).contains(&index))
    };
    for r in &dsm.requirements {
        let row = r.index as u32 + 1;
        let col = FIRST_COL + r.index as u16;
        sheet.write_string_with_format(0, col, &r.reference_code, &col_header)?;
        sheet.set_column_width(col, 3.5)?;
        // Alternate bands in the row header make category blocks visible.
        let band = group_of(r.index).is_some_and(|g| g % 2 == 1);
        let label_format = if band { &group_band } else { &Format::new() };
        sheet.write_number_with_format(row, 0, (r.index + 1) as f64, label_format)?;
        sheet.write_string_with_format(row, 1, &r.reference_code, label_format)?;
        sheet.write_string_with_format(row, 2, &r.title, label_format)?;
        let group = group_of(r.index)
            .map(|g| dsm.groups[g].label.clone())
            .unwrap_or_else(|| r.category.clone());
        sheet.write_string_with_format(row, 3, &group, label_format)?;
        sheet.write_blank(row, col, &diagonal)?;
    }
    for cell in &dsm.cells {
        let text: String = cell.link_types.iter().map(|t| dsm_link_letter(t)).collect();
        let format = match (cell.upstream_changed, cell.in_loop) {
            (true, true) => &changed_loop,
            (true, false) => &changed,
            (false, true) => &loop_mark,
            (false, false) => &mark,
        };
        sheet.write_string_with_format(
            cell.row as u32 + 1,
            FIRST_COL + cell.col as u16,
            &text,
            format,
        )?;
    }
    sheet.set_freeze_panes(1, FIRST_COL)?;

    let code_of = |id: i32| {
        dsm.requirements
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.reference_code.clone())
            .unwrap_or_else(|| format!("#{id}"))
    };
    let loops = workbook.add_worksheet().set_name("Loops")?;
    loops.write_string_with_format(0, 0, "Loop", &header)?;
    loops.write_string_with_format(0, 1, "Requirements", &header)?;
    loops.write_string_with_format(0, 2, "Cycle", &header)?;
    loops.set_column_width(1, 14)?;
    loops.set_column_width(2, 80)?;
    for (i, l) in dsm.loops.iter().enumerate() {
        let row = i as u32 + 1;
        loops.write_number(row, 0, (i + 1) as f64)?;
        loops.write_number(row, 1, l.requirement_ids.len() as f64)?;
        let mut cycle: Vec<String> = l.path.iter().map(|id| code_of(*id)).collect();
        if let Some(first) = cycle.first().cloned() {
            cycle.push(first);
        }
        loops.write_string(row, 2, cycle.join(" → "))?;
    }

    let legend = workbook.add_worksheet().set_name("Legend")?;
    legend.set_column_width(0, 14)?;
    legend.set_column_width(1, 70)?;
    let rows: [(&str, &str); 9] = [
        (
            "Row / column",
            "A mark in row i, column j: requirement i's current version links to requirement j.",
        ),
        ("D", "Derives from"),
        ("R", "Refines"),
        ("P", "Depends on"),
        ("S", "Satisfies"),
        ("~", "Relates to"),
        ("Dark cell", "Diagonal (the requirement itself)"),
        (
            "Amber cell",
            "Both requirements are in the same dependency loop (see Loops)",
        ),
        (
            "Red border",
            "Upstream changed: the target was edited after the source was approved",
        ),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        legend.write_string_with_format(i as u32, 0, *k, &header)?;
        legend.write_string(i as u32, 1, *v)?;
    }
    legend.write_string(rows.len() as u32 + 1, 0, "Order")?;
    legend.write_string(rows.len() as u32 + 1, 1, &dsm.order)?;
    legend.write_string(rows.len() as u32 + 2, 0, "Link types")?;
    legend.write_string(rows.len() as u32 + 2, 1, dsm.link_types.join(", "))?;

    Ok(workbook.save_to_buffer()?)
}

#[cfg(test)]
mod workbook_tests {
    use super::{
        matrix_links_workbook_with_repo, requirements_workbook_with_repo,
        verifications_workbook_with_repo,
    };
    use crate::importers::excel::ExcelImporter;
    use crate::models::{MatrixLink, Requirement, Verification};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use calamine::{Data, Reader, open_workbook_auto_from_rs};
    use chrono::{NaiveDate, NaiveDateTime};
    use std::io::Cursor;

    fn ts() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn req(id: i32, code: &str) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: format!("Req {id}"),
            description: String::new(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: code.into(),
            category_id: 1,
            parent_id: None,
            creation_date: ts(),
            update_date: ts(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: 1,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    fn ver(id: i32, code: &str) -> Verification {
        Verification {
            id,
            name: format!("Ver {id}"),
            reference_code: code.into(),
            description: String::new(),
            source: String::new(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        }
    }

    fn link(req_id: i32, verification_id: i32) -> MatrixLink {
        MatrixLink {
            req_id,
            verification_id,
            creation_date: ts(),
            project_id: 1,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        }
    }

    fn cell_text(cell: &Data) -> String {
        match cell {
            Data::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    #[test]
    fn matrix_links_workbook_two_code_columns() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, req(1, "REQ-B"));
        repo.requirements.insert(2, req(2, "REQ-A"));
        repo.requirements.insert(3, req(3, ""));
        repo.verifications.insert(10, ver(10, "TST-2"));
        repo.verifications.insert(11, ver(11, "TST-1"));
        repo.verifications.insert(12, ver(12, "   "));
        repo.matrices.push(link(2, 11));
        repo.matrices.push(link(1, 10));
        repo.matrices.push(link(3, 11));
        repo.matrices.push(link(2, 12));

        let bytes = matrix_links_workbook_with_repo(&repo, 1).expect("workbook");
        let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("open xlsx");
        let sheet = workbook.sheet_names().first().cloned().expect("sheet");
        let range = workbook.worksheet_range(&sheet).expect("range");
        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|row| row.iter().take(2).map(cell_text).collect())
            .filter(|row: &Vec<String>| row.iter().any(|c| !c.trim().is_empty()))
            .collect();
        let expected: Vec<Vec<String>> = vec![
            vec!["requirement_code".into(), "verification_code".into()],
            vec!["REQ-A".into(), "TST-1".into()],
            vec!["REQ-B".into(), "TST-2".into()],
        ];
        assert_eq!(rows, expected);
    }

    fn imported(bytes: Vec<u8>) -> ExcelImporter {
        ExcelImporter::from_bytes("export.xlsx", &bytes).expect("importer reads export")
    }

    fn column_names(importer: &ExcelImporter) -> Vec<String> {
        importer.columns.iter().map(|c| c.name.clone()).collect()
    }

    #[test]
    fn requirements_workbook_round_trips_through_importer() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, req(1, "REQ-1"));
        repo.requirements.insert(2, req(2, "REQ-2"));

        let importer = imported(requirements_workbook_with_repo(&repo, 1).expect("workbook"));
        assert_eq!(
            column_names(&importer),
            [
                "ID",
                "Title",
                "Description",
                "Reference",
                "Category",
                "Applicability",
                "Status",
                "Verification",
                "Author",
                "Reviewer",
                "Creation Date",
                "Update Date",
                "Deadline Date",
                "Justification",
            ]
        );
        assert_eq!(importer.import_type, "requirements");
        let mut rows: Vec<(String, String)> = importer
            .data
            .iter()
            .map(|row| (row[1].clone(), row[3].clone()))
            .collect();
        rows.sort();
        assert_eq!(
            rows,
            [
                ("Req 1".to_string(), "REQ-1".to_string()),
                ("Req 2".to_string(), "REQ-2".to_string()),
            ]
        );
    }

    #[test]
    fn verifications_workbook_round_trips_through_importer() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(10, ver(10, "TST-1"));

        let importer = imported(verifications_workbook_with_repo(&repo, 1).expect("workbook"));
        assert_eq!(
            column_names(&importer),
            [
                "ID",
                "Name",
                "Description",
                "Source",
                "Reference",
                "Status",
                "Parent"
            ]
        );
        assert_eq!(importer.data.len(), 1);
        assert_eq!(importer.data[0][1], "Ver 10");
        assert_eq!(importer.data[0][4], "TST-1");
    }

    #[test]
    fn matrix_links_workbook_round_trips_through_importer() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, req(1, "REQ-A"));
        repo.verifications.insert(10, ver(10, "TST-1"));
        repo.matrices.push(link(1, 10));

        let importer = imported(matrix_links_workbook_with_repo(&repo, 1).expect("workbook"));
        assert_eq!(importer.import_type, "matrix");
        assert_eq!(importer.data, [["REQ-A".to_string(), "TST-1".to_string()]]);
    }
}
