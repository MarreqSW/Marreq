use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    DieselCachedRepo, LookupRepository, RequirementsRepository, TestsRepository, UserRepository,
};
use diesel::prelude::*;
use rocket::serde::json::{json, Json, Value};

// --------------------------------
// API Routes
// --------------------------------

/// Requirements
#[get("/requirements")]
pub fn api_get_requirement() -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    let ret_val = DieselCachedRepo::read()
        .get_requirements_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        Ok(val)
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/requirements", data = "<new_req>")]
pub async fn api_post_requirement(
    new_req: Json<NewRequirement>,
) -> Result<Value, rocket::http::Status> {
    let ret_value = DieselCachedRepo::write().insert_new_requirement(&new_req);

    if let Ok(val) = ret_value {
        // Log the requirement creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_req) {
            let _ = crate::logger::Logger::log_create(
                DieselCachedRepo::read()
                    .inner_repo()
                    .get_conn()
                    .expect("Failed to get database connection")
                    .as_mut(),
                0, // API user ID (system)
                crate::models::EntityType::Requirement,
                val,
                Some(new_req.project_id),
                Some(new_values),
                Some(format!(
                    "Created requirement via API: {}",
                    new_req.req_title
                )),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/requirements/<ident>")]
pub async fn api_delete_requirement_by_id(ident: i32) -> rocket::http::Status {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    match DieselCachedRepo::write().delete_requirement(ident) {
        Ok(requirement) => {
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&requirement) {
                let _ = crate::logger::Logger::log_delete(
                    connection.as_mut(),
                    0, // API user ID (system)
                    crate::models::EntityType::Requirement,
                    ident,
                    Some(requirement.project_id),
                    Some(old_values),
                    Some(format!(
                        "Deleted requirement via API: {}",
                        requirement.req_title
                    )),
                    None,
                );
            }
            rocket::http::Status::NoContent
        }
        Err(RepoError::NotFound) => rocket::http::Status::NotFound,
        Err(e) => {
            eprintln!("Error deleting requirement via API: {:?}", e);
            rocket::http::Status::InternalServerError
        }
    }
}

#[get("/requirements/<ident>")]
pub fn api_get_requirement_by_id(
    ident: i32,
) -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    match DieselCachedRepo::read().get_requirement_by_id(ident) {
        Ok(req) => Ok(Json(vec![req])),
        Err(crate::repository::errors::RepoError::NotFound) => Err(rocket::http::Status::NotFound),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

/// Categories
#[get("/categories")]
pub fn api_get_categories() -> Result<Json<Vec<Category>>, rocket::http::Status> {
    let ret_val = DieselCachedRepo::read()
        .get_categories_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    match ret_val {
        Ok(val) if val.is_empty() => Err(rocket::http::Status::NotFound),
        Ok(val) => Ok(val),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

/// Status
#[get("/debug_status/<id>")]
pub fn debug_status(id: i32) -> Result<String, rocket::http::Status> {
    use crate::repository::{DieselCachedRepo, LookupRepository};
    
    match DieselCachedRepo::read().get_requirement_status_by_id(id) {
        Ok(status) => Ok(format!("Status ID {}: {} ({})", status.req_st_id, status.req_st_title, status.req_st_description)),
        Err(e) => {
            eprintln!("Error getting status {}: {:?}", id, e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, rocket::http::Status> {
    let ret_val = DieselCachedRepo::read()
        .get_requirement_status_all()
        .map(|req_statuses| {
            // Convert RequirementStatus to Status for backward compatibility
            req_statuses.into_iter().map(|rs| Status {
                st_id: rs.req_st_id,
                st_title: rs.req_st_title,
                st_description: rs.req_st_description,
                st_short_name: rs.req_st_short_name,
            }).collect::<Vec<Status>>()
        })
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    match ret_val {
        Ok(val) if val.is_empty() => Err(rocket::http::Status::NotFound),
        Ok(val) => Ok(val),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

#[post("/status", data = "<new_status>")]
pub async fn api_post_status(new_status: Json<NewStatus>) -> Value {
    let new_id = match DieselCachedRepo::write().create_status(&new_status) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating status via API: {:?}", e);
            return json!({ "status": "error", "message": "Failed to create status" });
        }
    };
    json!({ "status": "ok", "id": new_id })
}

/// Tests
#[get("/tests")]
pub fn api_get_test() -> Result<Json<Vec<Test>>, rocket::http::Status> {
    let ret_val = DieselCachedRepo::read()
        .get_tests_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    ret_val.map_err(|_| rocket::http::Status::InternalServerError)
}

#[get("/tests/<ident>")]
pub fn api_get_test_by_id(ident: i32) -> Result<Json<Vec<Test>>, rocket::http::Status> {
    match DieselCachedRepo::read().get_test_by_id(ident) {
        Ok(test) => Ok(Json(vec![test])),
        Err(crate::repository::errors::RepoError::NotFound) => Err(rocket::http::Status::NotFound),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

#[post("/tests", data = "<new_test>")]
pub async fn api_post_test(new_test: Json<NewTest>) -> Result<Value, rocket::http::Status> {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");
    let ret_value = DieselCachedRepo::write().insert_test(&new_test);

    if let Ok(val) = ret_value {
        // Log the test creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_test) {
            let _ = crate::logger::Logger::log_create(
                connection.as_mut(),
                0, // API user ID (system)
                crate::models::EntityType::Test,
                val,
                Some(new_test.project_id),
                Some(new_values),
                Some(format!("Created test via API: {}", new_test.test_name)),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/tests/<ident>")]
pub async fn api_delete_test_by_id(ident: i32) -> rocket::http::Status {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    match DieselCachedRepo::write().delete_test(ident) {
        Ok(test) => {
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&test) {
                let _ = crate::logger::Logger::log_delete(
                    connection.as_mut(),
                    0, // API user ID (system)
                    crate::models::EntityType::Test,
                    ident,
                    Some(test.project_id),
                    Some(old_values),
                    Some(format!("Deleted test via API: {}", test.test_name)),
                    None,
                );
            }
            rocket::http::Status::NoContent
        }
        Err(RepoError::NotFound) => rocket::http::Status::NotFound,
        Err(e) => {
            eprintln!("Error deleting test via API: {:?}", e);
            rocket::http::Status::InternalServerError
        }
    }
}

/// Users
#[get("/users")]
pub fn api_get_users() -> Result<Json<Vec<User>>, rocket::http::Status> {
    DieselCachedRepo::read()
        .get_users_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json)
        .map_err(|_| rocket::http::Status::InternalServerError)
}

#[get("/users/<ident>")]
pub fn api_get_users_by_id(ident: i32) -> Result<Json<Vec<User>>, rocket::http::Status> {
    match DieselCachedRepo::read().get_user_by_id(ident) {
        Ok(user) => Ok(Json(vec![user])),
        Err(crate::repository::errors::RepoError::NotFound) => Err(rocket::http::Status::NotFound),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

#[post("/users", data = "<new_user>")]
pub async fn api_post_user(new_user: Json<NewUser>) -> Result<Value, rocket::http::Status> {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");
    let ret_value = DieselCachedRepo::write().insert_user(&new_user);

    if let Ok(val) = ret_value {
        // Log the user creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_user) {
            let _ = crate::logger::Logger::log_create(
                connection.as_mut(),
                0, // API user ID (system)
                crate::models::EntityType::User,
                val,
                new_user.project_id,
                Some(new_values),
                Some(format!("Created user via API: {}", new_user.user_username)),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/users/<ident>")]
pub async fn api_delete_user_by_id(ident: i32) -> rocket::http::Status {
    match DieselCachedRepo::write().delete_user(ident) {
        Ok(_) => rocket::http::Status::NoContent,
        Err(RepoError::NotFound) => rocket::http::Status::NotFound,
        Err(_) => rocket::http::Status::BadRequest,
    }
}

/// Matrix

#[get("/matrix")]
pub fn api_get_matrix() -> Result<Json<Vec<Matrix>>, rocket::http::Status> {
    use crate::schema::matrix::dsl::*;
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    let ret_val = matrix
        .load::<Matrix>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    match ret_val {
        Ok(val) if val.is_empty() => Err(rocket::http::Status::NotFound),
        Ok(val) => Ok(val),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

/// Categories - Enhanced API endpoints

#[get("/categories/<ident>")]
pub fn api_get_category_by_id(ident: i32) -> Result<Json<Category>, rocket::http::Status> {
    match DieselCachedRepo::read().get_category_by_id(ident) {
        Ok(cat) => Ok(Json(cat)),
        Err(_) => Err(rocket::http::Status::NotFound),
    }
}

#[post("/categories", data = "<new_category>")]
pub async fn api_post_category(
    new_category: Json<NewCategory>,
) -> Result<Value, rocket::http::Status> {
    let ret_value = DieselCachedRepo::write().insert_new_category(&new_category);

    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    if let Ok(val) = ret_value {
        // Log the category creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_category) {
            let _ = crate::logger::Logger::log_create(
                connection.as_mut(),
                0, // API user ID (system)
                crate::models::EntityType::Category,
                val,
                Some(new_category.project_id),
                Some(new_values),
                Some(format!(
                    "Created category via API: {}",
                    new_category.cat_title
                )),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[put("/categories/<ident>", data = "<category>")]
pub async fn api_put_category(
    ident: i32,
    category: Json<NewCategory>,
) -> Result<Value, rocket::http::Status> {
    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(ident);

    let ret_value = DieselCachedRepo::write().edit_category(&category_with_id);

    match ret_value {
        Ok(true) => Ok(json!({ "status": "ok", "message": "Category updated successfully" })),
        Ok(false) => Err(rocket::http::Status::NotFound),
        Err(_) => Err(rocket::http::Status::BadRequest),
    }
}

#[delete("/categories/<ident>")]
pub async fn api_delete_category_by_id(ident: i32) -> rocket::http::Status {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    match DieselCachedRepo::write().delete_category(ident) {
        Ok(category) => {
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&category) {
                let _ = crate::logger::Logger::log_delete(
                    connection.as_mut(),
                    0, // API user ID (system)
                    crate::models::EntityType::Category,
                    ident,
                    Some(category.project_id),
                    Some(old_values),
                    Some(format!("Deleted category via API: {}", category.cat_title)),
                    None,
                );
            }
            rocket::http::Status::NoContent
        }
        Err(RepoError::NotFound) => rocket::http::Status::NotFound,
        Err(_) => rocket::http::Status::BadRequest,
    }
}

/// Applicability - Complete API endpoints

#[get("/applicability")]
pub fn api_get_applicability() -> Result<Json<Vec<Applicability>>, rocket::http::Status> {
    let ret_val = DieselCachedRepo::read()
        .get_applicability_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying applicability: {:?}", _err);
            "Error querying applicability from the database".into()
        })
        .map(Json);

    match ret_val {
        Ok(val) if val.is_empty() => Err(rocket::http::Status::NotFound),
        Ok(val) => Ok(val),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
}

#[get("/applicability/<ident>")]
pub fn api_get_applicability_by_id(
    ident: i32,
) -> Result<Json<Applicability>, rocket::http::Status> {
    match DieselCachedRepo::read().get_applicability_by_id(ident) {
        Ok(app) => Ok(Json(app)),
        Err(_) => Err(rocket::http::Status::NotFound),
    }
}

#[post("/applicability", data = "<new_applicability>")]
pub async fn api_post_applicability(
    new_applicability: Json<NewApplicability>,
) -> Result<Value, rocket::http::Status> {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");
    let ret_value = DieselCachedRepo::write().insert_new_applicability(&new_applicability);

    match ret_value {
        Ok(val) => {
            // Log the applicability creation via API
            if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_applicability) {
                let _ = crate::logger::Logger::log_create(
                    connection.as_mut(),
                    0, // API user ID (system)
                    crate::models::EntityType::Applicability,
                    val,
                    Some(new_applicability.project_id),
                    Some(new_values),
                    Some(format!(
                        "Created applicability via API: {}",
                        new_applicability.app_title
                    )),
                    None,
                );
            }
            Ok(json!({ "status": "ok", "id": val }))
        }
        Err(_) => Err(rocket::http::Status::BadRequest),
    }
}

#[put("/applicability/<ident>", data = "<applicability>")]
pub async fn api_put_applicability(
    ident: i32,
    applicability: Json<NewApplicability>,
) -> Result<Value, rocket::http::Status> {
    let mut applicability_with_id = applicability.into_inner();
    applicability_with_id.app_id = Some(ident);

    let ret_value = DieselCachedRepo::write().edit_applicability(&applicability_with_id);

    match ret_value {
        Ok(true) => Ok(json!({ "status": "ok", "message": "Applicability updated successfully" })),
        Ok(false) => Err(rocket::http::Status::NotFound),
        Err(_) => Err(rocket::http::Status::BadRequest),
    }
}

#[delete("/applicability/<ident>")]
pub async fn api_delete_applicability_by_id(ident: i32) -> rocket::http::Status {
    let mut connection = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .expect("Failed to get database connection");

    match DieselCachedRepo::write().delete_applicability(ident) {
        Ok(applicability) => {
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&applicability) {
                let _ = crate::logger::Logger::log_delete(
                    connection.as_mut(),
                    0, // API user ID (system)
                    crate::models::EntityType::Applicability,
                    ident,
                    Some(applicability.project_id),
                    Some(old_values),
                    Some(format!(
                        "Deleted applicability via API: {}",
                        applicability.app_title
                    )),
                    None,
                );
            }
            rocket::http::Status::NoContent
        }
        Err(RepoError::NotFound) => rocket::http::Status::NotFound,
        Err(_) => rocket::http::Status::BadRequest,
    }
}
