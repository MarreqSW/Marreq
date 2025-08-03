use diesel::prelude::*;

use crate::models::*;
use crate::helper_functions::*;

use xlsxwriter::*;
use std::fs;

pub fn create_matrix_workbook()->Result<Vec<u8>,xlsxwriter::XlsxError> {
    use crate::schema::requirements::dsl::*;
    use crate::schema::matrix::dsl::*;
    use crate::schema::tests::dsl::*;
    
    let connection = &mut establish_connection();

    let workbook = Workbook::new("target/matrix.xls")?;
    let mut sheet1 = workbook.add_worksheet(None)?;
    
    let mut all_reqs = requirements
    .load::<Requirement>(connection)
    .map_err(|_err| -> String {
        #[cfg(debug_assertions)]
        println!("Error querying page views: {:?}", _err);
        "Error querying page views from the database".into()
    }).unwrap();

    // Sort requirements by ID
    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));

    let total_tests:i64 = tests.count().get_result(connection).unwrap();    

    sheet1.write_string(0,0, "Req ID", None)?;
    sheet1.write_string(0,1, "Title", None)?;
    sheet1.write_string(0,2, "Reference", None)?;


    for i in 1..total_tests+1 {
        let ts:Test = tests
        .filter(test_id.eq(i as i32))
        .get_result(connection).unwrap();

        let test_status_name = get_status_name_by_id(ts.test_status);
        let out_str = format!("Test #{} ({})", i, test_status_name);
        sheet1.write_string(0, 2+i as u16, &out_str, None)?;
    }

    let mut i = 1;
    
    
    for req in all_reqs.iter() {
        let mut j = 0;    
        sheet1.write_number(i, j, req.req_id as f64, None)?;
        j += 1;
        sheet1.write_string(i, j, &req.req_title, None)?;
        j += 1;
        sheet1.write_string(i, j, &req.req_reference, None)?;
        j += 1;

        for indx in 1..total_tests+1 {   
            let test_present :i64 = matrix
            .filter(matrix_req_id.eq(req.req_id))
            .filter(matrix_test_id.eq(indx as i32))
            .count()
            .get_result(connection).unwrap();
            
            if test_present > 0 {
                //out_str = format!("{}<td>Yes</td>", out_str);
                sheet1.write_string(i, j, "Yes", None)?;
                j += 1;
            } else {
                //out_str = format!("{}<td>No</td>", out_str);
                sheet1.write_string(i, j, "No", None)?;
                j += 1;
            }
        }
        //out_str = format!("{}</tr>\n", out_str);
        i += 1;
    }

    workbook.close().expect("workbook can be closed");
    let result = fs::read("target/matrix.xls").expect("can read file");
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