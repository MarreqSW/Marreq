use crate::models::*;
use diesel::dsl::now;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use serde::{Deserialize, Serialize};

// Authentication helper functions
use bcrypt::{hash, verify, DEFAULT_COST};
use rocket::http::CookieJar;

pub fn is_authenticated(cookies: &CookieJar<'_>) -> Option<User> {
    let user_id_cookie = cookies.get_private("user_id");
    let username_cookie = cookies.get_private("username");
    
    match (user_id_cookie, username_cookie) {
        (Some(user_id_cookie), Some(username_cookie)) => {
            match user_id_cookie.value().parse::<i32>() {
                Ok(user_id) => {
                    // Verify the user still exists in the database
                    let user = get_user_by_id(user_id);
                    if user.user_username == username_cookie.value() {
                        Some(user)
                    } else {
                        None
                    }
                }
                Err(_) => None
            }
        }
        _ => None
    }
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn authenticate_user(username: &str, password: &str) -> Result<Option<User>, String> {
    use crate::schema::users::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    let user = users
        .filter(user_username.eq(username))
        .first::<User>(connection.as_mut())
        .optional()
        .map_err(|_e| format!("Database error: {}", _e))?;
    
    match user {
        Some(user) => {
            match verify_password(password, &user.user_password) {
                Ok(true) => Ok(Some(user)),
                Ok(false) => Ok(None),
                Err(e) => Err(format!("Password verification error: {}", e)),
            }
        }
        None => Ok(None),
    }
}

pub fn change_user_password(_user_id: i32, current_password: &str, new_password: &str) -> Result<(), String> {
    use crate::schema::users::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    // Get the user
    let user = users
        .filter(user_id.eq(user_id))
        .first::<User>(connection.as_mut())
        .map_err(|_e| format!("User not found: {}", _e))?;
    
    // Verify current password
    match verify_password(current_password, &user.user_password) {
        Ok(true) => {
            // Hash new password
            let hashed_password = hash_password(new_password)
                .map_err(|_e| format!("Password hashing error: {}", _e))?;
            
            // Update password in database
            diesel::update(users.filter(user_id.eq(user_id)))
                .set(user_password.eq(hashed_password))
                .execute(connection.as_mut())
                .map_err(|_e| format!("Database update error: {}", _e))?;
            
            Ok(())
        }
        Ok(false) => Err("Current password is incorrect".to_string()),
        Err(e) => Err(format!("Password verification error: {}", e)),
    }
}

/// Returns the status list
pub fn get_status_all() -> Result<Vec<Status>, String> {
    use crate::schema::status::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    status
        .order(st_id)
        .load::<Status>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying status: {:?}", _err);
            "Error querying status from the database".into()
        })
}

/// Returns the categories list
pub fn get_categories_all() -> Result<Vec<Category>, String> {
    use crate::schema::categories::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    categories
        .order(cat_id)
        .load::<Category>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying categories: {:?}", _err);
            "Error querying categories from the database".into()
        })
}

/// Returns the applicability list
pub fn get_applicability_all() -> Result<Vec<Applicability>, String> {
    use crate::schema::applicability::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    applicability
        .order(app_id)
        .load::<Applicability>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying applicability: {:?}", _err);
            "Error querying applicability from the database".into()
        })
}

pub fn get_applicability_by_id(id: i32) -> Applicability {
    use crate::schema::applicability::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    applicability
        .filter(app_id.eq(id))
        .get_result(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying applicability: {:?}", _err);
            "Error querying applicability from the database".into()
        })
        .unwrap()
}

/// Get applicability by ID with project filtering and fallback
pub fn get_applicability_by_id_safe(id: i32, target_project_id: i32) -> Applicability {
    use crate::schema::applicability::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    // First try to find the applicability in the specific project
    match applicability
        .filter(app_id.eq(id))
        .filter(crate::schema::applicability::project_id.eq(target_project_id))
        .get_result::<Applicability>(connection.as_mut()) {
        Ok(result) => result,
        Err(_) => {
            // Fallback: try to find any applicability with this ID
            match applicability
                .filter(app_id.eq(id))
                .get_result::<Applicability>(connection.as_mut()) {
                Ok(result) => result,
                Err(_) => {
                    // Final fallback: return a default applicability
                    Applicability {
                        app_id: id,
                        app_title: format!("Unknown Applicability ({})", id),
                        app_description: "Applicability not found".to_string(),
                        app_tag: "unknown".to_string(),
                        project_id: target_project_id,
                    }
                }
            }
        }
    }
}

pub fn get_category_by_id(id: i32) -> Category {
    use crate::schema::categories::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    categories
        .filter(cat_id.eq(id))
        .get_result(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying category: {:?}", _err);
            format!("Error querying category with ID {} from the database", id)
        })
        .unwrap_or_else(|_| Category {
            cat_id: id,
            cat_title: format!("Unknown Category ({})", id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        })
}

/// Get category by ID with project filtering and fallback
pub fn get_category_by_id_safe(id: i32, target_project_id: i32) -> Category {
    use crate::schema::categories::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    // First try to find the category in the specific project
    match categories
        .filter(cat_id.eq(id))
        .filter(crate::schema::categories::project_id.eq(target_project_id))
        .get_result::<Category>(connection.as_mut()) {
        Ok(result) => result,
        Err(_) => {
            // Fallback: try to find any category with this ID
            match categories
                .filter(cat_id.eq(id))
                .get_result::<Category>(connection.as_mut()) {
                Ok(result) => result,
                Err(_) => {
                    // Final fallback: return a default category
                    Category {
                        cat_id: id,
                        cat_title: format!("Unknown Category ({})", id),
                        cat_description: "Category not found".to_string(),
                        cat_tag: "unknown".to_string(),
                        project_id: target_project_id,
                    }
                }
            }
        }
    }
}

/// Returns a DecorateRequirement vector for a given requirement vector
/// This function never fails, but if some requirement data is not found
/// is filled with default value.
pub fn decorate_requirements(reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {
    let mut result = Vec::new();

    for r in reqs {
        let a = DecoratedRequirement {
            req_id: r.req_id,
            req_title: r.req_title,
            req_verification: get_verification_by_id_safe(r.req_verification, r.project_id).verification_name,
            req_description: r.req_description,
            req_current_status: get_status_by_id(r.req_current_status).st_title,
            req_current_status_id: r.req_current_status,  // Add numeric status ID
            req_author: if r.req_author != 0 {
                get_user_by_id(r.req_author).user_name
            } else {
                "".to_string()
            },
            req_reviewer: if r.req_reviewer != 0 {
                get_user_by_id(r.req_reviewer).user_name
            } else {
                "".to_string()
            },
            req_link: r.req_link,
            req_reference: r.req_reference,
            req_category: get_category_by_id_safe(r.req_category, r.project_id).cat_title,
            req_applicability: get_applicability_by_id_safe(r.req_applicability, r.project_id).app_title,
            req_parent_id: r.req_parent,

            req_parent_title: if r.req_parent != 0 {
                match get_requirement_by_id_safe(r.req_parent) {
                    Ok(parent_req) => parent_req.req_title,
                    Err(_) => "[Deleted Parent]".to_string()
                }
            } else {
                "".to_string()
            },
            req_creation_date: r.req_creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_update_date: r.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_deadline_date: r.req_deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_justification: r.req_justification,
            project_id: r.project_id,
        };
        result.push(a);
    }

    result
}

pub fn get_user_by_id(id: i32) -> User {
    use crate::schema::users::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    let result: User = users
        .filter(user_id.eq(id))
        .get_result(connection.as_mut())
        .expect("Error reading table Users");

    result
}

pub fn get_status_by_id(id: i32) -> Status {
    use crate::schema::status::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    let result: Status = status
        .filter(st_id.eq(id))
        .get_result(connection.as_mut())
        .expect("Error reading table Status");

    result
}

/// Struct for verification data that matches the database schema
#[derive(Serialize, Deserialize, Queryable)]
pub struct VerificationData {
    pub verification_id: i32,
    pub verification_name: String,
    pub verification_description: String,
    pub project_id: i32,
}

/// Return all verification types with correct database mapping
pub fn get_verification_all() -> Result<Vec<VerificationData>, String> {
    use crate::schema::verification::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    verification
        .order(verification_id)
        .load::<VerificationData>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying verification: {:?}", _err);
            "Error querying verification from the database".into()
        })
}

pub fn get_verification_by_project(_project_id: i32) -> Result<Vec<VerificationData>, String> {
    use crate::schema::verification::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    verification
        .filter(project_id.eq(_project_id))
        .order(verification_id)
        .load::<VerificationData>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying verification: {:?}", _err);
            "Error querying verification from the database".into()
        })
}

/// Get verification by ID with correct database mapping
pub fn get_verification_by_id(id: i32) -> VerificationData {
    use crate::schema::verification::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    verification
        .filter(verification_id.eq(id))
        .get_result(connection.as_mut())
        .unwrap_or_else(|_| VerificationData {
            verification_id: id,
            verification_name: format!("Unknown Verification ({})", id),
            verification_description: "Verification not found".to_string(),
            project_id: 1,
        })
}

/// Get verification by ID with project filtering and fallback
pub fn get_verification_by_id_safe(id: i32, target_project_id: i32) -> VerificationData {
    use crate::schema::verification::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    // First try to find the verification in the specific project
    match verification
        .filter(verification_id.eq(id))
        .filter(crate::schema::verification::project_id.eq(target_project_id))
        .get_result::<VerificationData>(connection.as_mut()) {
        Ok(result) => result,
        Err(_) => {
            // Fallback: try to find any verification with this ID
            match verification
                .filter(verification_id.eq(id))
                .get_result::<VerificationData>(connection.as_mut()) {
                Ok(result) => result,
                Err(_) => {
                    // Final fallback: return a default verification
                    VerificationData {
                        verification_id: id,
                        verification_name: format!("Unknown Verification ({})", id),
                        verification_description: "Verification not found".to_string(),
                        project_id: target_project_id,
                    }
                }
            }
        }
    }
}

pub fn get_status_name_by_id(id: i32) -> String {
    get_status_by_id(id).st_title
}

pub fn get_requirement_by_id(id: i32) -> Requirement {
    use crate::schema::requirements::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    requirements
        .filter(req_id.eq(id))
        .get_result(connection.as_mut())
        .unwrap_or_else(|_| Requirement {
            req_id: id,
            req_title: format!("Unknown Requirement ({})", id),
            req_description: "Requirement not found".to_string(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "".to_string(),
            req_reference: format!("REQ-UNK-{}", id),
            req_category: 1,
            req_parent: 0,
            req_creation_date: chrono::Utc::now().naive_utc(),
            req_update_date: chrono::Utc::now().naive_utc(),
            req_deadline_date: chrono::Utc::now().naive_utc(),
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        })
}

/// Get requirement by ID with proper error handling
pub fn get_requirement_by_id_safe(id: i32) -> Result<Requirement, String> {
    use crate::schema::requirements::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    match requirements
        .filter(req_id.eq(id))
        .get_result::<Requirement>(connection.as_mut()) {
        Ok(requirement) => Ok(requirement),
        Err(diesel::result::Error::NotFound) => Err(format!("Requirement with ID {} not found", id)),
        Err(e) => Err(format!("Database error: {}", e))
    }
}

pub fn get_requirement_title_by_id(id: i32) -> String {
    match get_requirement_by_id_safe(id) {
        Ok(req) => req.req_title,
        Err(_) => "[Requirement Not Found]".to_string()
    }
}

/// Return all requirements
pub fn get_requirements_all() -> Result<Vec<Requirement>, String> {
    use crate::schema::requirements::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    requirements
        .order(req_id)
        .load::<Requirement>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

pub fn get_tests_all() -> Result<Vec<Test>, String> {
    use crate::schema::tests::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    tests
        .order(test_id)
        .load::<Test>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

pub fn get_users_all() -> Result<Vec<User>, String> {
    use crate::schema::users::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    users
        .order(user_id)
        .load::<User>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}



pub fn get_test_by_id(id: i32) -> Test {
    use crate::schema::tests::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    tests.filter(test_id.eq(id)).get_result(connection.as_mut()).unwrap_or_else(|_| Test {
        test_id: id,
        test_name: format!("Unknown Test ({})", id),
        test_description: "Test not found".to_string(),
        test_source: "Unknown".to_string(),
        test_status: 1,
        test_parent: 0,
        project_id: 1,
    })
}

/// Get test by ID with proper error handling
pub fn get_test_by_id_safe(id: i32) -> Result<Test, String> {
    use crate::schema::tests::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    match tests.filter(test_id.eq(id)).get_result::<Test>(connection.as_mut()) {
        Ok(test) => Ok(test),
        Err(diesel::result::Error::NotFound) => Err(format!("Test with ID {} not found", id)),
        Err(e) => Err(format!("Database error: {}", e))
    }
}

pub fn get_test_status_by_id(id: i32) -> String {
    use crate::schema::tests::dsl::*;
    use crate::models::Status;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    let ts: Test = match tests.filter(test_id.eq(id)).get_result(connection.as_mut()) {
        Ok(test) => test,
        Err(_) => return "[Test Not Found]".to_string()
    };

    let result: Status = match crate::schema::status::dsl::status
        .filter(crate::schema::status::dsl::st_id.eq(ts.test_status))
        .first(connection.as_mut()) {
            Ok(status) => status,
            Err(_) => return "[Status Not Found]".to_string()
        };

    result.st_title
}

pub fn insert_new_requirement(conn: &mut PgConnection, new: &NewRequirement) 
            -> Result<i32, Box<dyn Error>> 
{
    let res:Requirement = diesel::insert_into(crate::schema::requirements::table)
    .values(new)
    .get_result(conn)?;

    Ok(res.req_id)
}

pub fn edit_requirement(
    conn: &mut PgConnection,
    new: &NewRequirement,
) -> Result<bool, Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    let id = new.req_id.unwrap_or(0);

    diesel::update(requirements)
        .filter(req_id.eq(id))
        .set(new)
        .execute(conn)?;

    Ok(true)
}

pub fn delete_requirement(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    let ret_value = diesel::delete(requirements.filter(req_id.eq(id))).execute(conn);

    if ret_value == Ok(1) {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_test(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::tests::dsl::*;

    let ret_value = diesel::delete(tests.filter(test_id.eq(id))).execute(conn);

    if ret_value == Ok(1) {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn delete_user(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let ret_value = diesel::delete(users.filter(user_id.eq(id))).execute(conn);

    if ret_value == Ok(1) {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn insert_new_test(conn: &mut PgConnection, new: &NewTest) -> Result<i32, Box<dyn Error>> {
    let res: Test = diesel::insert_into(crate::schema::tests::table)
        .values(new)
        .get_result(conn)?;

    Ok(res.test_id)
}

pub fn edit_test(conn: &mut PgConnection, new: &NewTest) -> Result<bool, Box<dyn Error>> {
    use crate::schema::tests::dsl::*;

    let id = new.test_id.unwrap_or(0);

    diesel::update(tests)
        .filter(test_id.eq(id))
        .set(new)
        .execute(conn)?;

    Ok(true)
}
pub fn decorate_tests(tests: Vec<Test>) -> Vec<DecoratedTest> {
    let mut result = Vec::new();

    for r in tests {
        let a = DecoratedTest {
            test_id: r.test_id,
            test_name: r.test_name,
            test_description: r.test_description,
            test_source: r.test_source,
            test_status: get_status_by_id(r.test_status).st_title,
            test_status_id: r.test_status,  // Add numeric status ID
            test_parent_id: r.test_parent,
            test_parent_title: if r.test_parent != 0 {
                get_test_by_id(r.test_parent).test_name
            } else {
                "".to_string()
            },
            project_id: r.project_id,
        };
        #[cfg(debug_assertions)]
        println!("Decorate: {:?}", a);
        result.push(a);
    }

    result
}

pub fn insert_new_matrix_item(
    conn: &mut PgConnection,
    new: &NewMatrix,
) -> Result<(), Box<dyn Error>> {
    #[cfg(debug_assertions)]
    println!("Inserting, ({}, {})", new.matrix_req_id, new.matrix_test_id);
    diesel::insert_into(crate::schema::matrix::table)
        .values(new)
        .execute(conn)?;

    Ok(())
}

pub fn get_requirements_for_test(test_id: i32) -> Result<Vec<Requirement>, String> {
    use crate::schema::matrix::dsl::*;
    use crate::schema::requirements::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    let linked_requirements = matrix
        .filter(matrix_test_id.eq(test_id))
        .inner_join(requirements.on(matrix_req_id.eq(req_id)))
        .select((
            req_id,
            req_title,
            req_description,
            req_verification,
            req_current_status,
            req_author,
            req_reviewer,
            req_link,
            req_reference,
            req_category,
            req_parent,
            req_creation_date,
            req_update_date,
            req_deadline_date,
            req_applicability,
            req_justification,
            crate::schema::requirements::project_id,
        ))
        .load::<Requirement>(connection.as_mut())
        .map_err(|_e| format!("Error getting requirements for test: {}", _e))?;

    Ok(linked_requirements)
}

pub fn update_test_requirement_links(
    conn: &mut PgConnection,
    test_id: i32,
    requirement_ids: &[i32],
) -> Result<(), Box<dyn Error>> {
    use crate::schema::matrix::dsl::*;

    // First, delete all existing links for this test
    diesel::delete(matrix.filter(matrix_test_id.eq(test_id)))
        .execute(conn)?;

    // Then, insert the new links
    for req_id in requirement_ids {
        let matrix_item = NewMatrix {
            matrix_req_id: *req_id,
            matrix_test_id: test_id,
            project_id: 1, // Default to project 1 for now
        };
        insert_new_matrix_item(conn, &matrix_item)?;
    }

    Ok(())
}

pub fn insert_new_user(conn: &mut PgConnection, new: &NewUser) -> Result<i32, Box<dyn Error>> {
    let a: User = diesel::insert_into(crate::schema::users::table)
        .values(new)
        .get_result(conn)?;

    #[cfg(debug_assertions)]
    println!("New user id {}", a.user_id);

    Ok(a.user_id)
}

pub fn update_user(conn: &mut PgConnection, user_data: &NewUser) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let user_id_value = user_data.user_id.ok_or("User ID is required")?;

    let result = diesel::update(users.filter(user_id.eq(user_id_value)))
        .set((
            user_name.eq(&user_data.user_name),
            user_username.eq(&user_data.user_username),
            user_email.eq(&user_data.user_email),
            user_level.eq(user_data.user_level),
        ))
        .execute(conn)?;

    Ok(result > 0)
}

pub fn update_user_without_password(conn: &mut PgConnection, user_data: &crate::models::UpdateUser) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let user_id_value = user_data.user_id.ok_or("User ID is required")?;

    let result = diesel::update(users.filter(user_id.eq(user_id_value)))
        .set((
            user_name.eq(&user_data.user_name),
            user_username.eq(&user_data.user_username),
            user_email.eq(&user_data.user_email),
            user_level.eq(user_data.user_level),
        ))
        .execute(conn)?;

    Ok(result > 0)
}

pub fn update_requirement(conn: &mut PgConnection, req: i32) -> Result<(), Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    diesel::update(requirements)
        .filter(req_id.eq(req))
        .set(req_update_date.eq(now))
        .execute(conn)?;

    Ok(())
}

pub fn create_test(conn: &mut PgConnection, new: &NewTest)
            -> Result<i32, Box<dyn Error>>
{
    let res : Test = diesel::insert_into(crate::schema::tests::table)
    .values(new)
    .get_result(conn)?;

    Ok(res.test_id)
}

pub fn create_status(conn: &mut PgConnection, new: &NewStatus)
-> Result<i32, Box<dyn Error>>
{
    let res: Status = diesel::insert_into(crate::schema::status::table)
    .values(new)
    .get_result(conn)?;

    Ok(res.st_id)
}

pub fn create_user(conn: &mut PgConnection, new: &NewUser) -> Result<i32, Box<dyn Error>> {
    let res: User = diesel::insert_into(crate::schema::users::table)
        .values(new)
        .get_result(conn)?;

    Ok(res.user_id)
}

pub fn insert_new_category(conn: &mut PgConnection, new: &NewCategory) -> Result<i32, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let result = diesel::insert_into(categories)
        .values(new)
        .get_result::<Category>(conn)?;

    Ok(result.cat_id)
}

pub fn edit_category(conn: &mut PgConnection, new: &NewCategory) -> Result<bool, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let category_id = new.cat_id.unwrap_or(0);
    if category_id == 0 {
        return Err("Category ID is required for editing".into());
    }

    let updated = diesel::update(categories.filter(cat_id.eq(category_id)))
        .set((
            cat_title.eq(&new.cat_title),
            cat_description.eq(&new.cat_description),
            cat_tag.eq(&new.cat_tag),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn delete_category(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let deleted = diesel::delete(categories.filter(cat_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn generate_pdf_content(
    total_requirements: usize,
    total_tests: usize,
    total_categories: usize,
    total_users: usize,
    coverage_percentage: f64,
    avg_tests_per_requirement: f64,
    covered_requirements: usize,
    total_links: usize,
    requirements_by_status: std::collections::HashMap<String, i32>,
    tests_by_status: std::collections::HashMap<String, i32>,
    requirements_by_category: std::collections::HashMap<String, i32>,
) -> String {
    let mut content = String::new();
    
    // Header
    content.push_str("
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset='utf-8'>
        <title>ReqMan - Project Report</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .header { text-align: center; border-bottom: 2px solid #333; padding-bottom: 20px; margin-bottom: 30px; }
            .section { margin-bottom: 30px; }
            .section h2 { color: #2c3e50; border-bottom: 1px solid #bdc3c7; padding-bottom: 10px; }
            .metric-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }
            .metric-card { background: #f8f9fa; border: 1px solid #dee2e6; border-radius: 8px; padding: 20px; text-align: center; }
            .metric-value { font-size: 2em; font-weight: bold; color: #007bff; margin-bottom: 10px; }
            .metric-label { color: #6c757d; font-size: 0.9em; }
            .chart-container { margin: 20px 0; }
            .status-item { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #eee; }
            .status-name { font-weight: bold; }
            .status-count { color: #007bff; }
            .coverage-bar { background: #e9ecef; height: 20px; border-radius: 10px; overflow: hidden; margin: 10px 0; }
            .coverage-fill { background: #28a745; height: 100%; transition: width 0.3s ease; }
            .footer { margin-top: 40px; text-align: center; color: #6c757d; font-size: 0.8em; }
        </style>
    </head>
    <body>
        <div class='header'>
            <h1>ReqMan Project Report</h1>
            <p>Generated on: ");
    
    content.push_str(&chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
    content.push_str("</p>
        </div>
        
        <div class='section'>
            <h2>Executive Summary</h2>
            <div class='metric-grid'>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&total_requirements.to_string());
    content.push_str("</div>
                    <div class='metric-label'>Total Requirements</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&total_tests.to_string());
    content.push_str("</div>
                    <div class='metric-label'>Total Tests</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&format!("{:.1}%", coverage_percentage));
    content.push_str("</div>
                    <div class='metric-label'>Coverage</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&format!("{:.1}", avg_tests_per_requirement));
    content.push_str("</div>
                    <div class='metric-label'>Avg Tests/Req</div>
                </div>
            </div>
        </div>
        
        <div class='section'>
            <h2>Requirements by Status</h2>");
    
    for (status, count) in requirements_by_status {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", status, count));
    }
    
    content.push_str("
        </div>
        
        <div class='section'>
            <h2>Tests by Status</h2>");
    
    for (status, count) in tests_by_status {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", status, count));
    }
    
    content.push_str("
        </div>
        
        <div class='section'>
            <h2>Requirements by Category</h2>");
    
    for (category, count) in requirements_by_category {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", category, count));
    }
    
    content.push_str(&format!("
        </div>
        
        <div class='section'>
            <h2>Coverage Analysis</h2>
            <p><strong>Covered Requirements:</strong> {} out of {} ({:.1}%)</p>
            <div class='coverage-bar'>
                <div class='coverage-fill' style='width: {:.1}%'></div>
            </div>
            <p><strong>Total Test Links:</strong> {}</p>
            <p><strong>Average Tests per Requirement:</strong> {:.1}</p>
        </div>
        
        <div class='section'>
            <h2>Project Statistics</h2>
            <div class='metric-grid'>
                <div class='metric-card'>
                    <div class='metric-value'>{}</div>
                    <div class='metric-label'>Categories</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>{}</div>
                    <div class='metric-label'>Users</div>
                </div>
            </div>
        </div>
        
        <div class='footer'>
            <p>This report was generated automatically by ReqMan</p>
        </div>
    </body>
    </html>", 
        covered_requirements, 
        total_requirements, 
        coverage_percentage,
        coverage_percentage,
        total_links,
        avg_tests_per_requirement,
        total_categories,
        total_users
    ));
    
    content
}

pub fn generate_pdf_from_html(_html_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use printpdf::*;
    use std::io::{Cursor, BufWriter};
    
    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page1 = doc.get_page(page1);
    let layer1 = page1.get_layer(layer1);
    
    // Add title
    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load title font: {}", e))?;
    let title_font_size = 18.0;
    let title_text = "ReqMan Project Report";
    
    layer1.use_text(
        title_text,
        title_font_size,
        Mm(105.0),
        Mm(280.0),
        &title_font,
    );
    
    // Add generation date
    let date_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load date font: {}", e))?;
    let date_font_size = 10.0;
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    layer1.use_text(
        &format!("Generated on: {}", current_date),
        date_font_size,
        Mm(20.0),
        Mm(270.0),
        &date_font,
    );
    
    // Add content sections
    let content_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load content font: {}", e))?;
    let content_font_size = 12.0;
    let mut y_position = Mm(250.0);
    
    // Parse HTML content to extract meaningful data
    // For now, we'll create a simple structured report
    let sections = vec![
        ("Project Overview", "This report contains project metrics and statistics"),
        ("Requirements", "Total requirements and their status distribution"),
        ("Tests", "Total tests and their status distribution"),
        ("Coverage", "Requirements coverage analysis"),
        ("Categories", "Requirements categorized by type"),
    ];
    
    for (section_title, section_desc) in sections {
        // Section title
        let section_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
            .map_err(|e| format!("Failed to load section font: {}", e))?;
        layer1.use_text(
            section_title,
            content_font_size + 2.0,
            Mm(20.0),
            y_position,
            &section_font,
        );
        y_position -= Mm(8.0);
        
        // Section description
        layer1.use_text(
            section_desc,
            content_font_size,
            Mm(20.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(15.0);
        
        // Add some spacing between sections
        if y_position < Mm(50.0) {
            // If we're running out of space, add a new page
            let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Page 2");
            let page2 = doc.get_page(page2);
            let layer2 = page2.get_layer(layer2);
            
            // Add page number
            layer2.use_text(
                "Page 2",
                content_font_size,
                Mm(20.0),
                Mm(20.0),
                &content_font,
            );
            
            y_position = Mm(280.0);
        }
    }
    
    // Add footer
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load footer font: {}", e))?;
    layer1.use_text(
        "Generated by ReqMan - Requirements Management System",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &footer_font,
    );
    
    // Write PDF to memory
    let cursor = Cursor::new(Vec::new());
    let mut buf_writer = BufWriter::new(cursor);
    doc.save(&mut buf_writer)?;
    
    let cursor = buf_writer.into_inner()?;
    Ok(cursor.into_inner())
}

pub fn generate_pdf_report_data(
    total_requirements: usize,
    total_tests: usize,
    total_categories: usize,
    total_users: usize,
    coverage_percentage: f64,
    avg_tests_per_requirement: f64,
    covered_requirements: usize,
    total_links: usize,
    requirements_by_status: std::collections::HashMap<String, i32>,
    tests_by_status: std::collections::HashMap<String, i32>,
    _requirements_by_category: std::collections::HashMap<String, i32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use printpdf::*;
    use std::io::{Cursor, BufWriter};
    
    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page1 = doc.get_page(page1);
    let layer1 = page1.get_layer(layer1);
    
    // Add title
    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load title font: {}", e))?;
    let title_font_size = 18.0;
    let title_text = "ReqMan Project Report";
    
    layer1.use_text(
        title_text,
        title_font_size,
        Mm(105.0),
        Mm(105.0),
        &title_font,
    );
    
    // Add generation date
    let date_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load date font: {}", e))?;
    let date_font_size = 10.0;
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    layer1.use_text(
        &format!("Generated on: {}", current_date),
        date_font_size,
        Mm(20.0),
        Mm(270.0),
        &date_font,
    );
    
    // Add metrics overview
    let metrics_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load metrics font: {}", e))?;
    let metrics_font_size = 14.0;
    let content_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load content font: {}", e))?;
    let content_font_size = 12.0;
    let mut y_position = Mm(250.0);
    
    // Project Overview Section
    layer1.use_text(
        "Project Overview",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    let overview_items = vec![
        format!("Total Requirements: {}", total_requirements),
        format!("Total Tests: {}", total_tests),
        format!("Total Categories: {}", total_categories),
        format!("Total Users: {}", total_users),
    ];
    
    for item in overview_items {
        layer1.use_text(
            &item,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
    }
    
    y_position -= Mm(8.0);
    
    // Coverage Analysis Section
    layer1.use_text(
        "Coverage Analysis",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    let coverage_items = vec![
        format!("Covered Requirements: {} out of {} ({:.1}%)", 
                covered_requirements, total_requirements, coverage_percentage),
        format!("Total Test Links: {}", total_links),
        format!("Average Tests per Requirement: {:.1}", avg_tests_per_requirement),
    ];
    
    for item in coverage_items {
        layer1.use_text(
            &item,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
    }
    
    y_position -= Mm(8.0);
    
    // Requirements by Status Section
    layer1.use_text(
        "Requirements by Status",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    for (status, count) in &requirements_by_status {
        let status_text = format!("{}: {}", status, count);
        layer1.use_text(
            &status_text,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
        
        if y_position < Mm(50.0) {
            // Add new page if needed
            let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Page 2");
            let page2 = doc.get_page(page2);
            let layer2 = page2.get_layer(layer2);
            
            // Continue on new page
            y_position = Mm(280.0);
            layer2.use_text(
                "Requirements by Status (continued)",
                metrics_font_size,
                Mm(20.0),
                y_position,
                &metrics_font,
            );
            y_position -= Mm(12.0);
        }
    }
    
    y_position -= Mm(8.0);
    
    // Tests by Status Section
    if y_position > Mm(60.0) {
        layer1.use_text(
            "Tests by Status",
            metrics_font_size,
            Mm(20.0),
            y_position,
            &metrics_font,
        );
        y_position -= Mm(12.0);
        
        for (status, count) in &tests_by_status {
            let status_text = format!("{}: {}", status, count);
            layer1.use_text(
                &status_text,
                content_font_size,
                Mm(25.0),
                y_position,
                &content_font,
            );
            y_position -= Mm(8.0);
        }
    } else {
        // Add new page for tests section
        let (page3, layer3) = doc.add_page(Mm(210.0), Mm(297.0), "Page 3");
        let page3 = doc.get_page(page3);
        let layer3 = page3.get_layer(layer3);
        
        layer3.use_text(
            "Tests by Status",
            metrics_font_size,
            Mm(20.0),
            Mm(280.0),
            &metrics_font,
        );
        
        let mut test_y = Mm(268.0);
        for (status, count) in &tests_by_status {
            let status_text = format!("{}: {}", status, count);
            layer3.use_text(
                &status_text,
                content_font_size,
                Mm(25.0),
                test_y,
                &content_font,
            );
            test_y -= Mm(8.0);
        }
    }
    
    // Add footer to all pages
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load footer font: {}", e))?;
    layer1.use_text(
        "Generated by ReqMan - Requirements Management System",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &footer_font,
    );
    
    // Write PDF to memory
    let cursor = Cursor::new(Vec::new());
    let mut buf_writer = BufWriter::new(cursor);
    doc.save(&mut buf_writer)?;
    
    let cursor = buf_writer.into_inner()?;
    Ok(cursor.into_inner())
}

// Project management functions
pub fn get_projects_all() -> Result<Vec<Project>, String> {
    use crate::schema::projects::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    projects
        .load::<Project>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying projects from the database".into()
        })
}

pub fn get_project_by_id(project_id_param: i32) -> Project {
    use crate::schema::projects::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    projects
        .filter(project_id.eq(project_id_param))
        .first::<Project>(connection.as_mut())
        .expect("Error loading project")
}

pub fn insert_new_project(conn: &mut PgConnection, new: &NewProject) -> Result<i32, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;
    
    let result = diesel::insert_into(projects)
        .values(new)
        .get_result::<Project>(conn)?;
    
    Ok(result.project_id)
}

pub fn edit_project(conn: &mut PgConnection, project_id_param: i32, update: &UpdateProject) -> Result<bool, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;
    
    let updated = diesel::update(projects.filter(project_id.eq(project_id_param)))
        .set((
            project_name.eq(&update.project_name),
            project_description.eq(&update.project_description),
            project_status.eq(&update.project_status),
            project_owner_id.eq(&update.project_owner_id),
            project_update_date.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn)?;
    
    Ok(updated > 0)
}

pub fn delete_project(conn: &mut PgConnection, project_id_param: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;
    
    let deleted = diesel::delete(projects.filter(project_id.eq(project_id_param)))
        .execute(conn)?;
    
    Ok(deleted > 0)
}

pub fn insert_new_applicability(conn: &mut PgConnection, new: &NewApplicability) -> Result<i32, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let result = diesel::insert_into(applicability)
        .values(new)
        .get_result::<Applicability>(conn)?;

    Ok(result.app_id)
}

pub fn edit_applicability(conn: &mut PgConnection, new: &NewApplicability) -> Result<bool, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let applicability_id = new.app_id.unwrap_or(0);
    if applicability_id == 0 {
        return Err("Applicability ID is required for editing".into());
    }

    let updated = diesel::update(applicability.filter(app_id.eq(applicability_id)))
        .set((
            app_title.eq(&new.app_title),
            app_description.eq(&new.app_description),
            app_tag.eq(&new.app_tag),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn delete_applicability(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let deleted = diesel::delete(applicability.filter(app_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn get_linked_tests_for_requirement(conn: &mut PgConnection, req_id: i32) -> Result<Vec<DecoratedTest>, Box<dyn Error>> {
    use crate::schema::matrix::dsl::*;
    use crate::schema::tests::dsl::*;

    // Get test IDs linked to this requirement
    let linked_test_ids: Vec<i32> = matrix
        .filter(matrix_req_id.eq(req_id))
        .select(matrix_test_id)
        .load(conn)?;

    if linked_test_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Get the actual test data for these IDs
    let tests_data: Vec<Test> = tests
        .filter(test_id.eq_any(linked_test_ids))
        .load(conn)?;

    // Decorate the tests with status names and parent titles
    let mut decorated_tests = Vec::new();
    for test in tests_data {
        let status_name = get_status_name_by_id(test.test_status);
        let parent_title = if test.test_parent > 0 {
            get_test_by_id(test.test_parent).test_name
        } else {
            "None".to_string()
        };

        decorated_tests.push(DecoratedTest {
            test_id: test.test_id,
            test_name: test.test_name,
            test_description: test.test_description,
            test_source: test.test_source,
            test_status: status_name,
            test_status_id: test.test_status,  // Add numeric status ID
            test_parent_id: test.test_parent,
            test_parent_title: parent_title,
            project_id: test.project_id,
        });
    }

    Ok(decorated_tests)
}

pub fn establish_connection() -> diesel::PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn get_selected_project_id(cookies: &rocket::http::CookieJar<'_>) -> Option<i32> {
    cookies.get("selected_project_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok())
}

pub fn get_projects_for_nav() -> Result<Vec<Project>, String> {
    get_projects_all()
}

pub fn get_requirements_by_project(_project_id: i32) -> Result<Vec<Requirement>, String> {
    use crate::schema::requirements::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    requirements
        .filter(crate::schema::requirements::project_id.eq(_project_id))
        .load::<Requirement>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying requirements from the database".into()
        })
}

pub fn get_tests_by_project(_project_id: i32) -> Result<Vec<Test>, String> {
    use crate::schema::tests::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    tests
        .filter(crate::schema::tests::project_id.eq(_project_id))
        .load::<Test>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying tests from the database".into()
        })
}

pub fn get_categories_by_project(_project_id: i32) -> Result<Vec<Category>, String> {
    use crate::schema::categories::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    categories
        .filter(crate::schema::categories::project_id.eq(_project_id))
        .load::<Category>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying categories from the database".into()
        })
}

pub fn get_applicability_by_project(_project_id: i32) -> Result<Vec<Applicability>, String> {
    use crate::schema::applicability::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    applicability
        .filter(crate::schema::applicability::project_id.eq(_project_id))
        .load::<Applicability>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying applicability from the database".into()
        })
}

pub fn get_matrix_by_project(_project_id: i32) -> Result<Vec<Matrix>, String> {
    use crate::schema::matrix::dsl::*;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    matrix
        .filter(crate::schema::matrix::project_id.eq(_project_id))
        .load::<Matrix>(connection.as_mut())
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying matrix from the database".into()
        })
}

/// Filter requirements by status, verification mode, and category, then sort by reference
pub fn filter_requirements(
    requirements: Vec<Requirement>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
) -> Vec<Requirement> {
    let mut filtered_requirements: Vec<Requirement> = requirements
        .into_iter()
        .filter(|req| {
            let status_match = status_filter.map_or(true, |status_id| req.req_current_status == status_id);
            let verification_match = verification_filter.map_or(true, |verification_id| req.req_verification == verification_id);
            let category_match = category_filter.map_or(true, |category_id| req.req_category == category_id);
            
            status_match && verification_match && category_match
        })
        .collect();
    
    // Sort by reference (empty references come last)
    filtered_requirements.sort_by(|a, b| {
        match (a.req_reference.is_empty(), b.req_reference.is_empty()) {
            (false, false) => a.req_reference.cmp(&b.req_reference),
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            (true, true) => a.req_id.cmp(&b.req_id), // Fallback to ID if both are empty
        }
    });
    
    filtered_requirements
}

/// Filter tests by status, verification mode, and category
pub fn filter_tests(
    tests: Vec<Test>,
    status_filter: Option<i32>,
    _verification_filter: Option<i32>,
    _category_filter: Option<i32>,
) -> Vec<Test> {
    tests
        .into_iter()
        .filter(|test| {
            let status_match = status_filter.map_or(true, |status_id| test.test_status == status_id);
            // Note: Tests don't have direct verification or category fields, 
            // but we can filter by status for now
            status_match
        })
        .collect()
}

/// Generate automatic reference code for a requirement based on category tag and project
pub fn generate_requirement_reference(category_id: i32, project_id: i32) -> Result<String, Box<dyn Error>> {
    use crate::schema::categories;
    use crate::schema::requirements;
    
    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));
    
    // Get the category to find its tag
    let category = categories::table
        .filter(categories::cat_id.eq(category_id))
        .first::<Category>(connection.as_mut())
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")))?;
    
    // Count existing requirements with the same category and project
    let existing_count = requirements::table
        .filter(requirements::req_category.eq(category_id))
        .filter(requirements::project_id.eq(project_id))
        .count()
        .get_result::<i64>(connection.as_mut())
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Database error")))?;
    
    // Generate reference: REQ-{CATEGORY_TAG}-{NEXT_NUMBER}
    let next_number = existing_count + 1;
    let reference = format!("REQ-{}-{}", category.cat_tag, next_number);
    
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Requirement, Test};
    use chrono::NaiveDate;

    #[test]
    fn hash_and_verify_password() {
        let password = "s3cr3t";
        let hashed = hash_password(password).expect("hashing failed");
        assert!(verify_password(password, &hashed).unwrap());
    }

    #[test]
    fn verify_password_rejects_invalid_password() {
        let password = "correct";
        let hashed = hash_password(password).expect("hashing failed");
        assert!(!verify_password("wrong", &hashed).unwrap());
    }

    fn dummy_datetime() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap()
    }

    fn sample_requirement(
        id: i32,
        status: i32,
        verification: i32,
        category: i32,
        reference: &str,
    ) -> Requirement {
        Requirement {
            req_id: id,
            req_title: format!("Req {}", id),
            req_description: String::new(),
            req_verification: verification,
            req_current_status: status,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: reference.to_string(),
            req_category: category,
            req_parent: 0,
            req_creation_date: dummy_datetime(),
            req_update_date: dummy_datetime(),
            req_deadline_date: dummy_datetime(),
            req_applicability: 0,
            req_justification: None,
            project_id: 0,
        }
    }

    #[test]
    fn filter_requirements_filters_and_sorts() {
        let reqs = vec![
            sample_requirement(1, 1, 1, 1, "REF-A"),
            sample_requirement(2, 1, 2, 1, ""),
            sample_requirement(3, 2, 1, 2, "REF-B"),
        ];

        let filtered = filter_requirements(reqs.clone(), Some(1), None, None);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].req_id, 1);
        assert_eq!(filtered[1].req_id, 2);

        let filtered2 = filter_requirements(reqs.clone(), None, Some(1), Some(1));
        assert_eq!(filtered2.len(), 1);
        assert_eq!(filtered2[0].req_id, 1);

        let filtered3 = filter_requirements(reqs, None, None, None);
        assert_eq!(filtered3[0].req_id, 1);
        assert_eq!(filtered3[1].req_id, 3);
        assert_eq!(filtered3[2].req_id, 2);
    }

    #[test]
    fn filter_tests_filters_by_status() {
        let only_status1 = filter_tests(
            vec![
                Test { test_id: 1, test_name: "T1".into(), test_description: String::new(), test_source: String::new(), test_status: 1, test_parent: 0, project_id: 0 },
                Test { test_id: 2, test_name: "T2".into(), test_description: String::new(), test_source: String::new(), test_status: 2, test_parent: 0, project_id: 0 },
            ],
            Some(1), None, None
        );
        assert_eq!(only_status1.len(), 1);
        assert_eq!(only_status1[0].test_id, 1);

        let all = filter_tests(
            vec![
                Test { test_id: 1, test_name: "T1".into(), test_description: String::new(), test_source: String::new(), test_status: 1, test_parent: 0, project_id: 0 },
                Test { test_id: 2, test_name: "T2".into(), test_description: String::new(), test_source: String::new(), test_status: 2, test_parent: 0, project_id: 0 },
            ],
            None, None, None
        );
        assert_eq!(all.len(), 2);
    }
}


