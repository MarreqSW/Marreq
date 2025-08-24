use crate::models::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

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

pub fn get_user_by_id(id: i32) -> User {
    use crate::schema::users::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    users
        .filter(user_id.eq(id))
        .get_result(connection.as_mut())
        .expect("Error reading table Users")
}

pub fn get_status_by_id(id: i32) -> Status {
    use crate::schema::status::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    status
        .filter(st_id.eq(id))
        .get_result(connection.as_mut())
        .expect("Error reading table Status")
}

/// Struct for verification data that matches the database schema
#[derive(Serialize, Deserialize, Queryable)]
pub struct VerificationData {
    pub verification_id: i32,
    pub verification_name: String,
    pub verification_description: String,
    pub project_id: i32,
}

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
            "Error querying users from the database".into()
        })
}

pub fn get_test_by_id(id: i32) -> Test {
    use crate::schema::tests::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    tests
        .filter(test_id.eq(id))
        .get_result(connection.as_mut())
        .expect("Error reading table Tests")
}

pub fn get_test_by_id_safe(id: i32) -> Result<Test, String> {
    use crate::schema::tests::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    match tests.filter(test_id.eq(id)).get_result::<Test>(connection.as_mut()) {
        Ok(test) => Ok(test),
        Err(diesel::result::Error::NotFound) => Err(format!("Test with ID {} not found", id)),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

pub fn get_test_status_by_id(id: i32) -> String {
    use crate::schema::tests::dsl::*;
    use crate::models::Status;

    let mut connection = crate::db::get_connection_pooled_safe()
        .unwrap_or_else(|_| panic!("Failed to get database connection"));

    let ts: Test = match tests.filter(test_id.eq(id)).get_result(connection.as_mut()) {
        Ok(test) => test,
        Err(_) => return "[Test Not Found]".to_string(),
    };

    let result: Status = match crate::schema::status::dsl::status
        .filter(crate::schema::status::dsl::st_id.eq(ts.test_status))
        .first(connection.as_mut()) {
            Ok(status) => status,
            Err(_) => return "[Status Not Found]".to_string(),
        };

    result.st_title
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
