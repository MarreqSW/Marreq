use crate::db::{get_pooled_connection_wrapper, PooledConnectionWrapper};
use crate::models::*;
use crate::schema::*;
use diesel::prelude::*;
use std::error::Error;

/// Get all projects using connection pool
pub fn get_projects_all_pooled() -> Result<Vec<Project>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    projects::table
        .load::<Project>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get requirements by project using connection pool
pub fn get_requirements_by_project_pooled(project_id: i32) -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    requirements::table
        .filter(requirements::project_id.eq(project_id))
        .load::<Requirement>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get tests by project using connection pool
pub fn get_tests_by_project_pooled(project_id: i32) -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    tests::table
        .filter(tests::project_id.eq(project_id))
        .load::<Test>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get categories by project using connection pool
pub fn get_categories_by_project_pooled(project_id: i32) -> Result<Vec<Category>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    categories::table
        .filter(categories::project_id.eq(project_id))
        .load::<Category>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get applicability by project using connection pool
pub fn get_applicability_by_project_pooled(project_id: i32) -> Result<Vec<Applicability>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    applicability::table
        .filter(applicability::project_id.eq(project_id))
        .load::<Applicability>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get matrix by project using connection pool
pub fn get_matrix_by_project_pooled(project_id: i32) -> Result<Vec<Matrix>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    matrix::table
        .filter(matrix::project_id.eq(project_id))
        .load::<Matrix>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all statuses using connection pool
pub fn get_status_all_pooled() -> Result<Vec<Status>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    status::table
        .load::<Status>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all categories using connection pool
pub fn get_categories_all_pooled() -> Result<Vec<Category>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    categories::table
        .load::<Category>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all applicability using connection pool
pub fn get_applicability_all_pooled() -> Result<Vec<Applicability>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    applicability::table
        .load::<Applicability>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all tests using connection pool
pub fn get_tests_all_pooled() -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    tests::table
        .load::<Test>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all users using connection pool
pub fn get_users_all_pooled() -> Result<Vec<User>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    users::table
        .load::<User>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all requirements using connection pool
pub fn get_requirements_all_pooled() -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    requirements::table
        .load::<Requirement>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get user by ID using connection pool
pub fn get_user_by_id_pooled(user_id: i32) -> Result<User, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    users::table
        .find(user_id)
        .first::<User>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get requirement by ID using connection pool
pub fn get_requirement_by_id_pooled(req_id: i32) -> Result<Requirement, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    requirements::table
        .find(req_id)
        .first::<Requirement>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get test by ID using connection pool
pub fn get_test_by_id_pooled(test_id: i32) -> Result<Test, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    tests::table
        .find(test_id)
        .first::<Test>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get category by ID using connection pool
pub fn get_category_by_id_pooled(cat_id: i32) -> Result<Category, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    categories::table
        .find(cat_id)
        .first::<Category>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get status by ID using connection pool
pub fn get_status_by_id_pooled(status_id: i32) -> Result<Status, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    status::table
        .find(status_id)
        .first::<Status>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get project by ID using connection pool
pub fn get_project_by_id_pooled(project_id: i32) -> Result<Project, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    projects::table
        .find(project_id)
        .first::<Project>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get verification by project using connection pool
pub fn get_verification_by_project_pooled(project_id: i32) -> Result<Vec<Verification>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    verification::table
        .filter(verification::project_id.eq(project_id))
        .load::<Verification>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all verification methods using connection pool
pub fn get_verification_all_pooled() -> Result<Vec<Verification>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    verification::table
        .load::<Verification>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get linked tests for requirement using connection pool
pub fn get_linked_tests_for_requirement_pooled(req_id: i32) -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    // Get test IDs linked to this requirement
    let linked_test_ids: Vec<i32> = matrix::table
        .filter(matrix::matrix_req_id.eq(req_id))
        .select(matrix::matrix_test_id)
        .load(conn.as_mut())?;

    if linked_test_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Get the actual test data for these IDs
    let tests_data: Vec<Test> = tests::table
        .filter(tests::test_id.eq_any(linked_test_ids))
        .load(conn.as_mut())?;

    Ok(tests_data)
}

/// Get requirements for test using connection pool
pub fn get_requirements_for_test_pooled(test_id: i32) -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    // Get requirement IDs linked to this test
    let linked_req_ids: Vec<i32> = matrix::table
        .filter(matrix::matrix_test_id.eq(test_id))
        .select(matrix::matrix_req_id)
        .load(conn.as_mut())?;

    if linked_req_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Get the actual requirement data for these IDs
    let requirements_data: Vec<Requirement> = requirements::table
        .filter(requirements::req_id.eq_any(linked_req_ids))
        .load(conn.as_mut())?;

    Ok(requirements_data)
}

/// Get status name by ID using connection pool
pub fn get_status_name_by_id_pooled(status_id: i32) -> Result<String, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    let status: Status = status::table
        .find(status_id)
        .first(conn.as_mut())?;
    
    Ok(status.st_title)
}

/// Get category by ID with safe fallback using connection pool
pub fn get_category_by_id_pooled_safe(id: i32) -> Category {
    match get_category_by_id_pooled(id) {
        Ok(category) => category,
        Err(_) => Category {
            cat_id: id,
            cat_title: format!("Unknown Category ({})", id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 0,
        }
    }
}

/// Get user by ID with safe fallback using connection pool
pub fn get_user_by_id_pooled_safe(id: i32) -> User {
    match get_user_by_id_pooled(id) {
        Ok(user) => user,
        Err(_) => User {
            user_id: id,
            user_username: format!("unknown_user_{}", id),
            user_name: format!("Unknown User ({})", id),
            user_email: format!("unknown{}@example.com", id),
            user_level: 1,
            user_creation_date: chrono::Utc::now().naive_utc(),
            user_last_login: chrono::Utc::now().naive_utc(),
            user_password: "".to_string(),
            project_id: None,
            is_admin: false,
        }
    }
}

/// Get requirement by ID with safe fallback using connection pool
pub fn get_requirement_by_id_pooled_safe(id: i32) -> Requirement {
    match get_requirement_by_id_pooled(id) {
        Ok(requirement) => requirement,
        Err(_) => Requirement {
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
        }
    }
}

/// Get test by ID with safe fallback using connection pool
pub fn get_test_by_id_pooled_safe(id: i32) -> Test {
    match get_test_by_id_pooled(id) {
        Ok(test) => test,
        Err(_) => Test {
            test_id: id,
            test_name: format!("Unknown Test ({})", id),
            test_description: "Test not found".to_string(),
            test_source: "Unknown".to_string(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        }
    }
}

/// Get project by ID with safe fallback using connection pool
pub fn get_project_by_id_pooled_safe(project_id: i32) -> Project {
    match get_project_by_id_pooled(project_id) {
        Ok(project) => project,
        Err(_) => Project {
            project_id: 0,
            project_name: "Unknown Project".to_string(),
            project_description: Some("Unknown project".to_string()),
            project_creation_date: Some(chrono::Utc::now().naive_utc()),
            project_update_date: Some(chrono::Utc::now().naive_utc()),
            project_status: Some("Unknown".to_string()),
            project_owner_id: Some(0),
        }
    }
}

/// Get verification by ID using connection pool
pub fn get_verification_by_id_pooled(verification_id: i32) -> Result<Verification, Box<dyn Error>> {
    let mut conn = get_pooled_connection_wrapper()?;
    
    verification::table
        .find(verification_id)
        .first::<Verification>(conn.as_mut())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get pooled connection for operations that need it
pub fn get_pooled_connection_for_operations() -> Result<PooledConnectionWrapper, Box<dyn Error>> {
    get_pooled_connection_wrapper()
}
