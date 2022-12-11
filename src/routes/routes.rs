use diesel::prelude::*;
use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::serde::json::{Json, Value, json};
use rocket::response::status::NotFound;
use rocket::response::{content, Redirect};
use rocket::form::Form;

//use rocket_dyn_templates::{Template, context, handlebars};
use rocket_dyn_templates::Template;

use std::path;

use crate::models::*;
use crate::routes::routes::html::*;
use crate::html::*;
use crate::generators::*;
use crate::helper_functions::*;

//use crate::DbConn;

#[get("/")]
pub fn index() -> Template {
    let ctx = json!({ "title": "Main"});
    Template::render("index", ctx)
}

#[get("/requirements")]
pub fn show_requirements() -> Template {

    let requirements = get_requirements_all().unwrap();
    let requirements_decorate = decorate_requirements(requirements);
    let requirements_json = json!(requirements_decorate);
    let ctx = json!({"requirements": requirements_json});
    
    Template::render("requirements", ctx)
}

#[get("/requirements/<req_id>")]
pub fn show_requirement_id(req_id: i32) -> Template {
    let req = get_requirement_by_id(req_id);
    let req_decorate = decorate_requirements(vec!(req));
    let ctx = json!({"requirements": req_decorate});

    Template::render("requirement_by_id", ctx)
}

#[get("/new_requirement")]
pub fn new_requirement() -> Template {
    let status = get_status_all().unwrap();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap();
    let categories_json = json!(categories);

    let parents = get_requirements_all().unwrap();
    let parents_json = json!(parents);
    
    let users = get_users_all().unwrap();
    let users_json = json!(users);

    let ctx = json!({"categories": categories_json, "status": status_json, "parent": parents_json, "users": users_json});

    Template::render("new_requirement", ctx)
}

#[post("/new_requirement", data = "<new_req>")]
pub fn post_requirement(new_req: Form<NewRequirement>) -> Redirect {

    let connection = &mut establish_connection();
    let my_id = insert_new_requirement (connection, &new_req).unwrap();

    Redirect::to(uri!(show_requirement_id(my_id)))
}

#[get("/tests")]
pub fn show_tests() -> Template {
    let tests = get_tests_all();
    let tests = json!(tests);

    Template::render("tests", tests)
}

#[get("/tests/<test_id_param>")]
pub fn show_test_id(test_id_param: i32) -> Template {
    let tests = get_tests_by_id(test_id_param);
    let tests = json!(tests);

    Template::render("test_by_id", tests)
}

#[get("/new_test")]
pub fn new_test() -> Template {
    let status = get_status_all().unwrap();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap();
    let categories_json = json!(categories);

    let parents = get_tests_all().unwrap();
    let parents_json = json!(parents);
    
    let users = get_users_all().unwrap();
    let users_json = json!(users);

    let requirements = get_requirements_all().unwrap();
    let requirements_json = json!(requirements);

    let ctx = json!({"categories": categories_json, "status": status_json, "parents": parents_json, "users": users_json, "requirements": requirements_json});

    Template::render("new_test", ctx)
}

#[post("/new_test", data = "<new_test>")]
pub fn post_test(new_test: Form<NewTestForm>) -> Redirect {

    let connection = &mut establish_connection();
    let my_new_test = NewTest {
        test_name: new_test.test_name.clone(),
        test_description: new_test.test_description.clone(),
        test_source : new_test.test_source.clone(),
        test_status : new_test.test_status,
        test_parent : new_test.test_parent,
    };
    let my_id = insert_new_test (connection, &my_new_test).unwrap();

    println!("NewTestForm requirements: {:#?}", new_test.test_req);
    for req in new_test.test_req.iter() {
        let matrix_item = NewMatrix {
            matrix_req_id: *req,
            matrix_test_id: my_id,
        };
        insert_new_matrix_item (connection, &matrix_item).unwrap();
    }

    Redirect::to(uri!(show_test_id(my_id)))
}

#[get("/status")]
pub fn show_status() -> content::RawHtml<String> {
    use crate::schema::status::dsl::*;

    let mut out_str = print_header();
    let connection = &mut establish_connection();

    let all_status =
    status
    .load::<Status>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap();

    for st in all_status.iter() {
        out_str = format!("{}
        <div class='AllStatus'>
            <div>Id: {}</div>
            <div>Title: {}</div>
            <div>Description: {}</div>
        </div>",
        out_str, st.st_id, st.st_title, st.st_description);
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

    out_str = format!("{}<p id='title1'>Total Tests: {}</p>", out_str, total_tests);
    out_str = format!("{}<table>", out_str);
    out_str = format!("{}<tr><th>Req ID</th><th>Title</th><th>Reference</th>", out_str);

    /* Prepare table headers */
    for i in 1..total_tests+1 {        
        let ts:Test = tests
        .filter(test_id.eq(i as i32))
        .get_result(connection).unwrap();

        let test_status_name = get_status_name_by_id(ts.test_status);
        out_str = format!("{}<th><a href='tests/{}'>Test #{}</a> ({})</th>", out_str, i, i, test_status_name);
    }

    out_str = format!("{}</tr>", out_str);

    /* 
     * Show all test (M) for every requirement (N)
     * NOTE: Not efficient O(N*M) !!!
     */
    for req in all_reqs.iter() {
        
        out_str = format!("{}<tr><td><a href='requirements/{}'>{}</a></td><td>{}</td><td>{}</td>", 
        out_str, req.req_id, req.req_id, req.req_title, req.req_reference);
        
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

#[get("/matrix/xls")]
pub async fn get_matrix_xls() -> (ContentType, NamedFile) {
    let _file = excel::create_matrix_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/matrix.xlsx");
    let res = NamedFile::open(&path_to_file).await.map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new("application", "vnd.openxmlformats-officedocument.spreadsheetml.sheet");
            (content_type, file)
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

// --------------------------------
// API
// --------------------------------
#[get("/requirements")]
pub fn api_get_reqs() -> Result<Json<Vec<Requirement>>, String> {
    
    get_requirements_all()
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[post("/requirements", data = "<new_req>")]
pub async fn api_post_requirement(new_req: Json<NewRequirement>) -> Value {
    let connection = &mut establish_connection();
    insert_new_requirement (connection, &new_req).unwrap();

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
    get_categories_all()
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, String> {
    
    get_status_all()
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[get("/tests")]
pub fn api_get_tests() -> Result<Json<Vec<Test>>, String> {

    get_tests_all()
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[get("/tests/<ident>")]
pub fn api_get_tests_by_id(ident: i32) -> Result<Json<Vec<Test>>, String> {
    use crate::schema::tests::dsl::*;
    let connection = &mut establish_connection();

    tests
    .filter(test_id.eq(ident))
    .load::<Test>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).map(Json)
}

#[post("/tests", data = "<new_test>")]
pub async fn api_post_test(new_test: Json<NewTest>) -> Value {
    let connection = &mut establish_connection();
    create_test (connection, &new_test).unwrap();

    json!({ "status": "ok", "id": 5 })
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
