use crate::models::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use rocket::http::CookieJar;
use std::env;
use std::error::Error;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn get_selected_project_id(cookies: &CookieJar<'_>) -> Option<i32> {
    cookies.get("selected_project_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok())
}

pub fn generate_requirement_reference(category_id: i32, project_id: i32) -> Result<String, Box<dyn Error>> {
    use crate::schema::categories;
    use crate::schema::requirements;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    let category = categories::table
        .filter(categories::cat_id.eq(category_id))
        .first::<Category>(connection.as_mut())
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")))?;

    let existing_count = requirements::table
        .filter(requirements::req_category.eq(category_id))
        .filter(requirements::project_id.eq(project_id))
        .count()
        .get_result::<i64>(connection.as_mut())
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Database error")))?;

    let next_number = existing_count + 1;
    let reference = format!("REQ-{}-{}", category.cat_tag, next_number);

    Ok(reference)
}
