use crate::helper_functions::decorators;
use crate::models::*;
use crate::repository::DieselRepo;
use diesel::prelude::*;
use std::fs;

pub fn create_matrix_workbook(
    cookies: &rocket::http::CookieJar<'_>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("Creating matrix workbook");

    use crate::helper_functions::*;
    use crate::schema::matrix::dsl::{matrix, req_id};
    use crate::schema::requirements::dsl::requirements;
    use crate::schema::tests::dsl::tests;

    let mut connection = DieselRepo::new()
        .get_conn()
        .map_err(|e| format!("Database connection error: {}", e))?;

    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);

    // Get requirements for the selected project
    let all_reqs = if let Some(selected_pid) = selected_project_id {
        requirements
            .filter(crate::schema::requirements::project_id.eq(selected_pid))
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| format!("Error querying requirements by project: {:?}", e))?
    } else {
        requirements
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| format!("Error querying requirements: {:?}", e))?
    };

    // Get tests for the selected project
    let all_tests = if let Some(selected_pid) = selected_project_id {
        tests
            .filter(crate::schema::tests::project_id.eq(selected_pid))
            .load::<TestCase>(connection.as_mut())
            .map_err(|e| format!("Error querying tests by project: {:?}", e))?
    } else {
        tests
            .load::<TestCase>(connection.as_mut())
            .map_err(|e| format!("Error querying tests: {:?}", e))?
    };

    eprintln!(
        "Found {} requirements and {} tests",
        all_reqs.len(),
        all_tests.len()
    );

    // Decorate requirements and tests to get real names
    let mut decorated_reqs = decorators::decorate_requirements(all_reqs);
    let mut decorated_tests = decorators::decorate_tests(all_tests);

    // Sort requirements by ID
    decorated_reqs.sort_by(|a, b| a.id.cmp(&b.id));

    // Sort tests by ID
    decorated_tests.sort_by(|a, b| a.id.cmp(&b.id));

    let workbook = xlsxwriter::Workbook::new("target/matrix.xls")?;
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

        // Check matrix links for each test
        for (col_idx, test) in decorated_tests.iter().enumerate() {
            let col = (col_idx + 4) as u16;

            // Check if this requirement is linked to this test
            let test_present: i64 = matrix
                .filter(req_id.eq(req.id))
                .filter(crate::schema::matrix::dsl::test_id.eq(test.id))
                .count()
                .get_result(connection.as_mut())
                .map_err(|e| format!("Error checking matrix link: {:?}", e))?;

            if test_present > 0 {
                sheet1.write_string(row, col, "Yes", None)?;
            }
            // Leave cell empty if no link exists
        }
    }

    eprintln!("Matrix data written successfully");

    workbook
        .close()
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Error closing workbook: {:?}", e).into()
        })?;

    let result =
        fs::read("target/matrix.xls").map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Error reading generated file: {:?}", e).into()
        })?;

    eprintln!("Matrix workbook created successfully");
    Ok(result)
}

pub fn create_requirements_workbook() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use crate::schema::requirements::dsl::*;

    let mut connection = DieselRepo::new().get_conn()?;

    let all_requirements = requirements
        .load::<Requirement>(connection.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Decorate requirements to get real names instead of IDs
    let decorated_requirements = decorators::decorate_requirements(all_requirements);

    // Create workbook
    let workbook = xlsxwriter::Workbook::new("target/requirements.xls")?;
    let mut worksheet = workbook.add_worksheet(Some("Requirements"))?;

    // Write headers
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

    // Write data
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
        worksheet.write_string(row, 13, &req.justification.as_deref().unwrap_or(""), None)?;
    }

    workbook.close()?;
    let result = fs::read("target/requirements.xls").expect("can read file");
    Ok(result)
}

pub fn create_tests_workbook() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use crate::schema::tests::dsl::*;

    let mut connection = DieselRepo::new().get_conn()?;

    let all_tests = tests
        .load::<TestCase>(connection.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Decorate tests to get real names instead of IDs
    let decorated_tests = decorators::decorate_tests(all_tests);

    // Create workbook
    let workbook = xlsxwriter::Workbook::new("target/tests.xls")?;
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
        worksheet.write_string(row, 6, &test.test_parent_title, None)?;
    }

    workbook.close()?;
    let result = fs::read("target/tests.xls").expect("can read file");
    Ok(result)
}
