use crate::db::get_pooled_connection;
use crate::models::*;
use crate::schema::*;
use diesel::prelude::*;
use std::error::Error;

/// Get all projects using connection pool
pub fn get_projects_all_pooled() -> Result<Vec<Project>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    projects::table
        .load::<Project>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get requirements by project using connection pool
pub fn get_requirements_by_project_pooled(project_id: i32) -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    requirements::table
        .filter(requirements::project_id.eq(project_id))
        .load::<Requirement>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get tests by project using connection pool
pub fn get_tests_by_project_pooled(project_id: i32) -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    tests::table
        .filter(tests::project_id.eq(project_id))
        .load::<Test>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get categories by project using connection pool
pub fn get_categories_by_project_pooled(project_id: i32) -> Result<Vec<Category>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    categories::table
        .filter(categories::project_id.eq(project_id))
        .load::<Category>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get applicability by project using connection pool
pub fn get_applicability_by_project_pooled(project_id: i32) -> Result<Vec<Applicability>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    applicability::table
        .filter(applicability::project_id.eq(project_id))
        .load::<Applicability>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get matrix by project using connection pool
pub fn get_matrix_by_project_pooled(project_id: i32) -> Result<Vec<Matrix>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    matrix::table
        .filter(matrix::project_id.eq(project_id))
        .load::<Matrix>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all statuses using connection pool
pub fn get_status_all_pooled() -> Result<Vec<Status>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    status::table
        .load::<Status>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all categories using connection pool
pub fn get_categories_all_pooled() -> Result<Vec<Category>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    categories::table
        .load::<Category>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all applicability using connection pool
pub fn get_applicability_all_pooled() -> Result<Vec<Applicability>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    applicability::table
        .load::<Applicability>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all tests using connection pool
pub fn get_tests_all_pooled() -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    tests::table
        .load::<Test>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all users using connection pool
pub fn get_users_all_pooled() -> Result<Vec<User>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    users::table
        .load::<User>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all requirements using connection pool
pub fn get_requirements_all_pooled() -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    requirements::table
        .load::<Requirement>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get user by ID using connection pool
pub fn get_user_by_id_pooled(user_id: i32) -> Result<User, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    users::table
        .find(user_id)
        .first::<User>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get requirement by ID using connection pool
pub fn get_requirement_by_id_pooled(req_id: i32) -> Result<Requirement, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    requirements::table
        .find(req_id)
        .first::<Requirement>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get test by ID using connection pool
pub fn get_test_by_id_pooled(test_id: i32) -> Result<Test, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    tests::table
        .find(test_id)
        .first::<Test>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get category by ID using connection pool
pub fn get_category_by_id_pooled(cat_id: i32) -> Result<Category, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    categories::table
        .find(cat_id)
        .first::<Category>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get status by ID using connection pool
pub fn get_status_by_id_pooled(status_id: i32) -> Result<Status, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    status::table
        .find(status_id)
        .first::<Status>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get project by ID using connection pool
pub fn get_project_by_id_pooled(project_id: i32) -> Result<Project, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    projects::table
        .find(project_id)
        .first::<Project>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get verification by project using connection pool
pub fn get_verification_by_project_pooled(project_id: i32) -> Result<Vec<Verification>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    verification::table
        .filter(verification::project_id.eq(project_id))
        .load::<Verification>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get all verification types using connection pool
pub fn get_verification_all_pooled() -> Result<Vec<Verification>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    verification::table
        .load::<Verification>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get linked tests for requirement using connection pool
pub fn get_linked_tests_for_requirement_pooled(req_id: i32) -> Result<Vec<Test>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    use crate::schema::matrix::dsl::*;
    
    let test_ids: Vec<i32> = matrix
        .filter(matrix_req_id.eq(req_id))
        .select(matrix_test_id)
        .load::<i32>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    
    if test_ids.is_empty() {
        return Ok(Vec::new());
    }
    
    tests::table
        .filter(tests::test_id.eq_any(test_ids))
        .load::<Test>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get requirements for test using connection pool
pub fn get_requirements_for_test_pooled(test_id: i32) -> Result<Vec<Requirement>, Box<dyn Error>> {
    let mut conn = get_pooled_connection()?;
    
    use crate::schema::matrix::dsl::*;
    
    let req_ids: Vec<i32> = matrix
        .filter(matrix_test_id.eq(test_id))
        .select(matrix_req_id)
        .load::<i32>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    
    if req_ids.is_empty() {
        return Ok(Vec::new());
    }
    
    requirements::table
        .filter(requirements::req_id.eq_any(req_ids))
        .load::<Requirement>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get status name by ID using connection pool
pub fn get_status_name_by_id_pooled(status_id: i32) -> String {
    match get_status_by_id_pooled(status_id) {
        Ok(status) => status.st_title,
        Err(_) => "Unknown".to_string()
    }
}

/// Get category by ID using connection pool (with error handling)
pub fn get_category_by_id_pooled_safe(id: i32) -> Category {
    match get_category_by_id_pooled(id) {
        Ok(category) => category,
        Err(_) => Category {
            cat_id: 0,
            cat_title: "Unknown".to_string(),
            cat_description: "Unknown category".to_string(),
            cat_tag: "UNK".to_string(),
            project_id: 0,
        }
    }
}

/// Get user by ID using connection pool (with error handling)
pub fn get_user_by_id_pooled_safe(id: i32) -> User {
    match get_user_by_id_pooled(id) {
        Ok(user) => user,
        Err(_) => User {
            user_id: 0,
            user_username: "Unknown".to_string(),
            user_name: "Unknown User".to_string(),
            user_email: "unknown@example.com".to_string(),
            user_level: 0,
            user_creation_date: chrono::Utc::now().naive_utc(),
            user_last_login: chrono::Utc::now().naive_utc(),
            user_password: "".to_string(),
            project_id: None,
            is_admin: false,
        }
    }
}

/// Get requirement by ID using connection pool (with error handling)
pub fn get_requirement_by_id_pooled_safe(id: i32) -> Requirement {
    match get_requirement_by_id_pooled(id) {
        Ok(req) => req,
        Err(_) => Requirement {
            req_id: 0,
            req_title: "Unknown".to_string(),
            req_description: "Unknown requirement".to_string(),
            req_justification: None,
            req_reference: "".to_string(),
            req_link: "".to_string(),
            req_author: 0,
            req_reviewer: 0,
            req_category: 0,
            req_applicability: 0,
            req_current_status: 0,
            req_verification: 0,
            req_parent: 0,
            req_creation_date: chrono::Utc::now().naive_utc(),
            req_update_date: chrono::Utc::now().naive_utc(),
            req_deadline_date: chrono::Utc::now().naive_utc(),
            project_id: 0,
        }
    }
}

/// Get test by ID using connection pool (with error handling)
pub fn get_test_by_id_pooled_safe(id: i32) -> Test {
    match get_test_by_id_pooled(id) {
        Ok(test) => test,
        Err(_) => Test {
            test_id: 0,
            test_name: "Unknown".to_string(),
            test_description: "Unknown test".to_string(),
            test_source: "".to_string(),
            test_status: 0,
            test_parent: 0,
            project_id: 0,
        }
    }
}

/// Get project by ID using connection pool (with error handling)
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
    let mut conn = get_pooled_connection()?;
    
    verification::table
        .find(verification_id)
        .first::<Verification>(&mut conn)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Get pooled connection for operations that need it
pub fn get_pooled_connection_for_operations() -> Result<diesel::pg::PgConnection, Box<dyn Error>> {
    // For now, return a new direct connection since we can't easily convert pooled to direct
    use crate::db::get_connection;
    get_connection()
}
