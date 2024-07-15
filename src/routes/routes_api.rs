use diesel::prelude::*;
use rocket::form::Form;
use rocket::response::Redirect;
use rocket::serde::json::{json, Json, Value};

use rocket_dyn_templates::Template;

use crate::helper_functions::*;
use crate::models::*;
use crate::routes::routes_html::*;

#[get("/requirements")]
pub fn api_get_reqs() -> Result<Json<Vec<Requirement>>, String> {
    get_requirements_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
}

#[post("/requirements", data = "<new_req>")]
pub async fn api_post_requirement(new_req: Json<NewRequirement>) -> Value {
    let connection = &mut establish_connection();
    insert_new_requirement(connection, &new_req).unwrap();

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
        })
        .map(Json)
}

#[get("/categories")]
pub fn api_get_categories() -> Result<Json<Vec<Category>>, String> {
    get_categories_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
}

#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, String> {
    get_status_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
}

#[get("/tests")]
pub fn api_get_tests() -> Result<Json<Vec<Test>>, String> {
    get_tests_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
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
        })
        .map(Json)
}

#[post("/tests", data = "<new_test>")]
pub async fn api_post_test(new_test: Json<NewTest>) -> Value {
    let connection = &mut establish_connection();
    create_test(connection, &new_test).unwrap();

    json!({ "status": "ok", "id": 5 })
}

#[get("/users")]
pub fn api_get_users() -> Result<Json<Vec<User>>, String> {
    get_users_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
}

#[get("/users/<ident>")]
pub fn api_get_users_by_id(ident: i32) -> Result<Json<Vec<User>>, String> {
    use crate::schema::users::dsl::*;
    let connection = &mut establish_connection();

    users
        .filter(user_id.eq(ident))
        .load::<User>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json)
}

#[post("/users", data = "<new_user>")]
pub async fn api_post_user(new_user: Json<NewUser>) -> Value {
    let connection = &mut establish_connection();
    create_user(connection, &new_user).unwrap();

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
        })
        .map(Json)
}

#[get("/new_user")]
pub fn new_user() -> Template {
    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap_or_default();
    let categories_json = json!(categories);

    let parents = get_tests_all().unwrap_or_default();
    let parents_json = json!(parents);

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    let requirements = get_requirements_all().unwrap_or_default();
    let requirements_json = json!(requirements);

    let ctx = json!({"categories": categories_json, "status": status_json, "parents": parents_json, "users": users_json, "requirements": requirements_json});

    Template::render("new_user", ctx)
}

#[post("/new_user", data = "<new_user>")]
pub fn post_user(new_user: Form<NewUser>) -> Redirect {
    let connection = &mut establish_connection();
    let my_id = insert_new_user(connection, &new_user).unwrap();

    Redirect::to(uri!(show_user_id(my_id)))
}
