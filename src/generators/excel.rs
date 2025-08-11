use crate::models::*;
use crate::schema::*;
use diesel::prelude::*;
use crate::db::get_connection_pooled_safe;
use std::fs;

pub fn create_matrix_workbook(cookies: &rocket::http::CookieJar<'_>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("Creating matrix workbook");
    
    use crate::schema::matrix::dsl::*;
    use crate::schema::requirements::dsl::*;
    use crate::schema::tests::dsl::*;
    use crate::helper_functions::*;
    
    let connection = &mut get_connection_pooled_safe();
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    // Get requirements for the selected project
    let mut all_reqs = if let Some(selected_pid) = selected_project_id {
        requirements
            .filter(crate::schema::requirements::project_id.eq(selected_pid))
            .load::<Requirement>(connection)
            .map_err(|e| format!("Error querying requirements by project: {:?}", e))?
    } else {
        requirements
            .load::<Requirement>(connection)
            .map_err(|e| format!("Error querying requirements: {:?}", e))?
    };

    // Get tests for the selected project
    let mut all_tests = if let Some(selected_pid) = selected_project_id {
        tests
            .filter(crate::schema::tests::project_id.eq(selected_pid))
            .load::<Test>(connection)
            .map_err(|e| format!("Error querying tests by project: {:?}", e))?
    } else {
        tests
            .load::<Test>(connection)
            .map_err(|e| format!("Error querying tests: {:?}", e))?
    };

    // Sort requirements by ID
    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
    
    // Sort tests by ID
    all_tests.sort_by(|a, b| a.test_id.cmp(&b.test_id));
    
    eprintln!("Found {} requirements and {} tests", all_reqs.len(), all_tests.len());
    
    let workbook = xlsxwriter::Workbook::new("target/matrix.xls")?;
    let mut sheet1 = workbook.add_worksheet(None)?;
    
    // Write headers
    // First column headers (requirement info)
    sheet1.write_string(0, 0, "Req ID", None)?;
    sheet1.write_string(0, 1, "Title", None)?;
    sheet1.write_string(0, 2, "Reference", None)?;
    
    // Test headers starting from column 3
    for (col_idx, test) in all_tests.iter().enumerate() {
        let col = (col_idx + 3) as u16;
        let header = format!("Test #{} ({})", test.test_id, test.test_name);
        sheet1.write_string(0, col, &header, None)?;
    }
    
    // Write requirement rows
    for (row_idx, req) in all_reqs.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        
        // Write requirement info
        sheet1.write_number(row, 0, req.req_id as f64, None)?;
        sheet1.write_string(row, 1, &req.req_title, None)?;
        sheet1.write_string(row, 2, &req.req_reference, None)?;
        
        // Check matrix links for each test
        for (col_idx, test) in all_tests.iter().enumerate() {
            let col = (col_idx + 3) as u16;
            
            // Check if this requirement is linked to this test
            let test_present: i64 = matrix
                .filter(matrix_req_id.eq(req.req_id))
                .filter(matrix_test_id.eq(test.test_id))
                .count()
                .get_result(connection)
                .map_err(|e| format!("Error checking matrix link: {:?}", e))?;
            
            if test_present > 0 {
                sheet1.write_string(row, col, "Yes", None)?;
            }
            // Leave cell empty if no link exists
        }
    }
    
    eprintln!("Matrix data written successfully");
    
    workbook.close().map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("Error closing workbook: {:?}", e).into()
    })?;
    
    let result = fs::read("target/matrix.xls").map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("Error reading generated file: {:?}", e).into()
    })?;
    
    eprintln!("Matrix workbook created successfully");
    Ok(result)
}

pub fn create_requirements_workbook() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use crate::schema::requirements::dsl::*;
    use crate::schema::categories::dsl::*;
    use crate::schema::applicability::dsl::*;
    use crate::schema::status::dsl::*;
    use crate::schema::verification::dsl::*;
    use crate::schema::users::dsl::*;

    let mut connection = get_connection_pooled_safe()?;

    let all_requirements = requirements
        .load::<Requirement>(connection.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Create workbook
    let workbook = xlsxwriter::Workbook::new("target/requirements.xls");
    let worksheet = workbook.add_worksheet(Some("Requirements"));

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
    worksheet.write_string(0, 9, "Link", None)?;
    worksheet.write_string(0, 10, "Creation Date", None)?;
    worksheet.write_string(0, 11, "Update Date", None)?;
    worksheet.write_string(0, 12, "Deadline Date", None)?;

    // Write data
    for (i, req) in all_requirements.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, req.req_id as f64, None)?;
        worksheet.write_string(row, 1, &req.req_title, None)?;
        worksheet.write_string(row, 2, &req.req_description, None)?;
        worksheet.write_string(row, 3, &req.req_reference, None)?;
        worksheet.write_string(row, 4, &req.req_category.to_string(), None)?;
        worksheet.write_string(row, 5, &req.req_applicability.to_string(), None)?;
        worksheet.write_string(row, 6, &req.req_current_status.to_string(), None)?;
        worksheet.write_string(row, 7, &req.req_verification.to_string(), None)?;
        worksheet.write_string(row, 8, &req.req_author.to_string(), None)?;
        worksheet.write_string(row, 9, &req.req_link, None)?;
        worksheet.write_string(row, 10, &req.req_creation_date.to_string(), None)?;
        worksheet.write_string(row, 11, &req.req_update_date.to_string(), None)?;
        worksheet.write_string(row, 12, &req.req_deadline_date.to_string(), None)?;
    }

    workbook.close().expect("workbook can be closed");
    let result = fs::read("target/requirements.xls").expect("can read file");
    Ok(result)
}

pub fn create_tests_workbook() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use crate::schema::tests::dsl::*;

    let mut connection = get_connection_pooled_safe()?;

    let all_tests = tests
        .load::<Test>(connection.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Create workbook
    let workbook = xlsxwriter::Workbook::new("target/tests.xls");
    let worksheet = workbook.add_worksheet(Some("Tests"));

    // Write headers
    worksheet.write_string(0, 0, "ID", None)?;
    worksheet.write_string(0, 1, "Name", None)?;
    worksheet.write_string(0, 2, "Description", None)?;
    worksheet.write_string(0, 3, "Source", None)?;
    worksheet.write_string(0, 4, "Status", None)?;
    worksheet.write_string(0, 5, "Parent", None)?;

    // Write data
    for (i, test) in all_tests.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, test.test_id as f64, None)?;
        worksheet.write_string(row, 1, &test.test_name, None)?;
        worksheet.write_string(row, 2, &test.test_description, None)?;
        worksheet.write_string(row, 3, &test.test_source, None)?;
        worksheet.write_string(row, 4, &test.test_status.to_string(), None)?;
        worksheet.write_number(row, 5, test.test_parent as f64, None)?;
    }

    workbook.close().expect("workbook can be closed");
    let result = fs::read("target/tests.xls").expect("can read file");
    Ok(result)
}