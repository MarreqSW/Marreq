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

    let mut connection = crate::repository::get_connection()
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

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Cookie;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/")]
    fn read_cookie_route(cookies: &CookieJar<'_>) -> String {
        get_selected_project_id(cookies)
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".into())
    }

    #[test]
    fn get_selected_project_id_returns_id() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client
            .get("/")
            .cookie(Cookie::new("selected_project_id", "42"))
            .dispatch();
        assert_eq!(response.into_string().unwrap(), "42");
    }

    #[test]
    fn get_selected_project_id_missing_cookie() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string().unwrap(), "none");
    }

    #[test]
    fn get_selected_project_id_invalid_cookie() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client
            .get("/")
            .cookie(Cookie::new("selected_project_id", "abc"))
            .dispatch();
        assert_eq!(response.into_string().unwrap(), "none");
    }

}
