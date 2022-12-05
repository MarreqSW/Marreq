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

    let workbook = Workbook::new("target/matrix.xlsx")?;
    let mut sheet1 = workbook.add_worksheet(None)?;
    
    let all_reqs = requirements
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap();

    let total_tests:i64 = tests.count().get_result(connection).unwrap();    

    sheet1.write_string(0,0, "Req ID", None)?;
    sheet1.write_string(0,1, "Title", None)?;
    sheet1.write_string(0,2, "Reference", None)?;


    for i in 1..total_tests+1 {
        let ts:Tests = tests
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
    let result = fs::read("target/matrix.xlsx").expect("can read file");
    Ok(result)
}