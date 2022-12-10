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


pub fn get_author_by_id(id: i32) -> String {
    "Màrius".to_string()
}

pub fn get_status_by_id(id: i32) -> Status {
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();
    let result:Status = status
    .filter(st_id.eq(id))
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

pub fn get_tests_all() -> Result<Vec<Tests> , String> {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    tests
    .load::<Tests>(connection)
    .map_err(|err| -> String {
        println!("Error querying page views: {:?}", err);
        "Error querying page views from the database".into()
    })
}

pub fn get_tests_by_id(id: i32) -> Tests {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();
    let result:Tests = tests
    .filter(test_id.eq(id))
    .get_result(connection).unwrap();

    result
}

pub fn get_test_status_by_id(id: i32) -> String {

    use crate::schema::tests::dsl::*;
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();

    let ts:Tests = tests
    .filter(test_id.eq(id))
    .get_result(connection).unwrap();

    let result:Status = status
    .filter(st_id.eq(ts.test_status))
    .get_result(connection).unwrap();

    result.st_title
}

pub fn create_requirement(conn: &mut PgConnection, new: &NewRequirement) 
            -> Result<(), Box<dyn Error>> 
{
    diesel::insert_into(crate::schema::requirements::table)
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