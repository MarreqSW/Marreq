use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use diesel::dsl::now;
use diesel::pg::PgConnection;

use crate::models::*;

pub fn get_status_name_by_id(id: i32) -> String {
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();
    let result:Status = status
    .filter(st_id.eq(id))
    .get_result(connection).unwrap();


    result.st_title
}

pub fn establish_connection() -> diesel::PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
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