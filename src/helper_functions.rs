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
    
    let connection = &mut establish_connection();
    
    let user = users
        .filter(user_username.eq(username))
        .first::<User>(connection)
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
    
    let connection = &mut establish_connection();
    
    // Get the user
    let user = users
        .filter(user_id.eq(user_id))
        .first::<User>(connection)
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
                .execute(connection)
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

    let connection = &mut establish_connection();

    status
        .order(st_id)
        .get_results(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

/// Returns the categories list
pub fn get_categories_all() -> Result<Vec<Category>, String> {
    use crate::schema::categories::dsl::*;

    let connection = &mut establish_connection();

    categories
        .order(cat_id)
        .get_results(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

pub fn get_applicability_all() -> Result<Vec<Applicability>, String> {
    use crate::schema::applicability::dsl::*;

    let connection = &mut establish_connection();

    applicability
        .order(app_id)
        .get_results(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying applicability from the database".into()
        })
}

pub fn get_applicability_by_id(id: i32) -> Applicability {
    use crate::schema::applicability::dsl::*;

    let connection = &mut establish_connection();

    applicability
        .filter(app_id.eq(id))
        .get_result(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying applicability from the database".into()
        })
        .unwrap()
}

pub fn get_category_by_id(id: i32) -> Category {
    use crate::schema::categories::dsl::*;

    let connection = &mut establish_connection();

    categories
        .filter(cat_id.eq(id))
        .get_result(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
        .unwrap()
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
            req_verification: get_verification_by_id(r.req_verification).verification_name,
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
            req_category: get_category_by_id(r.req_category).cat_title,
            req_applicability: get_applicability_by_id(r.req_applicability).app_title,
            req_parent_id: r.req_parent,

            req_parent_title: if r.req_parent != 0 {
                get_requirement_by_id(r.req_parent).req_title
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

    let connection = &mut establish_connection();
    let result: User = users
        .filter(user_id.eq(id))
        .get_result(connection)
        .expect("Error reading table Users");

    result
}

pub fn get_status_by_id(id: i32) -> Status {
    use crate::schema::status::dsl::*;

    let connection = &mut establish_connection();
    let result: Status = status
        .filter(st_id.eq(id))
        .get_result(connection)
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

    let connection = &mut establish_connection();

    verification
        .order(verification_id)
        .load::<VerificationData>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying verification: {:?}", _err);
            "Error querying verification from the database".into()
        })
}

pub fn get_verification_by_project(_project_id: i32) -> Result<Vec<VerificationData>, String> {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();

    verification
        .filter(project_id.eq(_project_id))
        .order(verification_id)
        .load::<VerificationData>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying verification: {:?}", _err);
            "Error querying verification from the database".into()
        })
}

/// Get verification by ID with correct database mapping
pub fn get_verification_by_id(id: i32) -> VerificationData {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();
    let result: VerificationData = verification
        .filter(verification_id.eq(id))
        .get_result(connection)
        .unwrap();

    result
}

pub fn get_status_name_by_id(id: i32) -> String {
    get_status_by_id(id).st_title
}

pub fn get_requirement_by_id(id: i32) -> Requirement {
    use crate::schema::requirements::dsl::*;

    let connection = &mut establish_connection();
    let result: Requirement = requirements
        .filter(req_id.eq(id))
        .get_result(connection)
        .unwrap();

    result
}

pub fn get_requirement_title_by_id(id: i32) -> String {
    get_requirement_by_id(id).req_title
}

/// Return all requirements
pub fn get_requirements_all() -> Result<Vec<Requirement>, String> {
    use crate::schema::requirements::dsl::*;

    let connection = &mut establish_connection();

    requirements
        .order(req_id)
        .load::<Requirement>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

pub fn get_tests_all() -> Result<Vec<Test>, String> {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    tests
        .order(test_id)
        .load::<Test>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}

pub fn get_users_all() -> Result<Vec<User>, String> {
    use crate::schema::users::dsl::*;

    let connection = &mut establish_connection();

    users
        .order(user_id)
        .load::<User>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying page views from the database".into()
        })
}



pub fn get_test_by_id(id: i32) -> Test {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();
    let result: Test = tests.filter(test_id.eq(id)).get_result(connection).unwrap();

    result
}

pub fn get_test_status_by_id(id: i32) -> String {
    use crate::schema::status::dsl::*;
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    let ts: Test = tests.filter(test_id.eq(id)).get_result(connection).unwrap();

    let result: Status = status
        .filter(st_id.eq(ts.test_status))
        .get_result(connection)
        .unwrap();

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

    let connection = &mut establish_connection();
    
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
        .load::<Requirement>(connection)
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

pub fn generate_pdf_from_html(html_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::fs;
    use std::io::Write;
    
    // Check if wkhtmltopdf is available
    let wkhtmltopdf_check = Command::new("which").arg("wkhtmltopdf").output();
    
    if wkhtmltopdf_check.is_err() || !wkhtmltopdf_check.unwrap().status.success() {
        // wkhtmltopdf not available, return error
        return Err("wkhtmltopdf is not installed. Please install it to generate PDFs.".into());
    }
    
    // Create a temporary HTML file
    let temp_html_path = "/tmp/report_temp.html";
    let temp_pdf_path = "/tmp/report_temp.pdf";
    
    // Write HTML content to temporary file
    let mut html_file = fs::File::create(temp_html_path)?;
    html_file.write_all(html_content.as_bytes())?;
    
    // Use wkhtmltopdf command line tool
    let output = Command::new("wkhtmltopdf")
        .arg("--page-size")
        .arg("A4")
        .arg("--orientation")
        .arg("Portrait")
        .arg("--margin-top")
        .arg("20")
        .arg("--margin-bottom")
        .arg("20")
        .arg("--margin-left")
        .arg("20")
        .arg("--margin-right")
        .arg("20")
        .arg("--encoding")
        .arg("UTF-8")
        .arg(temp_html_path)
        .arg(temp_pdf_path)
        .output()?;
    
    if !output.status.success() {
        return Err(format!("wkhtmltopdf failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    // Read the generated PDF
    let pdf_bytes = fs::read(temp_pdf_path)?;
    
    // Clean up temporary files
    let _ = fs::remove_file(temp_html_path);
    let _ = fs::remove_file(temp_pdf_path);
    
    Ok(pdf_bytes)
}

// Project management functions
pub fn get_projects_all() -> Result<Vec<Project>, String> {
    use crate::schema::projects::dsl::*;
    
    let connection = &mut establish_connection();
    
    projects
        .load::<Project>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying projects from the database".into()
        })
}

pub fn get_project_by_id(project_id_param: i32) -> Project {
    use crate::schema::projects::dsl::*;
    
    let connection = &mut establish_connection();
    
    projects
        .filter(project_id.eq(project_id_param))
        .first::<Project>(connection)
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
    
    let connection = &mut establish_connection();
    
    requirements
        .filter(crate::schema::requirements::project_id.eq(_project_id))
        .load::<Requirement>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying requirements from the database".into()
        })
}

pub fn get_tests_by_project(_project_id: i32) -> Result<Vec<Test>, String> {
    use crate::schema::tests::dsl::*;
    
    let connection = &mut establish_connection();
    
    tests
        .filter(crate::schema::tests::project_id.eq(_project_id))
        .load::<Test>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying tests from the database".into()
        })
}

pub fn get_categories_by_project(_project_id: i32) -> Result<Vec<Category>, String> {
    use crate::schema::categories::dsl::*;
    
    let connection = &mut establish_connection();
    
    categories
        .filter(crate::schema::categories::project_id.eq(_project_id))
        .load::<Category>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying categories from the database".into()
        })
}

pub fn get_applicability_by_project(_project_id: i32) -> Result<Vec<Applicability>, String> {
    use crate::schema::applicability::dsl::*;
    
    let connection = &mut establish_connection();
    
    applicability
        .filter(crate::schema::applicability::project_id.eq(_project_id))
        .load::<Applicability>(connection)
        .map_err(|_err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying.*: {:?}", _err);
            "Error querying applicability from the database".into()
        })
}

pub fn get_matrix_by_project(_project_id: i32) -> Result<Vec<Matrix>, String> {
    use crate::schema::matrix::dsl::*;
    
    let connection = &mut establish_connection();
    
    matrix
        .filter(crate::schema::matrix::project_id.eq(_project_id))
        .load::<Matrix>(connection)
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
    
    let connection = &mut establish_connection();
    
    // Get the category to find its tag
    let category = categories::table
        .filter(categories::cat_id.eq(category_id))
        .first::<Category>(connection)
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")))?;
    
    // Count existing requirements with the same category and project
    let existing_count = requirements::table
        .filter(requirements::req_category.eq(category_id))
        .filter(requirements::project_id.eq(project_id))
        .count()
        .get_result::<i64>(connection)
        .map_err(|_e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Database error")))?;
    
    // Generate reference: REQ-{CATEGORY_TAG}-{NEXT_NUMBER}
    let next_number = existing_count + 1;
    let reference = format!("REQ-{}-{}", category.cat_tag, next_number);
    
    Ok(reference)
}
