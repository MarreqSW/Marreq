use diesel::prelude::*;
use rocket::serde::json::{json, Json, Value};

use crate::helper_functions::*;
use crate::models::*;

// --------------------------------
// API Routes
// --------------------------------

/// Requirements
#[get("/requirements")]
pub fn api_get_requirement() -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    let ret_val = get_requirements_all()
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
    let connection = &mut establish_connection();
    let ret_value = insert_new_requirement(connection, &new_req);

    if let Ok(val) = ret_value {
        // Log the requirement creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_req) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Requirement,
                val,
                Some(new_req.project_id),
                new_values,
                Some(format!("Created requirement via API: {}", new_req.req_title)),
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
    let connection = &mut establish_connection();
    
    // Get the requirement details before deleting
    let requirement = get_requirement_by_id(ident);
    
    let ret_value = delete_requirement(connection, &ident).unwrap();

    #[cfg(debug_assertions)]
    println!("Delete value: {}", ret_value);
    if ret_value {
        // Log the requirement deletion via API
        if let Ok(old_values) = crate::logger::Logger::to_json_string(&requirement) {
            let _ = crate::logger::Logger::log_delete(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Requirement,
                ident,
                Some(requirement.project_id),
                old_values,
                Some(format!("Deleted requirement via API: {}", requirement.req_title)),
                None,
            );
        }
        rocket::http::Status::NoContent
    } else {
        rocket::http::Status::Accepted
    }
}

#[get("/requirements/<ident>")]
pub fn api_get_requirement_by_id(
    ident: i32,
) -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    use crate::schema::requirements::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = requirements
        .filter(req_id.eq(ident))
        .load::<Requirement>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

/// Categories
#[get("/categories")]
pub fn api_get_categories() -> Result<Json<Vec<Category>>, rocket::http::Status> {
    let ret_val = get_categories_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

/// Status
#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, rocket::http::Status> {
    let ret_val = get_status_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);
    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/status", data= "<new_status>")]
pub async fn api_post_status(new_status: Json<NewStatus>) -> Value {
    let connection = &mut establish_connection();
    let new_id = create_status (connection, &new_status).unwrap();

    json!({ "status": "ok", "id": new_id })
}

/// Tests
#[get("/tests")]
pub fn api_get_test() -> Result<Json<Vec<Test>>, rocket::http::Status> {
    let ret_val = get_tests_all()
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

#[get("/tests/<ident>")]
pub fn api_get_test_by_id(ident: i32) -> Result<Json<Vec<Test>>, rocket::http::Status> {
    use crate::schema::tests::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = tests
        .filter(test_id.eq(ident))
        .load::<Test>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/tests", data = "<new_test>")]
pub async fn api_post_test(new_test: Json<NewTest>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = create_test(connection, &new_test);

    if let Ok(val) = ret_value {
        // Log the test creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_test) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Test,
                val,
                Some(new_test.project_id),
                new_values,
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
    let connection = &mut establish_connection();
    
    // Get the test details before deleting
    let test = get_test_by_id(ident);
    
    let ret_value = delete_test(connection, &ident).unwrap();

    if ret_value {
        // Log the test deletion via API
        if let Ok(old_values) = crate::logger::Logger::to_json_string(&test) {
            let _ = crate::logger::Logger::log_delete(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Test,
                ident,
                Some(test.project_id),
                old_values,
                Some(format!("Deleted test via API: {}", test.test_name)),
                None,
            );
        }
        rocket::http::Status::NoContent
    } else {
        rocket::http::Status::Accepted
    }
}

/// Users
#[get("/users")]
pub fn api_get_users() -> Result<Json<Vec<User>>, rocket::http::Status> {
    let ret_val = get_users_all()
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

#[get("/users/<ident>")]
pub fn api_get_users_by_id(ident: i32) -> Result<Json<Vec<User>>, rocket::http::Status> {
    use crate::schema::users::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = users
        .filter(user_id.eq(ident))
        .load::<User>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/users", data = "<new_user>")]
pub async fn api_post_user(new_user: Json<NewUser>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = create_user(connection, &new_user);

    if let Ok(val) = ret_value {
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/users/<ident>")]
pub async fn api_delete_user_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    let ret_value = delete_user(connection, &ident);

    if let Ok(val) = ret_value {
        if val {
            rocket::http::Status::NoContent
        } else {
            rocket::http::Status::Accepted
        }
    } else {
        rocket::http::Status::BadRequest
    }
}

/// Matrix

#[get("/matrix")]
pub fn api_get_matrix() -> Result<Json<Vec<Matrix>>, rocket::http::Status> {
    use crate::schema::matrix::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = matrix
        .load::<Matrix>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

/// Categories - Enhanced API endpoints

#[get("/categories/<ident>")]
pub fn api_get_category_by_id(ident: i32) -> Result<Json<Category>, rocket::http::Status> {
    let ret_val = get_category_by_id(ident);
    Ok(Json(ret_val))
}

#[post("/categories", data = "<new_category>")]
pub async fn api_post_category(new_category: Json<NewCategory>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = insert_new_category(connection, &new_category);

    if let Ok(val) = ret_value {
        // Log the category creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_category) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Category,
                val,
                Some(new_category.project_id),
                new_values,
                Some(format!("Created category via API: {}", new_category.cat_title)),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[put("/categories/<ident>", data = "<category>")]
pub async fn api_put_category(ident: i32, category: Json<NewCategory>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    
    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(ident);
    
    let ret_value = edit_category(connection, &category_with_id);

    if let Ok(val) = ret_value {
        if val {
            Ok(json!({ "status": "ok", "message": "Category updated successfully" }))
        } else {
            Err(rocket::http::Status::NotFound)
        }
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/categories/<ident>")]
pub async fn api_delete_category_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    
    // Get the category details before deleting
    let category = get_category_by_id(ident);
    
    let ret_value = delete_category(connection, &ident);

    if let Ok(val) = ret_value {
        if val {
            // Log the category deletion via API
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&category) {
                let _ = crate::logger::Logger::log_delete(
                    connection,
                    0, // API user ID (system)
                    crate::models::EntityType::Category,
                    ident,
                    Some(category.project_id),
                    old_values,
                    Some(format!("Deleted category via API: {}", category.cat_title)),
                    None,
                );
            }
            rocket::http::Status::NoContent
        } else {
            rocket::http::Status::NotFound
        }
    } else {
        rocket::http::Status::BadRequest
    }
}

/// Applicability - Complete API endpoints

#[get("/applicability")]
pub fn api_get_applicability() -> Result<Json<Vec<Applicability>>, rocket::http::Status> {
    let ret_val = get_applicability_all()
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying applicability: {:?}", _err);
            "Error querying applicability from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[get("/applicability/<ident>")]
pub fn api_get_applicability_by_id(ident: i32) -> Result<Json<Applicability>, rocket::http::Status> {
    let ret_val = get_applicability_by_id(ident);
    Ok(Json(ret_val))
}

#[post("/applicability", data = "<new_applicability>")]
pub async fn api_post_applicability(new_applicability: Json<NewApplicability>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = insert_new_applicability(connection, &new_applicability);

    if let Ok(val) = ret_value {
        // Log the applicability creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_applicability) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Applicability,
                val,
                Some(new_applicability.project_id),
                new_values,
                Some(format!("Created applicability via API: {}", new_applicability.app_title)),
                None,
            );
        }
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[put("/applicability/<ident>", data = "<applicability>")]
pub async fn api_put_applicability(ident: i32, applicability: Json<NewApplicability>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    
    let mut applicability_with_id = applicability.into_inner();
    applicability_with_id.app_id = Some(ident);
    
    let ret_value = edit_applicability(connection, &applicability_with_id);

    if let Ok(val) = ret_value {
        if val {
            Ok(json!({ "status": "ok", "message": "Applicability updated successfully" }))
        } else {
            Err(rocket::http::Status::NotFound)
        }
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/applicability/<ident>")]
pub async fn api_delete_applicability_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    
    // Get the applicability details before deleting
    let applicability = get_applicability_by_id(ident);
    
    let ret_value = delete_applicability(connection, &ident);

    if let Ok(val) = ret_value {
        if val {
            // Log the applicability deletion via API
            if let Ok(old_values) = crate::logger::Logger::to_json_string(&applicability) {
                let _ = crate::logger::Logger::log_delete(
                    connection,
                    0, // API user ID (system)
                    crate::models::EntityType::Applicability,
                    ident,
                    Some(applicability.project_id),
                    old_values,
                    Some(format!("Deleted applicability via API: {}", applicability.app_title)),
                    None,
                );
            }
            rocket::http::Status::NoContent
        } else {
            rocket::http::Status::NotFound
        }
    } else {
        rocket::http::Status::BadRequest
    }
}
