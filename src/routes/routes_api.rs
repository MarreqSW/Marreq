use diesel::prelude::*;
use rocket::serde::json::{json, Json, Value};
use crate::helper_functions::*;
use crate::models::*;
use crate::repository::{DieselRepo, RequirementsRepository, TestsRepository, LookupRepository, UserRepository};

// --------------------------------
// API Routes
// --------------------------------

/// Requirements
#[get("/requirements")]
pub fn api_get_requirement() -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    let ret_val = DieselRepo::new()
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
    let connection = &mut establish_connection();
    let ret_value = DieselRepo::new().insert_new_requirement(&new_req);

    if let Ok(val) = ret_value {
        // Log the requirement creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_req) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Requirement,
                val,
                Some(new_req.project_id),
                Some(new_values),
                Some(format!("Created requirement via API: {}", new_req.req_title)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_requirement_cache(val);
        crate::cache::invalidate_project_cache(new_req.project_id);
        
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/requirements/<ident>")]
pub async fn api_delete_requirement_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    
    // Get the requirement details before deleting
    let requirement = DieselRepo::new()
        .get_requirement_by_id(ident)
        .unwrap_or_else(|_| Requirement {
            req_id: ident,
            req_title: format!("Unknown Requirement ({})", ident),
            req_description: "Requirement not found".to_string(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "".to_string(),
            req_reference: format!("REQ-UNK-{}", ident),
            req_category: 1,
            req_parent: 0,
            req_creation_date: chrono::Utc::now().naive_utc(),
            req_update_date: chrono::Utc::now().naive_utc(),
            req_deadline_date: chrono::Utc::now().naive_utc(),
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        });
    
    let ret_value = match DieselRepo::new().delete_requirement(ident) {
        Ok(success) => success,
        Err(e) => {
            eprintln!("Error deleting requirement via API: {:?}", e);
            return rocket::http::Status::InternalServerError;
        }
    };

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
                Some(old_values),
                Some(format!("Deleted requirement via API: {}", requirement.req_title)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_requirement_cache(ident);
        crate::cache::invalidate_project_cache(requirement.project_id);
        
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
    let ret_val = DieselRepo::new()
        .get_categories_all()
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
    let ret_val = DieselRepo::new()
        .get_status_all()
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
    let new_id = match DieselRepo::new().create_status(&new_status) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating status via API: {:?}", e);
            return json!({ "status": "error", "message": "Failed to create status" });
        }
    };

    // Invalidate relevant caches
    crate::cache::invalidate_status_cache(new_id);

    json!({ "status": "ok", "id": new_id })
}

/// Tests
#[get("/tests")]
pub fn api_get_test() -> Result<Json<Vec<Test>>, rocket::http::Status> {
    let ret_val = DieselRepo::new()
        .get_tests_all()
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
    let ret_value = DieselRepo::new().insert_test(&new_test);

    if let Ok(val) = ret_value {
        // Log the test creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_test) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Test,
                val,
                Some(new_test.project_id),
                Some(new_values),
                Some(format!("Created test via API: {}", new_test.test_name)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_test_cache(val);
        crate::cache::invalidate_project_cache(new_test.project_id);
        
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/tests/<ident>")]
pub async fn api_delete_test_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    
    // Get the test details before deleting
    let test = DieselRepo::new()
        .get_test_by_id(ident)
        .unwrap_or_else(|_| Test {
            test_id: ident,
            test_name: format!("Unknown Test ({})", ident),
            test_description: "Test not found".to_string(),
            test_source: String::new(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        });
    
    let ret_value = match DieselRepo::new().delete_test(ident) {
        Ok(success) => success,
        Err(e) => {
            eprintln!("Error deleting test via API: {:?}", e);
            return rocket::http::Status::InternalServerError;
        }
    };

    if ret_value {
        // Log the test deletion via API
        if let Ok(old_values) = crate::logger::Logger::to_json_string(&test) {
            let _ = crate::logger::Logger::log_delete(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Test,
                ident,
                Some(test.project_id),
                Some(old_values),
                Some(format!("Deleted test via API: {}", test.test_name)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_test_cache(ident);
        crate::cache::invalidate_project_cache(test.project_id);
        
        rocket::http::Status::NoContent
    } else {
        rocket::http::Status::Accepted
    }
}

/// Users
#[get("/users")]
pub fn api_get_users() -> Result<Json<Vec<User>>, rocket::http::Status> {
    let ret_val = DieselRepo::new()
        .get_users_all()
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
    let ret_value = DieselRepo::new().insert_user(&new_user);

    if let Ok(val) = ret_value {
        // Log the user creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_user) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::User,
                val,
                new_user.project_id,
                Some(new_values),
                Some(format!("Created user via API: {}", new_user.user_username)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_user_cache(val);
        if let Some(project_id) = new_user.project_id {
            crate::cache::invalidate_project_cache(project_id);
        }
        
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/users/<ident>")]
pub async fn api_delete_user_by_id(ident: i32) -> rocket::http::Status {
    let ret_value = DieselRepo::new().delete_user(ident);

    if let Ok(val) = ret_value {
        if val {
            // Invalidate relevant caches
            crate::cache::invalidate_user_cache(ident);
            // Note: We can't invalidate project cache here since we don't have the project_id
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
    match DieselRepo::new().get_category_by_id(ident) {
        Ok(cat) => Ok(Json(cat)),
        Err(_) => Err(rocket::http::Status::NotFound),
    }
}

#[post("/categories", data = "<new_category>")]
pub async fn api_post_category(new_category: Json<NewCategory>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = DieselRepo::new().insert_new_category(&new_category);

    if let Ok(val) = ret_value {
        // Log the category creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_category) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Category,
                val,
                Some(new_category.project_id),
                Some(new_values),
                Some(format!("Created category via API: {}", new_category.cat_title)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_category_cache(val);
        crate::cache::invalidate_project_cache(new_category.project_id);
        
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[put("/categories/<ident>", data = "<category>")]
pub async fn api_put_category(ident: i32, category: Json<NewCategory>) -> Result<Value, rocket::http::Status> {
    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(ident);
    
    let ret_value = DieselRepo::new().edit_category(&category_with_id);

    if let Ok(val) = ret_value {
        if val {
            // Invalidate relevant caches
            crate::cache::invalidate_category_cache(ident);
            crate::cache::invalidate_project_cache(category_with_id.project_id);
            
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
    let category = DieselRepo::new()
        .get_category_by_id(ident)
        .unwrap_or_else(|_| Category {
            cat_id: ident,
            cat_title: format!("Unknown Category ({})", ident),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        });
    
    let ret_value = DieselRepo::new().delete_category(ident);

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
                    Some(old_values),
                    Some(format!("Deleted category via API: {}", category.cat_title)),
                    None,
                );
            }
            
            // Invalidate relevant caches
            crate::cache::invalidate_category_cache(ident);
            crate::cache::invalidate_project_cache(category.project_id);
            
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
    let ret_val = DieselRepo::new()
        .get_applicability_all()
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
    match DieselRepo::new().get_applicability_by_id(ident) {
        Ok(app) => Ok(Json(app)),
        Err(_) => Err(rocket::http::Status::NotFound),
    }
}

#[post("/applicability", data = "<new_applicability>")]
pub async fn api_post_applicability(new_applicability: Json<NewApplicability>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = DieselRepo::new().insert_new_applicability(&new_applicability);

    if let Ok(val) = ret_value {
        // Log the applicability creation via API
        if let Ok(new_values) = crate::logger::Logger::to_json_string(&*new_applicability) {
            let _ = crate::logger::Logger::log_create(
                connection,
                0, // API user ID (system)
                crate::models::EntityType::Applicability,
                val,
                Some(new_applicability.project_id),
                Some(new_values),
                Some(format!("Created applicability via API: {}", new_applicability.app_title)),
                None,
            );
        }
        
        // Invalidate relevant caches
        crate::cache::invalidate_applicability_cache(val);
        crate::cache::invalidate_project_cache(new_applicability.project_id);
        
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[put("/applicability/<ident>", data = "<applicability>")]
pub async fn api_put_applicability(ident: i32, applicability: Json<NewApplicability>) -> Result<Value, rocket::http::Status> {
    let mut applicability_with_id = applicability.into_inner();
    applicability_with_id.app_id = Some(ident);
    
    let ret_value = DieselRepo::new().edit_applicability(&applicability_with_id);

    if let Ok(val) = ret_value {
        if val {
            // Invalidate relevant caches
            crate::cache::invalidate_applicability_cache(ident);
            crate::cache::invalidate_project_cache(applicability_with_id.project_id);
            
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
    let applicability = DieselRepo::new()
        .get_applicability_by_id(ident)
        .unwrap_or_else(|_| Applicability {
            app_id: ident,
            app_title: format!("Unknown Applicability ({})", ident),
            app_description: "Applicability not found".to_string(),
            app_tag: "unknown".to_string(),
            project_id: 1,
        });
    
    let ret_value = DieselRepo::new().delete_applicability(ident);

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
                    Some(old_values),
                    Some(format!("Deleted applicability via API: {}", applicability.app_title)),
                    None,
                );
            }
            
            // Invalidate relevant caches
            crate::cache::invalidate_applicability_cache(ident);
            crate::cache::invalidate_project_cache(applicability.project_id);
            
            rocket::http::Status::NoContent
        } else {
            rocket::http::Status::NotFound
        }
    } else {
        rocket::http::Status::BadRequest
    }
}
