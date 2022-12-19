use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use diesel::dsl::now;
use diesel::pg::PgConnection;
//use rocket::serde::json::{Json, Value, json};

use crate::models::*;

/// Returns the status list 
pub fn get_status_all() -> Result<Vec<Status>, String> {
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();

    status
    .get_results(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

/// Returns the categories list
pub fn get_categories_all() -> Result<Vec<Category>, String> {
    use crate::schema::categories::dsl::*;

    let connection = &mut establish_connection();

    categories
    .get_results(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
} 

pub fn get_category_by_id(id: i32) -> Category {
    use crate::schema::categories::dsl::*;

    let connection = &mut establish_connection();

    categories
    .filter(cat_id.eq(id))
    .get_result(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    }).unwrap()
}

/// Returns a DecorateRequirement vector for a given requirement vector
/// This function never fails, but if some requirement data is not found 
/// is filled with default value.
pub fn decorate_requirements (reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {

    let mut result = Vec::new();

    for r in reqs {
        let a = DecoratedRequirement {
            req_id: r.req_id,
            req_title: r.req_title,
            req_verification: get_verification_by_id(r.req_verification).ver_title,
            req_description: r.req_description,
            req_current_status: get_status_by_id(r.req_current_status).st_title,
            req_author : 
                if r.req_author != 0 {
                    get_user_by_id(r.req_author).user_name
                } else {
                    "".to_string()
                },
            req_reviewer: 
                if r.req_reviewer != 0 {
                    get_user_by_id(r.req_reviewer).user_name
                } else {
                    "".to_string()
                },
            req_link: r.req_link,
            req_reference: r.req_reference,
            req_category: get_category_by_id(r.req_category).cat_title,
            req_parent_id: r.req_parent,
            
            req_parent_title: 
                if r.req_parent != 0 {
                    get_requirement_by_id(r.req_parent).req_title
                } else {
                    "".to_string()
                },
            req_creation_date: r.req_creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_update_date: r.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_deadline_date: r.req_deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
        };
        result.push(a);
    }

    result
}

pub fn get_user_by_id(id: i32) -> User {
    
    use crate::schema::users::dsl::*;
    
    let connection = &mut establish_connection();
    let result:User = users
    .filter(user_id.eq(id))
    .get_result(connection).unwrap();

    result
}

pub fn get_status_by_id(id: i32) -> Status {
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();
    let result:Status = status
    .filter(st_id.eq(id))
    .get_result(connection).unwrap();

    result
}

pub fn get_verification_by_id(id: i32) -> Verification {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();
    let result:Verification = verification
    .filter(verification_id.eq(id))
    .get_result(connection).unwrap();

    result   
}

pub fn get_status_name_by_id(id: i32) -> String {
    get_status_by_id(id).st_title
}

pub fn get_requirement_by_id(id: i32) -> Requirement {
    use crate::schema::requirements::dsl::*;

    let connection = &mut establish_connection();
    let result:Requirement = requirements
    .filter(req_id.eq(id))
    .get_result(connection).unwrap();

    result
}

pub fn get_requirement_title_by_id(id: i32) -> String {
    get_requirement_by_id(id).req_title
}

/// Return all requirements 
pub fn get_requirements_all() -> Result<Vec<Requirement> , String> {
    use crate::schema::requirements::dsl::*;

    let connection = &mut establish_connection();

    requirements
    .load::<Requirement>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

pub fn get_tests_all() -> Result<Vec<Test> , String> {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    tests
    .load::<Test>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

pub fn get_users_all() -> Result<Vec<User>, String> {
    use crate::schema::users::dsl::*;

    let connection = &mut establish_connection();

    users
    .load::<User>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

/// Return all verification types 
pub fn get_verification_all() -> Result<Vec<Verification> , String> {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();

    verification
    .load::<Verification>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

pub fn get_tests_by_id(id: i32) -> Test {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();
    let result:Test = tests
    .filter(test_id.eq(id))
    .get_result(connection).unwrap();

    result
}

pub fn get_test_status_by_id(id: i32) -> String {

    use crate::schema::tests::dsl::*;
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();

    let ts:Test = tests
    .filter(test_id.eq(id))
    .get_result(connection).unwrap();

    let result:Status = status
    .filter(st_id.eq(ts.test_status))
    .get_result(connection).unwrap();

    result.st_title
}

pub fn insert_new_requirement(conn: &mut PgConnection, new: &NewRequirement) 
            -> Result<i32, Box<dyn Error>> 
{
    let a:Requirement = diesel::insert_into(crate::schema::requirements::table)
    .values(new)
    .get_result(conn)?;

    println!("New requirement id {}", a.req_id);

    Ok(a.req_id)
}

pub fn edit_requirement(conn: &mut PgConnection, new: &NewRequirement) 
            -> Result<bool, Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    diesel::update(requirements)
    .filter(req_id.eq(new.req_id))
    .set(new)
    .execute(conn)?;

    Ok(true)

}

pub fn insert_new_test( conn: &mut PgConnection, new: &NewTest) -> Result<i32, Box<dyn Error>> 
{
    let a:Test = diesel::insert_into(crate::schema::tests::table)
    .values(new)
    .get_result(conn)?;

    println!("New test id {}", a.test_id);

    Ok(a.test_id)
}

pub fn insert_new_matrix_item (conn: &mut PgConnection, new: &NewMatrix) 
        -> Result<(), Box<dyn Error>> 
{
    println!("Inserting, ({}, {})", new.matrix_req_id, new.matrix_test_id);
    diesel::insert_into(crate::schema::matrix::table)
    .values(new)
    .execute(conn)?;

    Ok(())
}

pub fn update_requirement(conn: &mut PgConnection, req: i32) -> Result<(), Box<dyn Error>> 
{
    use crate::schema::requirements::dsl::*;

    diesel::update(requirements)
    .filter(req_id.eq(req))
    .set(req_update_date.eq(now))
    .execute(conn)?;

    Ok(())
}

pub fn create_test(conn: &mut PgConnection, new: &NewTest)
            -> Result<(), Box<dyn Error>>
{
    diesel::insert_into(crate::schema::tests::table)
    .values(new)
    .execute(conn)?;

    Ok(())
}

pub fn establish_connection() -> diesel::PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}