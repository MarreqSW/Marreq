use crate::models::*;
use diesel::dsl::now;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::error::Error;

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
        .map_err(|e| format!("Database error: {}", e))?;
    
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
        .map_err(|e| format!("User not found: {}", e))?;
    
    // Verify current password
    match verify_password(current_password, &user.user_password) {
        Ok(true) => {
            // Hash new password
            let hashed_password = hash_password(new_password)
                .map_err(|e| format!("Password hashing error: {}", e))?;
            
            // Update password in database
            diesel::update(users.filter(user_id.eq(user_id)))
                .set(user_password.eq(hashed_password))
                .execute(connection)
                .map_err(|e| format!("Database update error: {}", e))?;
            
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
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
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
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
}

pub fn get_applicability_all() -> Result<Vec<Applicability>, String> {
    use crate::schema::applicability::dsl::*;

    let connection = &mut establish_connection();

    applicability
        .order(app_id)
        .get_results(connection)
        .map_err(|err| -> String {
            println!("Error querying applicability: {:?}", err);
            "Error querying applicability from the database".into()
        })
}

pub fn get_applicability_by_id(id: i32) -> Applicability {
    use crate::schema::applicability::dsl::*;

    let connection = &mut establish_connection();

    applicability
        .filter(app_id.eq(id))
        .get_result(connection)
        .map_err(|err| -> String {
            println!("Error querying applicability: {:?}", err);
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
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
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
            req_verification: get_verification_by_id(r.req_verification).ver_title,
            req_description: r.req_description,
            req_current_status: get_status_by_id(r.req_current_status).st_title,
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

pub fn get_verification_by_id(id: i32) -> Verification {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();
    let result: Verification = verification
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
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
}

pub fn get_tests_all() -> Result<Vec<Test>, String> {
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    tests
        .order(test_id)
        .load::<Test>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
}

pub fn get_users_all() -> Result<Vec<User>, String> {
    use crate::schema::users::dsl::*;

    let connection = &mut establish_connection();

    users
        .order(user_id)
        .load::<User>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
}

/// Return all verification types
pub fn get_verification_all() -> Result<Vec<Verification>, String> {
    use crate::schema::verification::dsl::*;

    let connection = &mut establish_connection();

    verification
        .order(verification_id)
        .load::<Verification>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
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
            test_parent_id: r.test_parent,
            test_parent_title: if r.test_parent != 0 {
                get_test_by_id(r.test_parent).test_name
            } else {
                "".to_string()
            },
        };
        println!("Decorate: {:?}", a);
        result.push(a);
    }

    result
}

pub fn insert_new_matrix_item(
    conn: &mut PgConnection,
    new: &NewMatrix,
) -> Result<(), Box<dyn Error>> {
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
        ))
        .load::<Requirement>(connection)
        .map_err(|e| format!("Error getting requirements for test: {}", e))?;

    Ok(linked_requirements)
}

pub fn insert_new_user(conn: &mut PgConnection, new: &NewUser) -> Result<i32, Box<dyn Error>> {
    let a: User = diesel::insert_into(crate::schema::users::table)
        .values(new)
        .get_result(conn)?;

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

pub fn establish_connection() -> diesel::PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
