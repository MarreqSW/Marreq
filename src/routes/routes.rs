use diesel::prelude::*;
use rocket::serde::json::{Json, Value, json};

use crate::models::*;
use crate::routes::routes::html::*;
use crate::html::*;
// use crate::DbConn;

use crate::lib::helper_functions::*;

#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}

use rocket::response::content;
#[get("/requirements")]
pub fn show_requirements() -> content::RawHtml<String> {
    use crate::schema::requirements::dsl::*;
    use crate::schema::status::dsl::*;
    use crate::schema::categories::dsl::*;
    
    let mut out_str = print_header();
    let connection = &mut establish_connection();
    
    let all_reqs = 
    requirements
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap();
    
    for req in all_reqs.iter() {

        let act_status = 
        status
        .filter(st_id.eq(req.req_status))
        .limit(1)
        .load::<Status>(connection).unwrap();
        
        
        let act_category = 
        categories
        .filter(cat_id.eq(req.req_category))
        .limit(1)
        .load::<Category>(connection).unwrap();

        out_str = format!("{}{}{}{}
        <p id='ReqEdit'><a href='{}{}'>Edit</a></p>", 
        out_str, req, act_status[0], act_category[0],
        "http://localhost:8000/requirements/edit/", req.req_id);
    }

    out_str = format!("{} {}",out_str, print_footer());
    
    content::RawHtml(out_str)
}

#[get("/requirements/edit/<req_id_ed>")]
pub fn requirement_edit(req_id_ed: i32) -> content::RawHtml<String> {
    use crate::schema::requirements::dsl::*;
    
    let mut out_str = print_header();

    let connection = &mut establish_connection();
    let requirement = requirements
    .filter(req_id.eq(req_id_ed))
    .limit(1)
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap();

    for req in requirement.iter() {
        out_str = format!("{}{}", out_str, req
        );
    }

    out_str = format!("{} {}",out_str, print_footer());
    content::RawHtml(out_str)
}

#[get("/matrix")]
pub fn get_matrix() -> content::RawHtml<String> {
    use crate::schema::requirements::dsl::*;
    use crate::schema::matrix::dsl::*;
    use crate::schema::tests::dsl::*;

    let mut out_str = print_header();
    let connection = &mut establish_connection();

    let all_reqs = requirements
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap();

    let total_tests:i64 = tests.count().get_result(connection).unwrap();    

    out_str = format!("{}<p id'title1'>Total Tests: {}</p>", out_str, total_tests);
    out_str = format!("{}<table>", out_str);
    out_str = format!("{}<tr><th>Req ID</th><th>Title</th><th>Reference</th>", out_str);

    for i in 1..total_tests+1 {
        

        let ts:Tests = tests
        .filter(test_id.eq(i as i32))
        .get_result(connection).unwrap();

        let test_status_name = get_status_name_by_id(ts.test_status);
        out_str = format!("{}<th>Test #{} ({})</th>", out_str, i, test_status_name);

        //test_status_vec.push(ts.test_status);
    }

    out_str = format!("{}</tr>", out_str);

    for req in all_reqs.iter() {
        
        out_str = format!("{}<tr><td>{}</td><td>{}</td><td>{}</td>", 
        out_str,req.req_id, req.req_title, req.req_reference);
        
        for indx in 1..total_tests+1 {   
            let test_present :i64 = matrix
            .filter(matrix_req_id.eq(req.req_id))
            .filter(matrix_test_id.eq(indx as i32))
            .count()
            .get_result(connection).unwrap();
            
            if test_present > 0 {
                out_str = format!("{}<td>Yes</td>", out_str);
            } else {
                out_str = format!("{}<td>No</td>", out_str);
            }
        }
        out_str = format!("{}</tr>\n", out_str);
    }

    out_str = format!("{}</table>", out_str);
    out_str = format!("{} {}",out_str, print_footer());
    
    content::RawHtml(out_str)
}


// --------------------------------
// API
// --------------------------------
#[get("/requirements")]
pub fn api_get_reqs() -> Result<Json<Vec<Requirement>>, String> {
    use crate::schema::requirements::dsl::*;

    let connection = &mut establish_connection();

    requirements
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[post("/requirements", data = "<new_req>")]
pub async fn api_post_requirement(new_req: Json<NewRequirement>) -> Value {
    let connection = &mut establish_connection();
    create_requirement (connection, &new_req).unwrap();

    json!({ "status": "ok", "id": 5 })
}

#[get("/requirements/<ident>")]
pub fn api_get_reqs_by_id(ident: i32) -> Result<Json<Vec<Requirement>>, String> {
    use crate::schema::requirements::dsl::*;
    let connection = &mut establish_connection();

    requirements
    .filter(req_id.eq(ident))
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[get("/categories")]
pub fn api_get_categories() -> Result<Json<Vec<Category>>, String> {
    use crate::schema::categories::dsl::*;
    let connection = &mut establish_connection();

    categories
    .load::<Category>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, String> {
    use crate::schema::status::dsl::*;
    let connection = &mut establish_connection();

    status
    .load::<Status>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}
 
#[get("/matrix")]
pub fn api_get_matrix() -> Result<Json<Vec<Matrix>>, String> {
    use crate::schema::matrix::dsl::*;
    let connection = &mut establish_connection();

    matrix
    .load::<Matrix>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}