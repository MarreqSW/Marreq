use diesel::prelude::*;
use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::response::status::NotFound;
use rocket::response::{content, Redirect};
use rocket::serde::json::json;
use rocket::http::{Cookie, CookieJar};

use rocket_dyn_templates::Template;

use std::path;

use crate::generators::*;
use crate::helper_functions::*;
use crate::html::*;
use crate::models::*;

// --------------------------------
// Authentication Helper Functions
// --------------------------------

fn require_auth(cookies: &CookieJar<'_>) -> Result<User, Redirect> {
    match is_authenticated(cookies) {
        Some(user) => Ok(user),
        None => Err(Redirect::to(uri!(login_page)))
    }
}

// --------------------------------
// Authentication Routes
// --------------------------------

#[get("/login")]
pub fn login_page() -> Template {
    let ctx = json!({ "title": "Login" });
    Template::render("login", ctx)
}

#[post("/login", data = "<login_form>")]
pub fn login(login_form: Form<LoginForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Template> {
    match authenticate_user(&login_form.username, &login_form.password) {
        Ok(Some(user)) => {
            // Set session cookie
            cookies.add_private(Cookie::new("user_id", user.user_id.to_string()));
            cookies.add_private(Cookie::new("username", user.user_username.clone()));
            cookies.add_private(Cookie::new("user_name", user.user_name.clone()));
            
            Ok(Redirect::to(uri!(index)))
        }
        Ok(None) => {
            let ctx = json!({
                "title": "Login",
                "error": "Invalid username or password"
            });
            Err(Template::render("login", ctx))
        }
        Err(e) => {
            let ctx = json!({
                "title": "Login",
                "error": format!("Authentication error: {}", e)
            });
            Err(Template::render("login", ctx))
        }
    }
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Redirect {
    use rocket::http::Cookie;
    
    // Create cookies with empty values and immediate expiration
    let mut user_id_cookie = Cookie::new("user_id", "");
    user_id_cookie.set_max_age(time::Duration::seconds(0));
    user_id_cookie.set_path("/");
    
    let mut username_cookie = Cookie::new("username", "");
    username_cookie.set_max_age(time::Duration::seconds(0));
    username_cookie.set_path("/");
    
    let mut user_name_cookie = Cookie::new("user_name", "");
    user_name_cookie.set_max_age(time::Duration::seconds(0));
    user_name_cookie.set_path("/");
    
    // Add the expired cookies to force removal
    cookies.add_private(user_id_cookie);
    cookies.add_private(username_cookie);
    cookies.add_private(user_name_cookie);
    
    Redirect::to(uri!(login_page))
}

#[get("/change_password")]
pub fn change_password_page() -> Template {
    let ctx = json!({ "title": "Change Password" });
    Template::render("change_password", ctx)
}

#[post("/change_password", data = "<password_form>")]
pub fn change_password(password_form: Form<ChangePasswordForm>, cookies: &CookieJar<'_>) -> Result<Template, Template> {
    // Get user ID from cookie
    let user_id_cookie = cookies.get_private("user_id");
    let user_id = match user_id_cookie {
        Some(cookie) => match cookie.value().parse::<i32>() {
            Ok(id) => id,
            Err(_) => {
                let ctx = json!({
                    "title": "Change Password",
                    "error": "Invalid session"
                });
                return Err(Template::render("change_password", ctx));
            }
        },
        None => {
            let ctx = json!({
                "title": "Change Password",
                "error": "Not logged in"
            });
            return Err(Template::render("change_password", ctx));
        }
    };
    
    // Validate passwords
    if password_form.new_password != password_form.confirm_password {
        let ctx = json!({
            "title": "Change Password",
            "error": "New passwords do not match"
        });
        return Err(Template::render("change_password", ctx));
    }
    
    if password_form.new_password.len() < 8 {
        let ctx = json!({
            "title": "Change Password",
            "error": "New password must be at least 8 characters long"
        });
        return Err(Template::render("change_password", ctx));
    }
    
    // Change password
    match change_user_password(user_id, &password_form.current_password, &password_form.new_password) {
        Ok(_) => {
            let ctx = json!({
                "title": "Change Password",
                "success": "Password changed successfully"
            });
            Ok(Template::render("change_password", ctx))
        }
        Err(e) => {
            let ctx = json!({
                "title": "Change Password",
                "error": e
            });
            Err(Template::render("change_password", ctx))
        }
    }
}

// --------------------------------
// Html Routes (TBD)
// --------------------------------

#[get("/")]
pub fn index(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let ctx = json!({ 
        "title": "Main",
        "user": user
    });
    Ok(Template::render("index", ctx))
}

#[get("/requirements")]
pub fn show_requirements(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let requirements = get_requirements_all();

    let ctx = match requirements {
        Ok(req) => {
            let requirements_decorate = decorate_requirements(req);
            let requirements_json = json!(requirements_decorate);
            json!({
                "requirements": requirements_json,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "user": user
            })
        }
    };

    Ok(Template::render("requirements", ctx))
}

#[get("/requirements/<req_id>")]
pub fn show_requirement_id(req_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let req = get_requirement_by_id(req_id);
    let req_decorate = decorate_requirements(vec![req]);
    let ctx = json!({
        "requirements": req_decorate,
        "user": user
    });

    Ok(Template::render("requirement_by_id", ctx))
}

#[get("/users")]
pub fn show_users(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let users = get_users_all();

    let ctx = match users {
        Ok(users_list) => {
            json!({
                "users": users_list,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "users": [],
                "user": user
            })
        }
    };

    Ok(Template::render("users", ctx))
}

#[get("/users/<user_id>")]
pub fn show_user_id(user_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let current_user = require_auth(cookies)?;
    let user = get_user_by_id(user_id);
    let ctx = json!({
        "user": current_user,
        "user_name": user.user_name,
        "user_username": user.user_username,
        "user_email": user.user_email,
        "user_level": user.user_level,
        "user_id": user.user_id,
        "user_creation_date": user.user_creation_date,
        "user_last_login": user.user_last_login
    });

    Ok(Template::render("user_by_id", ctx))
}

#[get("/edit_user/<user_id>")]
pub fn edit_user(user_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let current_user = require_auth(cookies)?;
    let user = get_user_by_id(user_id);
    #[cfg(debug_assertions)]
    println!("USer: {:?}", user);
    let ctx = json!({
        "users": user,
        "user": current_user
    });
    #[cfg(debug_assertions)]
    println!("edit user: {:?}", ctx);
    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/edit_user/<user_id>", data = "<user_form>")]
pub fn post_edit_user(user_id: i32, user_form: Form<UpdateUser>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _current_user = require_auth(cookies)?;
    
    let connection = &mut establish_connection();
    
    // Create an UpdateUser with the user_id
    let mut user_data = user_form.into_inner();
    user_data.user_id = Some(user_id);
    
    // Update the user in the database
    match update_user_without_password(connection, &user_data) {
        Ok(_) => Ok(Redirect::to(uri!(show_user_id(user_id)))),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error updating user: {:?}", e);
            Ok(Redirect::to(uri!(edit_user(user_id))))
        }
    }
}

#[get("/edit_requirement/<req_id>")]
pub fn get_edit_requirement(req_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let req = get_requirement_by_id(req_id);
    let req_decorate = decorate_requirements(vec![req]);
    let req_decorate_json = json!(req_decorate[0]);

    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap_or_default();
    let categories_json = json!(categories);

    let parents = get_requirements_all().unwrap_or_default();
    let parents_json = json!(parents);

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    let verification_types = get_verification_all().unwrap_or_default();
    let verification_json = json!(verification_types);

    let applicability = get_applicability_all().unwrap_or_default();
    let applicability_json = json!(applicability);

    let ctx = json!({
        "requirements": req_decorate_json, 
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json, 
        "applicability": applicability_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Requirement: {:#}", ctx);
    Ok(Template::render("edit_requirement_by_id", ctx))
}

#[allow(unused_variables)]
#[post("/edit_requirement/<req_id>", data = "<new_req>")]
pub fn post_edit_requirement(req_id: i32, new_req: Form<NewRequirement>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let my_id = new_req.req_id.unwrap_or(0);

    let connection = &mut establish_connection();
    edit_requirement(connection, &new_req).unwrap();

    Ok(Redirect::to(uri!(show_requirement_id(my_id))))
}

#[get("/new_requirement")]
pub fn new_requirement(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap_or_default();
    let categories_json = json!(categories);

    let parents = get_requirements_all().unwrap_or_default();
    let parents_json = json!(parents);

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    let verification_types = get_verification_all().unwrap_or_default();
    let verification_json = json!(verification_types);

    let applicability = get_applicability_all().unwrap_or_default();
    let applicability_json = json!(applicability);

    let ctx = json!({
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json, 
        "applicability": applicability_json,
        "user": user
    });

    Ok(Template::render("new_requirement", ctx))
}

#[post("/new_requirement", data = "<new_req>")]
pub fn post_requirement(new_req: Form<NewRequirement>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    let my_id = insert_new_requirement(connection, &new_req).unwrap();

    Ok(Redirect::to(uri!(show_requirement_id(my_id))))
}

#[get("/tests")]
pub fn show_tests(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let tests = get_tests_all().unwrap_or_default();
    let tests_decorate = decorate_tests(tests);
    let tests = json!(tests_decorate);
    let ctx = json!({
        "tests": tests,
        "user": user
    });

    Ok(Template::render("tests", ctx))
}

#[get("/tests/<test_id_param>")]
pub fn show_test_id(test_id_param: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let test = get_test_by_id(test_id_param);
    let test_decorate = decorate_tests(vec![test]);
    
    // Get linked requirements for this test
    let linked_requirements = get_requirements_for_test(test_id_param).unwrap_or_default();
    let linked_requirements_json = json!(linked_requirements);
    
    let decorated_test = &test_decorate[0];
    let ctx = json!({
        "test_id": decorated_test.test_id,
        "test_name": decorated_test.test_name,
        "test_description": decorated_test.test_description,
        "test_source": decorated_test.test_source,
        "test_status": decorated_test.test_status,
        "test_parent_id": decorated_test.test_parent_id,
        "test_parent_title": decorated_test.test_parent_title,
        "linked_requirements": linked_requirements_json,
        "user": user
    });

    Ok(Template::render("test_by_id", ctx))
}

#[get("/new_test")]
pub fn new_test(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap_or_default();
    let categories_json = json!(categories);

    let parents = get_tests_all().unwrap_or_default();
    let parents_json = json!(parents);

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    let requirements = get_requirements_all().unwrap_or_default();
    let requirements_json = json!(requirements);

    let ctx = json!({
        "categories": categories_json, 
        "status": status_json, 
        "parents": parents_json, 
        "users": users_json, 
        "requirements": requirements_json,
        "user": user
    });

    Ok(Template::render("new_test", ctx))
}

#[get("/edit_test/<test_id>")]
pub fn get_edit_test(test_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let test = get_test_by_id(test_id);
    let test_decorate = decorate_tests(vec![test]);
    let test_decorate_json = json!(test_decorate[0]);

    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let categories = get_categories_all().unwrap_or_default();
    let categories_json = json!(categories);

    let parents = get_tests_all().unwrap_or_default();
    let parents_json = json!(parents);

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    let verification_types = get_verification_all().unwrap_or_default();
    let verification_json = json!(verification_types);

    // Get linked requirements for this test
    let linked_requirements = get_requirements_for_test(test_id).unwrap_or_default();
    let linked_requirements_json = json!(linked_requirements);

    // Create a simple array of linked requirement IDs for template checking
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.req_id).collect();
    let linked_req_ids_json = json!(linked_req_ids);

    // Get all requirements for the multi-select
    let all_requirements = get_requirements_all().unwrap_or_default();
    let all_requirements_json = json!(all_requirements);

    let ctx = json!({
        "tests": test_decorate_json, 
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json,
        "linked_requirements": linked_requirements_json,
        "linked_req_ids": linked_req_ids_json,
        "requirements": all_requirements_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);
    Ok(Template::render("edit_test_by_id", ctx))
}

#[allow(unused_variables)]
#[post("/edit_test/<test_id>", data = "<edit_test_form>")]
pub fn post_edit_test(test_id: i32, edit_test_form: Form<EditTestForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    // First, update the test details
    let new_test = NewTest {
        test_id: Some(edit_test_form.test_id),
        test_name: edit_test_form.test_name.clone(),
        test_description: edit_test_form.test_description.clone(),
        test_source: edit_test_form.test_source.clone(),
        test_status: edit_test_form.test_status,
        test_parent: edit_test_form.test_parent,
    };
    
    edit_test(connection, &new_test).unwrap();
    
    // Then, update the requirement links
    update_test_requirement_links(connection, edit_test_form.test_id, &edit_test_form.linked_requirements).unwrap();

    Ok(Redirect::to(uri!(show_test_id(edit_test_form.test_id))))
}

#[post("/new_test", data = "<new_test>")]
pub fn post_test(new_test: Form<NewTestForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    let my_new_test = NewTest {
        test_id: None,
        test_name: new_test.test_name.clone(),
        test_description: new_test.test_description.clone(),
        test_source: new_test.test_source.clone(),
        test_status: new_test.test_status,
        test_parent: new_test.test_parent,
    };
    let my_id = insert_new_test(connection, &my_new_test).unwrap();

    #[cfg(debug_assertions)]
    println!("NewTestForm requirements: {:#?}", new_test.test_req);
    for req in new_test.test_req.iter() {
        let matrix_item = NewMatrix {
            matrix_req_id: *req,
            matrix_test_id: my_id,
        };
        insert_new_matrix_item(connection, &matrix_item).unwrap();
    }

    Ok(Redirect::to(uri!(show_test_id(my_id))))
}

#[get("/status")]
pub fn show_status() -> content::RawHtml<String> {
    use crate::schema::status::dsl::*;

    let mut out_str = print_header();
    let connection = &mut establish_connection();

    let all_status = status
        .load::<Status>(connection)
        .map_err(|err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .unwrap();

    for st in all_status.iter() {
        out_str = format!(
            "{}
        <div class='AllStatus'>
            <div>Id: {}</div>
            <div>Title: {}</div>
            <div>Description: {}</div>
        </div>",
            out_str, st.st_id, st.st_title, st.st_description
        );
    }

    out_str = format!("{} {}", out_str, print_footer());
    content::RawHtml(out_str)
}

#[get("/matrix?<sort_by>&<sort_order>&<test_status_filter>")]
pub fn get_matrix(cookies: &CookieJar<'_>, sort_by: Option<String>, sort_order: Option<String>, test_status_filter: Option<i32>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    use crate::schema::matrix::dsl::*;
    use crate::schema::requirements::dsl::*;
    use crate::schema::tests::dsl::*;

    let connection = &mut establish_connection();

    let mut all_reqs = requirements
        .load::<Requirement>(connection)
        .map_err(|err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .expect("Error getting matrix table");

    let mut all_tests = tests
        .load::<Test>(connection)
        .map_err(|err| -> String {
            #[cfg(debug_assertions)]
            println!("Error querying tests: {:?}", err);
            "Error querying tests from the database".into()
        })
        .expect("Error getting tests");

    // Always sort tests by test_id (number)
    all_tests.sort_by(|a, b| a.test_id.cmp(&b.test_id));

    // Filter tests by status if filter is provided
    if let Some(status_filter) = test_status_filter {
        all_tests.retain(|test| test.test_status == status_filter);
    }

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "req_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());
    
    // Check if sorting by test column
    if sort_by.starts_with("test_") {
        // Extract test ID from sort_by (e.g., "test_1" -> test_id = 1)
        if let Ok(target_test_id) = sort_by.trim_start_matches("test_").parse::<i32>() {
            // Sort requirements based on their link status to the specified test
            if sort_order == "desc" {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection)
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection)
                        .unwrap();
                    b_has_link.cmp(&a_has_link)
                });
            } else {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection)
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection)
                        .unwrap();
                    a_has_link.cmp(&b_has_link)
                });
            }
        }
    } else {
        // Sort requirements by requirement fields
        match sort_by.as_str() {
            "req_id" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_id.cmp(&a.req_id));
                } else {
                    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
                }
            }
            "req_title" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_title.cmp(&a.req_title));
                } else {
                    all_reqs.sort_by(|a, b| a.req_title.cmp(&b.req_title));
                }
            }
            "req_reference" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_reference.cmp(&a.req_reference));
                } else {
                    all_reqs.sort_by(|a, b| a.req_reference.cmp(&b.req_reference));
                }
            }
            _ => {
                // Default sort by req_id ascending
                all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
            }
        }
    }

    let total_tests = all_tests.len() as i32;
    let total_requirements = all_reqs.len() as i32;

    // Create matrix data structure
    let mut total_links = 0;
    let mut requirements_with_matrix = Vec::new();

    for req in &all_reqs {
        let mut req_matrix = Vec::new();
        
        for test in &all_tests {
            let test_present: i64 = matrix
                .filter(matrix_req_id.eq(req.req_id))
                .filter(matrix_test_id.eq(test.test_id))
                .count()
                .get_result(connection)
                .unwrap();

            if test_present > 0 {
                req_matrix.push(json!({
                    "linked": true,
                    "test_status": test.test_status
                }));
                total_links += 1;
            } else {
                req_matrix.push(json!({
                    "linked": false,
                    "test_status": null
                }));
            }
        }
        
        requirements_with_matrix.push(json!({
            "req_id": req.req_id,
            "req_title": req.req_title,
            "req_reference": req.req_reference,
            "matrix": req_matrix
        }));
    }

    // Prepare tests with status names
    let mut tests_with_status = Vec::new();
    for test in all_tests {
        let test_status_name = get_status_name_by_id(test.test_status);
        tests_with_status.push(json!({
            "test_id": test.test_id,
            "test_name": test.test_name,
            "test_status": test_status_name
        }));
    }

    // Get all statuses for the filter dropdown
    let all_statuses = get_status_all().unwrap_or_default();
    let statuses_json = json!(all_statuses);

    let ctx = json!({
        "requirements": requirements_with_matrix,
        "tests": tests_with_status,
        "total_tests": total_tests,
        "total_requirements": total_requirements,
        "total_links": total_links,
        "current_sort_by": sort_by,
        "current_sort_order": sort_order,
        "test_status_filter": test_status_filter,
        "statuses": statuses_json,
        "user": user
    });

    Ok(Template::render("matrix", ctx))
}

#[get("/matrix.xls")]
pub async fn get_matrix_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    let _file = excel::create_matrix_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/matrix.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/requirements.xls")]
pub async fn get_requirements_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    let _file = excel::create_requirements_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/requirements.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/tests.xls")]
pub async fn get_tests_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    let _file = excel::create_tests_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/tests.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/new_user")]
pub fn new_user(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user
    });
    Ok(Template::render("new_user", ctx))
}

#[get("/categories")]
pub fn show_categories(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let categories = get_categories_all();

    let ctx = match categories {
        Ok(cats) => {
            json!({
                "categories": cats,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "categories": [],
                "user": user
            })
        }
    };

    Ok(Template::render("categories", ctx))
}

#[get("/new_category")]
pub fn new_category(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let ctx = json!({
        "user": user
    });
    Ok(Template::render("new_category", ctx))
}

#[post("/new_category", data = "<new_category>")]
pub fn post_category(new_category: Form<NewCategory>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let result = insert_new_category(connection, &new_category);
    match result {
        Ok(_) => Ok(Redirect::to(uri!(show_categories))),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error creating category: {:?}", e);
            Ok(Redirect::to(uri!(new_category)))
        }
    }
}

#[get("/edit_category/<cat_id>")]
pub fn get_edit_category(cat_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let category = get_category_by_id(cat_id);
    let ctx = json!({
        "categories": category,
        "user": user
    });
    Ok(Template::render("edit_category", ctx))
}

#[post("/edit_category/<cat_id>", data = "<category>")]
pub fn post_edit_category(cat_id: i32, category: Form<NewCategory>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(cat_id);
    
    let result = edit_category(connection, &category_with_id);
    match result {
        Ok(_) => Ok(Redirect::to(uri!(show_categories))),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error updating category: {:?}", e);
            Ok(Redirect::to(uri!(get_edit_category(cat_id))))
        }
    }
}

#[delete("/delete_category/<cat_id>")]
pub fn delete_category_route(cat_id: i32, cookies: &CookieJar<'_>) -> Result<rocket::http::Status, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let result = delete_category(connection, &cat_id);
    match result {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting category: {:?}", e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

#[post("/new_user", data = "<new_user>")]
pub fn post_user(new_user: Form<NewUser>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    // Hash the password before inserting
    let mut user_with_hashed_password = new_user.into_inner();
    match hash_password(&user_with_hashed_password.user_password) {
        Ok(hashed_password) => {
            user_with_hashed_password.user_password = hashed_password;
            let my_id = insert_new_user(connection, &user_with_hashed_password).unwrap();
            Ok(Redirect::to(uri!(show_user_id(my_id))))
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error hashing password: {:?}", e);
            Ok(Redirect::to(uri!(new_user)))
        }
    }
}

#[get("/applicability")]
pub fn show_applicability(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let applicability = get_applicability_all();

    let ctx = match applicability {
        Ok(apps) => {
            json!({
                "applicability": apps,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "applicability": [],
                "user": user
            })
        }
    };

    Ok(Template::render("applicability", ctx))
}

#[get("/new_applicability")]
pub fn new_applicability(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let ctx = json!({
        "user": user
    });
    Ok(Template::render("new_applicability", ctx))
}

#[post("/new_applicability", data = "<new_applicability>")]
pub fn post_applicability(new_applicability: Form<NewApplicability>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let result = insert_new_applicability(connection, &new_applicability);
    match result {
        Ok(_) => Ok(Redirect::to(uri!(show_applicability))),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error creating applicability: {:?}", e);
            Ok(Redirect::to(uri!(new_applicability)))
        }
    }
}

#[get("/edit_applicability/<app_id>")]
pub fn get_edit_applicability(app_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let applicability = get_applicability_by_id(app_id);
    let ctx = json!({
        "applicability": applicability,
        "user": user
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/edit_applicability/<app_id>", data = "<applicability>")]
pub fn post_edit_applicability(app_id: i32, applicability: Form<NewApplicability>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let mut applicability_with_id = applicability.into_inner();
    applicability_with_id.app_id = Some(app_id);
    
    let result = edit_applicability(connection, &applicability_with_id);
    match result {
        Ok(_) => Ok(Redirect::to(uri!(show_applicability))),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error updating applicability: {:?}", e);
            Ok(Redirect::to(uri!(get_edit_applicability(app_id))))
        }
    }
}

#[delete("/delete_applicability/<app_id>")]
pub fn delete_applicability_route(app_id: i32, cookies: &CookieJar<'_>) -> Result<rocket::http::Status, Redirect> {
    let _user = require_auth(cookies)?;
    let connection = &mut establish_connection();
    
    let result = delete_applicability(connection, &app_id);
    match result {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting applicability: {:?}", e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

#[get("/requirements/tree")]
pub fn show_requirements_tree(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get all requirements
    let all_requirements = get_requirements_all().unwrap_or_default();
    
    // Build tree structure
    let mut tree_data = Vec::new();
    let mut children_map: std::collections::HashMap<i32, Vec<&Requirement>> = std::collections::HashMap::new();
    
    // Group requirements by parent
    for req in &all_requirements {
        if req.req_parent == 0 {
            // Root requirements
            tree_data.push(req);
        } else {
            // Child requirements
            children_map.entry(req.req_parent).or_insert_with(Vec::new).push(req);
        }
    }
    
    // Sort requirements by ID
    tree_data.sort_by(|a, b| a.req_id.cmp(&b.req_id));
    
    // Create tree structure with children
    let mut tree_structure = Vec::new();
    for root_req in tree_data {
        let mut node = json!({
            "requirement": root_req,
            "children": Vec::<serde_json::Value>::new()
        });
        
        // Add children if any
        if let Some(children) = children_map.get(&root_req.req_id) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| a.req_id.cmp(&b.req_id));
            
            let children_json: Vec<serde_json::Value> = sorted_children
                .iter()
                .map(|child| json!({
                    "requirement": child,
                    "children": Vec::<serde_json::Value>::new()
                }))
                .collect();
            
            node["children"] = json!(children_json);
        }
        
        tree_structure.push(node);
    }
    
    let ctx = json!({
        "tree_data": tree_structure,
        "total_requirements": all_requirements.len(),
        "user": user
    });
    
    Ok(Template::render("requirements_tree", ctx))
}

#[get("/reports")]
pub fn show_reports(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get all data for metrics
    let all_requirements = get_requirements_all().unwrap_or_default();
    let all_tests = get_tests_all().unwrap_or_default();
    let all_categories = get_categories_all().unwrap_or_default();
    let all_users = get_users_all().unwrap_or_default();
    let all_statuses = get_status_all().unwrap_or_default();
    
    // Calculate metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();
    
    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id(req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id(test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id(req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }
    
    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test(req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }
    
    let coverage_percentage = if total_requirements > 0 {
        (covered_requirements as f64 / total_requirements as f64) * 100.0
    } else {
        0.0
    };
    
    let avg_tests_per_requirement = if total_requirements > 0 {
        total_links as f64 / total_requirements as f64
    } else {
        0.0
    };
    
    // Recent activity (last 30 days)
    let now = chrono::Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);
    
    let mut recent_requirements = 0;
    let mut recent_tests = 0;
    
    for req in &all_requirements {
        // For now, we'll use a placeholder since creation_date might not be available
        recent_requirements += 1; // Placeholder
    }
    
    for test in &all_tests {
        // Assuming test has creation date - you might need to add this field
        // For now, we'll use a placeholder
        recent_tests += 1; // Placeholder
    }
    
    let ctx = json!({
        "user": user,
        "metrics": {
            "total_requirements": total_requirements,
            "total_tests": total_tests,
            "total_categories": total_categories,
            "total_users": total_users,
            "coverage_percentage": coverage_percentage,
            "avg_tests_per_requirement": avg_tests_per_requirement,
            "covered_requirements": covered_requirements,
            "total_links": total_links,
            "recent_requirements": recent_requirements,
            "recent_tests": recent_tests
        },
        "requirements_by_status": requirements_by_status,
        "tests_by_status": tests_by_status,
        "requirements_by_category": requirements_by_category,
        "all_statuses": all_statuses,
        "all_categories": all_categories
    });
    
    Ok(Template::render("reports", ctx))
}

#[get("/reports/pdf")]
pub fn generate_pdf_report(cookies: &CookieJar<'_>) -> Result<rocket::response::content::RawHtml<String>, Redirect> {
    let _user = require_auth(cookies)?;
    
    // Get the same data as the reports page
    let all_requirements = get_requirements_all().unwrap_or_default();
    let all_tests = get_tests_all().unwrap_or_default();
    let all_categories = get_categories_all().unwrap_or_default();
    let all_users = get_users_all().unwrap_or_default();
    let all_statuses = get_status_all().unwrap_or_default();
    
    // Calculate the same metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();
    
    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id(req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id(test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id(req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }
    
    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test(req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }
    
    let coverage_percentage = if total_requirements > 0 {
        (covered_requirements as f64 / total_requirements as f64) * 100.0
    } else {
        0.0
    };
    
    let avg_tests_per_requirement = if total_requirements > 0 {
        total_links as f64 / total_requirements as f64
    } else {
        0.0
    };
    
    // Generate PDF content
    let pdf_content = generate_pdf_content(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status,
        tests_by_status,
        requirements_by_category
    );
    
    // For now, return the HTML content
    // In a real implementation, you would generate and return the actual PDF
    Ok(rocket::response::content::RawHtml(pdf_content))
}

