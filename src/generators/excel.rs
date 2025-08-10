use diesel::prelude::*;

use crate::models::*;
use crate::helper_functions::*;
use crate::db::get_connection_pooled_safe;

use xlsxwriter::*;
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
    
    let workbook = Workbook::new("target/matrix.xls")?;
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

pub fn create_requirements_workbook() -> Result<Vec<u8>, xlsxwriter::XlsxError> {
    use crate::schema::requirements::dsl::*;
    
    let connection = &mut establish_connection();

    let workbook = Workbook::new("target/requirements.xls")?;
    let mut sheet1 = workbook.add_worksheet(None)?;
    
    // Get all requirements with decorated data
    let mut all_reqs = requirements
        .load::<Requirement>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying requirements: {:?}", _err);
            "Error querying requirements from the database".into()
        }).unwrap();

    // Sort requirements by ID
    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));

    let decorated_reqs = decorate_requirements(all_reqs);

    // Define headers
    let headers = vec![
        "Req ID", "Title", "Description", "Reference", "Category", "Applicability",
        "Status", "Verification", "Author", "Reviewer", "Parent", "Parent Title",
        "Link", "Creation Date", "Update Date", "Deadline Date", "Justification"
    ];

    // Write headers
    for (col, header) in headers.iter().enumerate() {
        sheet1.write_string(0, col as u16, header, None)?;
    }

    // Write data rows
    for (row, req) in decorated_reqs.iter().enumerate() {
        let row_num = (row + 1) as u32;
        let mut col = 0;

        sheet1.write_number(row_num, col, req.req_id as f64, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_title, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_description, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_reference, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_category, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_applicability, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_current_status, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_verification, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_author, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_reviewer, None)?;
        col += 1;
        
        if req.req_parent_id != 0 {
            sheet1.write_number(row_num, col, req.req_parent_id as f64, None)?;
        } else {
            sheet1.write_string(row_num, col, "None", None)?;
        }
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_parent_title, None)?;
        col += 1;
        
        if !req.req_link.is_empty() && req.req_link != " " {
            sheet1.write_string(row_num, col, &req.req_link, None)?;
        } else {
            sheet1.write_string(row_num, col, "None", None)?;
        }
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_creation_date, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_update_date, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &req.req_deadline_date, None)?;
        col += 1;
        
        if let Some(ref justification) = req.req_justification {
            sheet1.write_string(row_num, col, justification, None)?;
        } else {
            sheet1.write_string(row_num, col, "None", None)?;
        }
    }

    workbook.close().expect("workbook can be closed");
    let result = fs::read("target/requirements.xls").expect("can read file");
    Ok(result)
}

pub fn create_tests_workbook() -> Result<Vec<u8>, xlsxwriter::XlsxError> {
    use crate::schema::tests::dsl::*;
    
    let connection = &mut establish_connection();

    let workbook = Workbook::new("target/tests.xls")?;
    let mut sheet1 = workbook.add_worksheet(None)?;
    
    // Get all tests
    let mut all_tests = tests
        .load::<Test>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying tests: {:?}", _err);
            "Error querying tests from the database".into()
        }).unwrap();

    // Sort tests by ID
    all_tests.sort_by(|a, b| a.test_id.cmp(&b.test_id));

    // Define headers
    let headers = vec![
        "Test ID", "Name", "Description", "Status", "Source", "Parent ID", "Parent Name"
    ];

    // Write headers
    for (col, header) in headers.iter().enumerate() {
        sheet1.write_string(0, col as u16, header, None)?;
    }

    // Write data rows
    for (row, test) in all_tests.iter().enumerate() {
        let row_num = (row + 1) as u32;
        let mut col = 0;

        sheet1.write_number(row_num, col, test.test_id as f64, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &test.test_name, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &test.test_description, None)?;
        col += 1;
        
        // Get status name
        let status_name = get_status_name_by_id(test.test_status);
        sheet1.write_string(row_num, col, &status_name, None)?;
        col += 1;
        
        sheet1.write_string(row_num, col, &test.test_source, None)?;
        col += 1;
        
        if test.test_parent != 0 {
            sheet1.write_number(row_num, col, test.test_parent as f64, None)?;
        } else {
            sheet1.write_string(row_num, col, "None", None)?;
        }
        col += 1;
        
        // Get parent test name if exists
        if test.test_parent != 0 {
            let parent_test = tests
                .filter(test_id.eq(test.test_parent))
                .first::<Test>(connection)
                .ok();
            
            if let Some(parent) = parent_test {
                sheet1.write_string(row_num, col, &parent.test_name, None)?;
            } else {
                sheet1.write_string(row_num, col, "Unknown", None)?;
            }
        } else {
            sheet1.write_string(row_num, col, "None", None)?;
        }
    }

    workbook.close().expect("workbook can be closed");
    let result = fs::read("target/tests.xls").expect("can read file");
    Ok(result)
}